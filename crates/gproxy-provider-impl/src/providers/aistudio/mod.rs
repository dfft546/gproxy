use bytes::Bytes;

use gproxy_provider_core::{
    Credential, DispatchRule, DispatchTable, HttpMethod, Proto, ProviderConfig, ProviderError,
    ProviderResult, UpstreamCtx, UpstreamHttpRequest, UpstreamProvider,
    credential::ApiKeyCredential,
};

use crate::auth_extractor;

const PROVIDER_NAME: &str = "aistudio";
const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

// Mirrors `samples/crates/gproxy-provider-impl/src/provider/aistudio/mod.rs` dispatch semantics.
const DISPATCH_TABLE: DispatchTable = DispatchTable::new([
    // Claude
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    // Gemini
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    // OpenAI chat completions (AIStudio supports OpenAI-compat for chat)
    DispatchRule::Native,
    DispatchRule::Native,
    // OpenAI Responses (transform to Gemini)
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    // OpenAI basic ops (transform to Gemini)
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    // OAuth / usage (not implemented)
    DispatchRule::Unsupported,
    DispatchRule::Unsupported,
    DispatchRule::Unsupported,
]);

#[derive(Debug, Default)]
pub struct AIStudioProvider;

impl AIStudioProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl UpstreamProvider for AIStudioProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn dispatch_table(&self, _config: &ProviderConfig) -> DispatchTable {
        DISPATCH_TABLE
    }

    async fn build_gemini_generate(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::gemini::generate_content::request::GenerateContentRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        build_gemini_request(
            config,
            credential,
            &format!(
                "/v1beta/{}:generateContent",
                normalize_model_name(&req.path.model)
            ),
            &req.body,
            false,
        )
    }

    async fn build_gemini_generate_stream(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::gemini::stream_content::request::StreamGenerateContentRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let mut path = format!(
            "/v1beta/{}:streamGenerateContent",
            normalize_model_name(&req.path.model)
        );
        if let Some(query) = req
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            path.push('?');
            path.push_str(query);
        }
        build_gemini_request(config, credential, &path, &req.body, true)
    }

    async fn build_gemini_count_tokens(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::gemini::count_tokens::request::CountTokensRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        build_gemini_request(
            config,
            credential,
            &format!(
                "/v1beta/{}:countTokens",
                normalize_model_name(&req.path.model)
            ),
            &req.body,
            false,
        )
    }

    async fn build_gemini_models_list(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::gemini::list_models::request::ListModelsRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = aistudio_base_url(config)?;
        let api_key = aistudio_api_key(credential)?;
        let mut url = build_url(Some(base_url), DEFAULT_BASE_URL, "/v1beta/models");
        if let Some(q) = build_gemini_query(&req.query) {
            url = format!("{url}?{q}");
        }
        let mut headers = Vec::new();
        auth_extractor::set_header(&mut headers, "x-goog-api-key", api_key);
        auth_extractor::set_accept_json(&mut headers);
        Ok(UpstreamHttpRequest {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
            is_stream: false,
        })
    }

    async fn build_gemini_models_get(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::gemini::get_model::request::GetModelRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = aistudio_base_url(config)?;
        let api_key = aistudio_api_key(credential)?;
        let url = build_url(
            Some(base_url),
            DEFAULT_BASE_URL,
            &format!("/v1beta/{}", normalize_model_name(&req.path.name)),
        );
        let mut headers = Vec::new();
        auth_extractor::set_header(&mut headers, "x-goog-api-key", api_key);
        auth_extractor::set_accept_json(&mut headers);
        Ok(UpstreamHttpRequest {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
            is_stream: false,
        })
    }

    async fn build_openai_chat(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::openai::create_chat_completions::request::CreateChatCompletionRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = aistudio_base_url(config)?;
        let api_key = aistudio_api_key(credential)?;
        let url = build_url(
            Some(base_url),
            DEFAULT_BASE_URL,
            "/v1beta/openai/chat/completions",
        );
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
            is_stream: req.body.stream.unwrap_or(false),
        })
    }
}

fn aistudio_base_url(config: &ProviderConfig) -> ProviderResult<&str> {
    match config {
        ProviderConfig::AIStudio(cfg) => Ok(cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL)),
        _ => Err(ProviderError::InvalidConfig(
            "expected ProviderConfig::AIStudio".to_string(),
        )),
    }
}

fn aistudio_api_key(credential: &Credential) -> ProviderResult<&str> {
    match credential {
        Credential::AIStudio(ApiKeyCredential { api_key }) => Ok(api_key.as_str()),
        _ => Err(ProviderError::InvalidConfig(
            "expected Credential::AIStudio".to_string(),
        )),
    }
}

fn build_gemini_request<T: serde::Serialize>(
    config: &ProviderConfig,
    credential: &Credential,
    path: &str,
    body: &T,
    is_stream: bool,
) -> ProviderResult<UpstreamHttpRequest> {
    let base_url = aistudio_base_url(config)?;
    let api_key = aistudio_api_key(credential)?;
    let url = build_url(Some(base_url), DEFAULT_BASE_URL, path);
    let body = serde_json::to_vec(body).map_err(|err| ProviderError::Other(err.to_string()))?;
    let mut headers = Vec::new();
    auth_extractor::set_header(&mut headers, "x-goog-api-key", api_key);
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

fn normalize_model_name(model: &str) -> String {
    if model.starts_with("models/") {
        model.to_string()
    } else {
        format!("models/{model}")
    }
}

fn build_gemini_query(
    query: &gproxy_protocol::gemini::list_models::request::ListModelsQuery,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(size) = query.page_size {
        parts.push(format!("pageSize={size}"));
    }
    if let Some(token) = query.page_token.as_ref()
        && !token.is_empty()
    {
        parts.push(format!("pageToken={}", urlencoding::encode(token)));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("&"))
    }
}

fn build_url(base_url: Option<&str>, default_base: &str, path: &str) -> String {
    let base = base_url.unwrap_or(default_base).trim_end_matches('/');
    let mut path = path.trim_start_matches('/');
    if base.ends_with("/v1") && (path == "v1" || path.starts_with("v1/")) {
        path = path.trim_start_matches("v1/").trim_start_matches("v1");
    }
    if base.ends_with("/v1beta") && (path == "v1beta" || path.starts_with("v1beta/")) {
        path = path
            .trim_start_matches("v1beta/")
            .trim_start_matches("v1beta");
    }
    format!("{base}/{path}")
}
