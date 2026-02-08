use bytes::Bytes;
use serde::Serialize;

use gproxy_provider_core::{
    Credential, DispatchRule, DispatchTable, HttpMethod, Proto, ProviderConfig, ProviderError,
    ProviderResult, UpstreamCtx, UpstreamHttpRequest, UpstreamProvider,
    credential::ApiKeyCredential,
};

use crate::auth_extractor;

const PROVIDER_NAME: &str = "deepseek";
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
const TOKENIZER_BYTES: &[u8] = include_bytes!("tokenizer.json");
const MODEL_CHAT: &str = "deepseek-chat";
const MODEL_REASONER: &str = "deepseek-reasoner";

// Mirrors `samples/crates/gproxy-provider-impl/src/provider/deepseek/mod.rs` dispatch semantics.
const DISPATCH_TABLE: DispatchTable = DispatchTable::new([
    // Claude
    DispatchRule::Native,
    DispatchRule::Native,
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
    DispatchRule::Transform {
        target: Proto::OpenAIChat,
    },
    DispatchRule::Transform {
        target: Proto::OpenAIChat,
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
    DispatchRule::Native,
    DispatchRule::Native,
    // OpenAI Responses (map to chat completions)
    DispatchRule::Transform {
        target: Proto::OpenAIChat,
    },
    DispatchRule::Transform {
        target: Proto::OpenAIChat,
    },
    // OpenAI basic ops
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    // OAuth / usage (not implemented)
    DispatchRule::Unsupported,
    DispatchRule::Unsupported,
    DispatchRule::Unsupported,
]);

#[derive(Debug, Default)]
pub struct DeepSeekProvider;

impl DeepSeekProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl UpstreamProvider for DeepSeekProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn dispatch_table(&self, _config: &ProviderConfig) -> DispatchTable {
        DISPATCH_TABLE
    }

    async fn build_claude_messages(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::claude::create_message::request::CreateMessageRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = deepseek_base_url(config)?;
        let api_key = deepseek_api_key(credential)?;
        let url = build_url(Some(base_url), DEFAULT_BASE_URL, "/anthropic/v1/messages");
        let is_stream = req.body.stream.unwrap_or(false);
        let body =
            serde_json::to_vec(&req.body).map_err(|err| ProviderError::Other(err.to_string()))?;
        let mut headers = Vec::new();
        auth_extractor::set_header(&mut headers, "x-api-key", api_key);
        auth_extractor::set_accept_json(&mut headers);
        auth_extractor::set_content_type_json(&mut headers);
        apply_anthropic_headers(&mut headers, &req.headers)?;
        Ok(UpstreamHttpRequest {
            method: HttpMethod::Post,
            url,
            headers,
            body: Some(Bytes::from(body)),
            is_stream,
        })
    }

    async fn build_openai_chat(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::openai::create_chat_completions::request::CreateChatCompletionRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = deepseek_base_url(config)?;
        let api_key = deepseek_api_key(credential)?;
        let url = build_url(Some(base_url), DEFAULT_BASE_URL, "/v1/chat/completions");
        let is_stream = req.body.stream.unwrap_or(false);
        let body =
            serde_json::to_vec(&req.body).map_err(|err| ProviderError::Other(err.to_string()))?;
        let mut headers = Vec::new();
        auth_extractor::set_bearer(&mut headers, api_key);
        auth_extractor::set_accept_json(&mut headers);
        auth_extractor::set_content_type_json(&mut headers);
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
        req: &gproxy_protocol::openai::count_tokens::request::InputTokenCountRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        // Validate credential (must be present) but compute locally.
        let _ = deepseek_api_key(credential)?;
        let tokens = count_input_tokens(&req.body)?;
        let response = gproxy_protocol::openai::count_tokens::response::InputTokenCountResponse {
            object: gproxy_protocol::openai::count_tokens::types::InputTokenObjectType::ResponseInputTokens,
            input_tokens: tokens,
        };
        let body =
            serde_json::to_vec(&response).map_err(|err| ProviderError::Other(err.to_string()))?;
        Ok(local_json_request(body))
    }

    async fn build_openai_models_list(
        &self,
        _ctx: &UpstreamCtx,
        _config: &ProviderConfig,
        credential: &Credential,
        _req: &gproxy_protocol::openai::list_models::request::ListModelsRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let _ = deepseek_api_key(credential)?;
        let response = gproxy_protocol::openai::list_models::response::ListModelsResponse {
            object: gproxy_protocol::openai::list_models::response::ListObjectType::List,
            data: deepseek_models(),
        };
        let body =
            serde_json::to_vec(&response).map_err(|err| ProviderError::Other(err.to_string()))?;
        Ok(local_json_request(body))
    }

    async fn build_openai_models_get(
        &self,
        _ctx: &UpstreamCtx,
        _config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::openai::get_model::request::GetModelRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let _ = deepseek_api_key(credential)?;
        let model = req.path.model.as_str();
        let Some(found) = deepseek_models().into_iter().find(|m| m.id == model) else {
            return Err(ProviderError::Other("model_not_found".to_string()));
        };
        let body =
            serde_json::to_vec(&found).map_err(|err| ProviderError::Other(err.to_string()))?;
        Ok(local_json_request(body))
    }
}

fn deepseek_base_url(config: &ProviderConfig) -> ProviderResult<&str> {
    match config {
        ProviderConfig::DeepSeek(cfg) => Ok(cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL)),
        _ => Err(ProviderError::InvalidConfig(
            "expected ProviderConfig::DeepSeek".to_string(),
        )),
    }
}

fn deepseek_api_key(credential: &Credential) -> ProviderResult<&str> {
    match credential {
        Credential::DeepSeek(ApiKeyCredential { api_key }) => Ok(api_key.as_str()),
        _ => Err(ProviderError::InvalidConfig(
            "expected Credential::DeepSeek".to_string(),
        )),
    }
}

fn local_json_request(body: Vec<u8>) -> UpstreamHttpRequest {
    let mut headers = Vec::new();
    auth_extractor::set_accept_json(&mut headers);
    auth_extractor::set_content_type_json(&mut headers);
    UpstreamHttpRequest {
        method: HttpMethod::Post,
        url: "local://deepseek".to_string(),
        headers,
        body: Some(Bytes::from(body)),
        is_stream: false,
    }
}

fn deepseek_models() -> Vec<gproxy_protocol::openai::get_model::types::Model> {
    use gproxy_protocol::openai::get_model::types::{Model, ModelObjectType};
    vec![
        Model {
            id: MODEL_CHAT.to_string(),
            created: None,
            object: ModelObjectType::Model,
            owned_by: "deepseek".to_string(),
        },
        Model {
            id: MODEL_REASONER.to_string(),
            created: None,
            object: ModelObjectType::Model,
            owned_by: "deepseek".to_string(),
        },
    ]
}

fn count_input_tokens(
    body: &gproxy_protocol::openai::count_tokens::request::InputTokenCountRequestBody,
) -> ProviderResult<i64> {
    use std::sync::{Mutex, OnceLock};
    use tokenizers::Tokenizer;

    let mut value =
        serde_json::to_value(body).map_err(|err| ProviderError::Other(err.to_string()))?;
    if let Some(map) = value.as_object_mut() {
        map.remove("model");
    }
    let text =
        serde_json::to_string(&value).map_err(|err| ProviderError::Other(err.to_string()))?;

    static TOKENIZER: OnceLock<Mutex<Option<Tokenizer>>> = OnceLock::new();
    let cache = TOKENIZER.get_or_init(|| Mutex::new(None));
    let tokenizer = {
        let mut guard = cache
            .lock()
            .map_err(|_| ProviderError::Other("tokenizer lock failed".to_string()))?;
        if guard.is_none() {
            let tokenizer = Tokenizer::from_bytes(TOKENIZER_BYTES)
                .map_err(|err| ProviderError::Other(err.to_string()))?;
            *guard = Some(tokenizer);
        }
        guard.as_ref().expect("tokenizer").clone()
    };
    let encoding = tokenizer
        .encode(text, false)
        .map_err(|err| ProviderError::Other(err.to_string()))?;
    Ok(encoding.get_ids().len() as i64)
}

fn build_url(base_url: Option<&str>, default_base: &str, path: &str) -> String {
    let base = base_url.unwrap_or(default_base).trim_end_matches('/');
    let mut path = path.trim_start_matches('/');
    if base.ends_with("/v1") && (path == "v1" || path.starts_with("v1/")) {
        path = path.trim_start_matches("v1/").trim_start_matches("v1");
    }
    format!("{base}/{path}")
}

fn apply_anthropic_headers(
    headers: &mut gproxy_provider_core::Headers,
    anthropic_headers: &impl Serialize,
) -> ProviderResult<()> {
    let value = serde_json::to_value(anthropic_headers)
        .map_err(|err| ProviderError::Other(err.to_string()))?;
    let map = value
        .as_object()
        .ok_or_else(|| ProviderError::Other("unexpected anthropic headers shape".to_string()))?;

    if let Some(version) = map
        .get("anthropic-version")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
    {
        auth_extractor::set_header(headers, "anthropic-version", version);
    }
    if let Some(beta) = map.get("anthropic-beta") {
        let s = match beta {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    if let Some(s) = item.as_str() {
                        out.push(s.to_string());
                    }
                }
                if out.is_empty() {
                    None
                } else {
                    Some(out.join(","))
                }
            }
            _ => None,
        };
        if let Some(s) = s {
            auth_extractor::set_header(headers, "anthropic-beta", &s);
        }
    }
    Ok(())
}
