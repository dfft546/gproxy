use std::sync::OnceLock;

use base64::Engine;
use bytes::Bytes;
use rand::RngCore;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tiktoken_rs::{CoreBPE, get_bpe_from_model, o200k_base};

use gproxy_provider_core::credential::CodexCredential;
use gproxy_provider_core::{
    AuthRetryAction, Credential, DispatchRule, DispatchTable, HttpMethod, ModelGetRequest,
    ModelListRequest, OAuthCallbackRequest, OAuthCallbackResult, OAuthCredential,
    OAuthStartRequest, Proto, ProviderConfig, ProviderError, ProviderResult, Request, UpstreamBody,
    UpstreamCtx, UpstreamHttpRequest, UpstreamHttpResponse, UpstreamProvider, header_set,
};

use gproxy_protocol::openai;
use gproxy_protocol::openai::create_response::types::{
    EasyInputMessage, EasyInputMessageContent, EasyInputMessageRole, EasyInputMessageType,
    InputItem, InputParam, Instructions, Metadata,
};

use crate::auth_extractor;
mod instructions;
mod oauth;
mod usage;

const PROVIDER_NAME: &str = "codex";
const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const DEFAULT_ISSUER: &str = "https://auth.openai.com";
const OAUTH_STATE_TTL_SECS: u64 = 600;
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Default)]
struct IdTokenClaims {
    email: Option<String>,
    plan: Option<String>,
    account_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct CodexProvider;

impl CodexProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl UpstreamProvider for CodexProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn dispatch_table(&self, _config: &ProviderConfig) -> DispatchTable {
        DispatchTable::new([
            // Claude
            DispatchRule::Unsupported,
            DispatchRule::Transform {
                target: Proto::OpenAIResponse,
            },
            DispatchRule::Transform {
                target: Proto::OpenAI,
            },
            DispatchRule::Transform {
                target: Proto::OpenAI,
            },
            DispatchRule::Transform {
                target: Proto::OpenAI,
            },
            // Gemini
            DispatchRule::Unsupported,
            DispatchRule::Transform {
                target: Proto::OpenAIResponse,
            },
            DispatchRule::Transform {
                target: Proto::OpenAI,
            },
            DispatchRule::Transform {
                target: Proto::OpenAI,
            },
            DispatchRule::Transform {
                target: Proto::OpenAI,
            },
            // OpenAI chat completions
            DispatchRule::Unsupported,
            DispatchRule::Transform {
                target: Proto::OpenAIResponse,
            },
            // OpenAI Responses
            DispatchRule::Unsupported,
            DispatchRule::Native,
            // OpenAI basic ops
            DispatchRule::Native,
            DispatchRule::Native,
            DispatchRule::Native,
            // OAuth start/callback + upstream usage are supported (see samples).
            DispatchRule::Native,
            DispatchRule::Native,
            DispatchRule::Native,
        ])
    }

    fn oauth_start(
        &self,
        ctx: &UpstreamCtx,
        config: &ProviderConfig,
        req: &OAuthStartRequest,
    ) -> ProviderResult<UpstreamHttpResponse> {
        oauth::oauth_start(ctx, config, req)
    }

    fn oauth_callback(
        &self,
        ctx: &UpstreamCtx,
        config: &ProviderConfig,
        req: &OAuthCallbackRequest,
    ) -> ProviderResult<OAuthCallbackResult> {
        oauth::oauth_callback(ctx, config, req)
    }

    fn upgrade_credential<'a>(
        &'a self,
        _ctx: &'a UpstreamCtx,
        _config: &'a ProviderConfig,
        credential: &'a Credential,
        _req: &'a Request,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = ProviderResult<Option<Credential>>> + Send + 'a>,
    > {
        Box::pin(async move { oauth::enrich_credential_profile_if_missing(credential).await })
    }

    fn on_auth_failure<'a>(
        &'a self,
        ctx: &'a UpstreamCtx,
        config: &'a ProviderConfig,
        credential: &'a Credential,
        req: &'a Request,
        failure: &'a gproxy_provider_core::provider::UpstreamFailure,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = ProviderResult<AuthRetryAction>> + Send + 'a>,
    > {
        oauth::on_auth_failure(ctx, config, credential, req, failure)
    }

    async fn build_upstream_usage(
        &self,
        ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
    ) -> ProviderResult<UpstreamHttpRequest> {
        usage::build_upstream_usage(ctx, config, credential)
    }

    async fn build_openai_responses(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &openai::create_response::request::CreateResponseRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = codex_base_url(config)?;
        let (access_token, account_id) = codex_credential(credential)?;
        let mut body = req.body.clone();
        apply_non_codex_instructions(&mut body, extract_user_agent(_ctx));
        normalize_codex_input(&mut body);
        // Codex upstream requires explicit non-persistent responses.
        body.store = Some(false);
        // Codex upstream does not support max tokens parameter.
        body.max_output_tokens = None;
        // Codex upstream rejects OpenAI stream_options.
        body.stream_options = None;
        let is_stream = body.stream.unwrap_or(false);
        let url = format!("{}/responses", base_url.trim_end_matches('/'));
        let body =
            serde_json::to_vec(&body).map_err(|err| ProviderError::Other(err.to_string()))?;

        let mut headers = Vec::new();
        auth_extractor::set_bearer(&mut headers, access_token);
        auth_extractor::set_accept_json(&mut headers);
        auth_extractor::set_content_type_json(&mut headers);
        auth_extractor::set_header(&mut headers, "chatgpt-account-id", account_id);

        Ok(UpstreamHttpRequest {
            method: HttpMethod::Post,
            url,
            headers,
            body: Some(Bytes::from(body)),
            is_stream,
        })
    }

    async fn build_openai_input_tokens(
        &self,
        _ctx: &UpstreamCtx,
        _config: &ProviderConfig,
        credential: &Credential,
        req: &openai::count_tokens::request::InputTokenCountRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let _ = codex_credential(credential)?;
        let tokens = count_input_tokens(&req.body)?;
        let response = openai::count_tokens::response::InputTokenCountResponse {
            object: openai::count_tokens::types::InputTokenObjectType::ResponseInputTokens,
            input_tokens: tokens,
        };
        let body =
            serde_json::to_vec(&response).map_err(|err| ProviderError::Other(err.to_string()))?;
        Ok(local_json_request(body))
    }

    fn local_response(
        &self,
        _ctx: &UpstreamCtx,
        _config: &ProviderConfig,
        credential: &Credential,
        req: &Request,
    ) -> ProviderResult<Option<UpstreamHttpResponse>> {
        match req {
            Request::ModelList(ModelListRequest::OpenAI(_)) => {
                let _ = codex_credential(credential)?;
                let list = load_models_value()?;
                let body = serde_json::to_vec(list)
                    .map_err(|err| ProviderError::Other(err.to_string()))?;
                Ok(Some(local_json_response(200, body)))
            }
            Request::ModelGet(ModelGetRequest::OpenAI(req)) => {
                let _ = codex_credential(credential)?;
                let list = load_models_value()?;
                let model = normalize_model_id(&req.path.model);
                let (status, body_json) = match find_model_value(list, &model) {
                    Some(found) => (200, found),
                    None => (
                        404,
                        serde_json::json!({ "error": { "message": "model not found" } }),
                    ),
                };
                let body = serde_json::to_vec(&body_json)
                    .map_err(|err| ProviderError::Other(err.to_string()))?;
                Ok(Some(local_json_response(status, body)))
            }
            _ => Ok(None),
        }
    }
}

fn codex_base_url(config: &ProviderConfig) -> ProviderResult<&str> {
    match config {
        ProviderConfig::Codex(cfg) => Ok(cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL)),
        _ => Err(ProviderError::InvalidConfig(
            "expected ProviderConfig::Codex".to_string(),
        )),
    }
}

fn codex_credential(credential: &Credential) -> ProviderResult<(&str, &str)> {
    match credential {
        Credential::Codex(cred) => Ok((cred.access_token.as_str(), cred.account_id.as_str())),
        _ => Err(ProviderError::InvalidConfig(
            "expected Credential::Codex".to_string(),
        )),
    }
}

fn local_json_request(body: Vec<u8>) -> UpstreamHttpRequest {
    let mut headers = Vec::new();
    auth_extractor::set_accept_json(&mut headers);
    auth_extractor::set_content_type_json(&mut headers);
    UpstreamHttpRequest {
        method: HttpMethod::Post,
        url: "local://codex".to_string(),
        headers,
        body: Some(Bytes::from(body)),
        is_stream: false,
    }
}

fn local_json_response(status: u16, body: Vec<u8>) -> UpstreamHttpResponse {
    let mut headers = Vec::new();
    header_set(&mut headers, "content-type", "application/json");
    UpstreamHttpResponse {
        status,
        headers,
        body: UpstreamBody::Bytes(Bytes::from(body)),
    }
}

fn count_input_tokens(
    body: &openai::count_tokens::request::InputTokenCountRequestBody,
) -> ProviderResult<i64> {
    let bpe = bpe_for_model(&body.model)?;
    let mut total = 0i64;
    if let Some(input) = &body.input {
        total += count_input_param(input, &bpe);
    }
    if let Some(instructions) = &body.instructions {
        total += count_text(instructions, &bpe);
    }
    Ok(total)
}

fn bpe_for_model(model: &str) -> ProviderResult<CoreBPE> {
    if let Ok(bpe) = get_bpe_from_model(model) {
        return Ok(bpe);
    }
    o200k_base().map_err(|err| ProviderError::Other(err.to_string()))
}

fn count_input_param(input: &openai::create_response::types::InputParam, bpe: &CoreBPE) -> i64 {
    match input {
        openai::create_response::types::InputParam::Text(text) => count_text(text, bpe),
        openai::create_response::types::InputParam::Items(items) => {
            items.iter().map(|item| count_input_item(item, bpe)).sum()
        }
    }
}

fn count_input_item(item: &openai::create_response::types::InputItem, bpe: &CoreBPE) -> i64 {
    use openai::create_response::types::InputItem;
    match item {
        InputItem::EasyMessage(message) => count_easy_message(&message.content, bpe),
        InputItem::Reference(_) => 0,
        InputItem::Item(item) => count_item(item, bpe),
    }
}

fn count_easy_message(
    content: &openai::create_response::types::EasyInputMessageContent,
    bpe: &CoreBPE,
) -> i64 {
    match content {
        openai::create_response::types::EasyInputMessageContent::Text(text) => {
            count_text(text, bpe)
        }
        openai::create_response::types::EasyInputMessageContent::Parts(parts) => parts
            .iter()
            .map(|part| count_input_content(part, bpe))
            .sum(),
    }
}

fn count_item(item: &openai::create_response::types::Item, bpe: &CoreBPE) -> i64 {
    use openai::create_response::types::Item;
    match item {
        Item::InputMessage(message) => count_input_message(message, bpe),
        Item::OutputMessage(message) => count_output_message(message, bpe),
        Item::FunctionOutput(output) => count_tool_call_output(&output.output, bpe),
        Item::CustomToolCallOutput(output) => count_tool_call_output(&output.output, bpe),
        _ => 0,
    }
}

fn count_input_message(
    message: &openai::create_response::types::InputMessage,
    bpe: &CoreBPE,
) -> i64 {
    message
        .content
        .iter()
        .map(|part| count_input_content(part, bpe))
        .sum()
}

fn count_output_message(
    message: &openai::create_response::types::OutputMessage,
    bpe: &CoreBPE,
) -> i64 {
    use openai::create_response::types::OutputMessageContent;
    message
        .content
        .iter()
        .map(|part| match part {
            OutputMessageContent::OutputText(text) => count_text(&text.text, bpe),
            OutputMessageContent::Refusal(refusal) => count_text(&refusal.refusal, bpe),
        })
        .sum()
}

fn count_tool_call_output(
    output: &openai::create_response::types::ToolCallOutput,
    bpe: &CoreBPE,
) -> i64 {
    match output {
        openai::create_response::types::ToolCallOutput::Text(text) => count_text(text, bpe),
        openai::create_response::types::ToolCallOutput::Content(items) => items
            .iter()
            .map(|item| match item {
                openai::create_response::types::FunctionAndCustomToolCallOutput::InputText(
                    content,
                ) => count_text(&content.text, bpe),
                openai::create_response::types::FunctionAndCustomToolCallOutput::InputImage(_) => 0,
                openai::create_response::types::FunctionAndCustomToolCallOutput::InputFile(_) => 0,
            })
            .sum(),
    }
}

fn count_input_content(
    content: &openai::create_response::types::InputContent,
    bpe: &CoreBPE,
) -> i64 {
    match content {
        openai::create_response::types::InputContent::InputText(text) => {
            count_text(&text.text, bpe)
        }
        openai::create_response::types::InputContent::InputImage(_) => 0,
        openai::create_response::types::InputContent::InputFile(_) => 0,
    }
}

fn count_text(text: &str, bpe: &CoreBPE) -> i64 {
    bpe.encode_ordinary(text).len() as i64
}

static MODELS_CACHE: OnceLock<JsonValue> = OnceLock::new();

fn load_models_value() -> ProviderResult<&'static JsonValue> {
    if let Some(value) = MODELS_CACHE.get() {
        return Ok(value);
    }
    let raw = include_str!("models.json");
    let parsed: JsonValue =
        serde_json::from_str(raw).map_err(|err| ProviderError::Other(err.to_string()))?;
    if parsed.get("data").and_then(|v| v.as_array()).is_none() {
        return Err(ProviderError::Other(
            "models.json missing data array".to_string(),
        ));
    }
    let _ = MODELS_CACHE.set(parsed);
    Ok(MODELS_CACHE.get().expect("models cache"))
}

fn find_model_value(list: &JsonValue, target: &str) -> Option<JsonValue> {
    let data = list.get("data")?.as_array()?;
    data.iter()
        .find(|item| {
            item.get("id")
                .and_then(|value| value.as_str())
                .map(|id| normalize_model_id(id) == target)
                .unwrap_or(false)
        })
        .cloned()
}

fn normalize_model_id(model: &str) -> String {
    let model = model.trim_start_matches('/');
    model.strip_prefix("models/").unwrap_or(model).to_string()
}

fn apply_non_codex_instructions(
    body: &mut openai::create_response::request::CreateResponseRequestBody,
    user_agent: Option<&str>,
) {
    if is_codex_user_agent(user_agent) {
        return;
    }
    let model = body.model.clone();
    let personality = resolve_codex_personality(body.metadata.as_ref());
    let extra = instructions::instructions_for_model(&model, personality);
    apply_instruction_text(body, &extra);
}

fn apply_instruction_text(
    body: &mut openai::create_response::request::CreateResponseRequestBody,
    extra: &str,
) {
    let extra = extra.trim();
    if extra.is_empty() {
        return;
    }
    let extra_text = extra.to_string();
    body.instructions = match body.instructions.take() {
        Some(Instructions::Text(existing)) => {
            if existing.trim().is_empty() {
                Some(Instructions::Text(extra_text))
            } else {
                Some(Instructions::Text(format!("{existing}\n\n{extra}")))
            }
        }
        Some(Instructions::Items(mut items)) => {
            items.push(instruction_text_item(extra_text));
            Some(Instructions::Items(items))
        }
        None => Some(Instructions::Text(extra_text)),
    };
}

fn instruction_text_item(text: String) -> InputItem {
    InputItem::EasyMessage(EasyInputMessage {
        r#type: EasyInputMessageType::Message,
        role: EasyInputMessageRole::System,
        content: EasyInputMessageContent::Text(text),
    })
}

fn normalize_codex_input(body: &mut openai::create_response::request::CreateResponseRequestBody) {
    let Some(input) = body.input.take() else {
        return;
    };

    body.input = Some(match input {
        InputParam::Text(text) => {
            InputParam::Items(vec![InputItem::EasyMessage(EasyInputMessage {
                r#type: EasyInputMessageType::Message,
                role: EasyInputMessageRole::User,
                content: EasyInputMessageContent::Text(text),
            })])
        }
        InputParam::Items(items) => InputParam::Items(items),
    });
}

fn resolve_codex_personality(
    metadata: Option<&Metadata>,
) -> Option<instructions::CodexPersonality> {
    metadata.and_then(|meta| {
        meta.get("codex_personality")
            .or_else(|| meta.get("personality"))
            .and_then(|value| instructions::parse_personality(value))
    })
}

fn is_codex_user_agent(user_agent: Option<&str>) -> bool {
    user_agent
        .map(|ua| ua.to_ascii_lowercase().contains("codex"))
        .unwrap_or(false)
}

fn extract_user_agent(ctx: &UpstreamCtx) -> Option<&str> {
    ctx.user_agent
        .as_deref()
        .filter(|value| !value.trim().is_empty())
}

fn json_response(body: serde_json::Value) -> UpstreamHttpResponse {
    let mut headers = Vec::new();
    header_set(&mut headers, "content-type", "application/json");
    let bytes = Bytes::from(serde_json::to_vec(&body).unwrap_or_default());
    UpstreamHttpResponse {
        status: 200,
        headers,
        body: UpstreamBody::Bytes(bytes),
    }
}

fn json_error(status: u16, message: &str) -> UpstreamHttpResponse {
    let mut headers = Vec::new();
    header_set(&mut headers, "content-type", "application/json");
    let bytes = Bytes::from(
        serde_json::to_vec(&serde_json::json!({ "error": message })).unwrap_or_default(),
    );
    UpstreamHttpResponse {
        status,
        headers,
        body: UpstreamBody::Bytes(bytes),
    }
}

fn generate_oauth_state() -> String {
    let mut state_bytes = [0u8; 32];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut state_bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(state_bytes)
}

fn parse_id_token_claims(id_token: &str) -> IdTokenClaims {
    let mut claims = IdTokenClaims::default();
    let mut parts = id_token.split('.');
    let (_h, payload_b64, _s) = match (parts.next(), parts.next(), parts.next()) {
        (Some(h), Some(p), Some(s)) if !h.is_empty() && !p.is_empty() && !s.is_empty() => (h, p, s),
        _ => return claims,
    };
    let payload_bytes = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload_b64) {
        Ok(bytes) => bytes,
        Err(_) => return claims,
    };
    let payload = match serde_json::from_slice::<JsonValue>(&payload_bytes) {
        Ok(value) => value,
        Err(_) => return claims,
    };

    let email = payload
        .get("email")
        .and_then(|value| value.as_str())
        .or_else(|| {
            payload
                .get("https://api.openai.com/profile")
                .and_then(|profile| profile.get("email"))
                .and_then(|value| value.as_str())
        })
        .map(|value| value.to_string());

    let (plan, account_id) = payload
        .get("https://api.openai.com/auth")
        .map(|auth| {
            let plan = auth
                .get("chatgpt_plan_type")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let account_id = auth
                .get("chatgpt_account_id")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            (plan, account_id)
        })
        .unwrap_or((None, None));

    claims.email = email;
    claims.plan = plan;
    claims.account_id = account_id;
    claims
}
