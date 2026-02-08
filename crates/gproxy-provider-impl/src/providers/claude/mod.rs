use bytes::Bytes;
use serde::Serialize;

use gproxy_provider_core::{
    Credential, DispatchRule, DispatchTable, HttpMethod, Proto, ProviderConfig, ProviderError,
    ProviderResult, UpstreamCtx, UpstreamHttpRequest, UpstreamProvider,
    credential::ApiKeyCredential,
};

use crate::auth_extractor;

const PROVIDER_NAME: &str = "claude";
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

const DISPATCH_TABLE: DispatchTable = DispatchTable::new([
    // Claude
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    DispatchRule::Native,
    // Gemini
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    // OpenAI chat completions (Anthropic OpenAI-compat is supported)
    DispatchRule::Native,
    DispatchRule::Native,
    // OpenAI Responses
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    // OpenAI basic ops
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    DispatchRule::Transform {
        target: Proto::Claude,
    },
    // OAuth / usage (not implemented for this provider)
    DispatchRule::Unsupported,
    DispatchRule::Unsupported,
    DispatchRule::Unsupported,
]);

#[derive(Debug, Default)]
pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl UpstreamProvider for ClaudeProvider {
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
        let base_url = match config {
            ProviderConfig::Claude(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected ProviderConfig::Claude".to_string(),
                ));
            }
        };
        let base_url = base_url.trim_end_matches('/');

        let api_key = match credential {
            Credential::Claude(ApiKeyCredential { api_key }) => api_key.as_str(),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected Credential::Claude".to_string(),
                ));
            }
        };

        let url = build_url(Some(base_url), DEFAULT_BASE_URL, "/v1/messages");
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

    async fn build_claude_count_tokens(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::claude::count_tokens::request::CountTokensRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = match config {
            ProviderConfig::Claude(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected ProviderConfig::Claude".to_string(),
                ));
            }
        };
        let base_url = base_url.trim_end_matches('/');

        let api_key = match credential {
            Credential::Claude(ApiKeyCredential { api_key }) => api_key.as_str(),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected Credential::Claude".to_string(),
                ));
            }
        };

        let url = build_url(
            Some(base_url),
            DEFAULT_BASE_URL,
            "/v1/messages/count_tokens",
        );
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
            is_stream: false,
        })
    }

    async fn build_claude_models_list(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::claude::list_models::request::ListModelsRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = match config {
            ProviderConfig::Claude(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected ProviderConfig::Claude".to_string(),
                ));
            }
        };
        let base_url = base_url.trim_end_matches('/');

        let api_key = match credential {
            Credential::Claude(ApiKeyCredential { api_key }) => api_key.as_str(),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected Credential::Claude".to_string(),
                ));
            }
        };

        let mut url = build_url(Some(base_url), DEFAULT_BASE_URL, "/v1/models");
        let query = build_claude_models_list_query(&req.query);
        if !query.is_empty() {
            url.push('?');
            url.push_str(&query);
        }
        let mut headers = Vec::new();
        auth_extractor::set_header(&mut headers, "x-api-key", api_key);
        auth_extractor::set_accept_json(&mut headers);
        apply_anthropic_headers(&mut headers, &req.headers)?;
        Ok(UpstreamHttpRequest {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
            is_stream: false,
        })
    }

    async fn build_claude_models_get(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::claude::get_model::request::GetModelRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = match config {
            ProviderConfig::Claude(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected ProviderConfig::Claude".to_string(),
                ));
            }
        };
        let base_url = base_url.trim_end_matches('/');

        let api_key = match credential {
            Credential::Claude(ApiKeyCredential { api_key }) => api_key.as_str(),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected Credential::Claude".to_string(),
                ));
            }
        };

        let url = build_url(
            Some(base_url),
            DEFAULT_BASE_URL,
            &format!("/v1/models/{}", req.path.model_id),
        );
        let mut headers = Vec::new();
        auth_extractor::set_header(&mut headers, "x-api-key", api_key);
        auth_extractor::set_accept_json(&mut headers);
        apply_anthropic_headers(&mut headers, &req.headers)?;
        Ok(UpstreamHttpRequest {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
            is_stream: false,
        })
    }

    // Anthropic OpenAI-compatible passthrough.
    async fn build_openai_chat(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::openai::create_chat_completions::request::CreateChatCompletionRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let base_url = match config {
            ProviderConfig::Claude(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected ProviderConfig::Claude".to_string(),
                ));
            }
        };
        let base_url = base_url.trim_end_matches('/');

        let api_key = match credential {
            Credential::Claude(ApiKeyCredential { api_key }) => api_key.as_str(),
            _ => {
                return Err(ProviderError::InvalidConfig(
                    "expected Credential::Claude".to_string(),
                ));
            }
        };

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

    // NOTE: We intentionally do not support arbitrary passthrough requests here.
    // Upstream calls are modeled as typed ops (protocol requests) plus a few
    // internal abilities like oauth/usage, handled elsewhere.
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
    // We rely on `gproxy-protocol`'s serde renames for Anthropic header values and
    // translate them to plain HTTP header strings.
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

fn build_claude_models_list_query(
    query: &gproxy_protocol::claude::list_models::request::ListModelsQuery,
) -> String {
    let mut parts = Vec::new();
    if let Some(after_id) = &query.after_id {
        parts.push(format!("after_id={after_id}"));
    }
    if let Some(before_id) = &query.before_id {
        parts.push(format!("before_id={before_id}"));
    }
    if let Some(limit) = query.limit {
        parts.push(format!("limit={limit}"));
    }
    parts.join("&")
}
