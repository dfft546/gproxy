use std::sync::OnceLock;

use bytes::Bytes;
use serde_json::Value as JsonValue;

use gproxy_provider_core::{
    Credential, DispatchRule, DispatchTable, HttpMethod, ModelGetRequest, ModelListRequest, Proto,
    ProviderConfig, ProviderError, ProviderResult, Request, UpstreamBody, UpstreamCtx,
    UpstreamHttpRequest, UpstreamHttpResponse, UpstreamProvider, credential::ApiKeyCredential,
    header_set,
};

use crate::auth_extractor;

const PROVIDER_NAME: &str = "vertexexpress";
const DEFAULT_BASE_URL: &str = "https://aiplatform.googleapis.com";
const MODELS_JSON: &str = include_str!("models.json");

// Mirrors `samples/crates/gproxy-provider-impl/src/provider/vertexexpress/mod.rs` dispatch semantics.
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
    // OpenAI chat completions (Vertex Express does not provide OpenAI-compat; transform to Gemini)
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    // OpenAI Responses
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    DispatchRule::Transform {
        target: Proto::Gemini,
    },
    // OpenAI basic ops
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
pub struct VertexExpressProvider;

impl VertexExpressProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl UpstreamProvider for VertexExpressProvider {
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
        let model = vertexexpress_model(&req.path.model);
        let body = vertex_generate_payload(model, &req.body)?;
        build_gemini_request(
            config,
            credential,
            &format!("/v1beta1/publishers/google/models/{model}:generateContent"),
            &body,
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
        let model = vertexexpress_model(&req.path.model);
        let body = vertex_generate_payload(model, &req.body)?;
        let path = append_query(
            &format!("/v1beta1/publishers/google/models/{model}:streamGenerateContent"),
            req.query.as_deref(),
        );
        build_gemini_request(config, credential, &path, &body, true)
    }

    async fn build_gemini_count_tokens(
        &self,
        _ctx: &UpstreamCtx,
        config: &ProviderConfig,
        credential: &Credential,
        req: &gproxy_protocol::gemini::count_tokens::request::CountTokensRequest,
    ) -> ProviderResult<UpstreamHttpRequest> {
        let model = vertexexpress_model(&req.path.model);
        let body = vertex_count_tokens_payload(model, &req.body);
        build_gemini_request(
            config,
            credential,
            &format!("/v1beta1/publishers/google/models/{model}:countTokens"),
            &body,
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
        let base_url = vertexexpress_base_url(config)?;
        let api_key = vertexexpress_api_key(credential)?;
        let mut url = build_url(
            Some(base_url),
            DEFAULT_BASE_URL,
            "/v1beta1/publishers/google/models",
        );
        let mut query = format!("key={}", urlencoding::encode(api_key));
        if let Some(extra) = build_gemini_query(&req.query) {
            query.push('&');
            query.push_str(&extra);
        }
        url = format!("{url}?{query}");
        let mut headers = Vec::new();
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
        let base_url = vertexexpress_base_url(config)?;
        let api_key = vertexexpress_api_key(credential)?;
        let name = vertexexpress_model(&req.path.name);
        let url = build_url(
            Some(base_url),
            DEFAULT_BASE_URL,
            &format!("/v1beta1/publishers/google/models/{name}"),
        );
        let url = format!("{url}?key={}", urlencoding::encode(api_key));
        let mut headers = Vec::new();
        auth_extractor::set_accept_json(&mut headers);
        Ok(UpstreamHttpRequest {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
            is_stream: false,
        })
    }

    fn local_response(
        &self,
        _ctx: &UpstreamCtx,
        _config: &ProviderConfig,
        credential: &Credential,
        req: &Request,
    ) -> ProviderResult<Option<UpstreamHttpResponse>> {
        match req {
            Request::ModelList(ModelListRequest::Gemini(_)) => {
                let _ = vertexexpress_api_key(credential)?;
                let list = load_models_value()?;
                let body = serde_json::to_vec(list)
                    .map_err(|err| ProviderError::Other(err.to_string()))?;
                Ok(Some(local_json_response(200, body)))
            }
            Request::ModelGet(ModelGetRequest::Gemini(req)) => {
                let _ = vertexexpress_api_key(credential)?;
                let list = load_models_value()?;
                let name = normalize_vertex_model_id(&req.path.name);
                let (status, body_json) = match find_model_value(list, &name) {
                    Some(model) => (200, model),
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

fn vertexexpress_base_url(config: &ProviderConfig) -> ProviderResult<&str> {
    match config {
        ProviderConfig::VertexExpress(cfg) => {
            Ok(cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL))
        }
        _ => Err(ProviderError::InvalidConfig(
            "expected ProviderConfig::VertexExpress".to_string(),
        )),
    }
}

fn vertexexpress_api_key(credential: &Credential) -> ProviderResult<&str> {
    match credential {
        Credential::VertexExpress(ApiKeyCredential { api_key }) => Ok(api_key.as_str()),
        _ => Err(ProviderError::InvalidConfig(
            "expected Credential::VertexExpress".to_string(),
        )),
    }
}

fn vertexexpress_model(model: &str) -> &str {
    model.strip_prefix("models/").unwrap_or(model)
}

fn build_gemini_request<T: serde::Serialize>(
    config: &ProviderConfig,
    credential: &Credential,
    path: &str,
    body: &T,
    is_stream: bool,
) -> ProviderResult<UpstreamHttpRequest> {
    let base_url = vertexexpress_base_url(config)?;
    let api_key = vertexexpress_api_key(credential)?;
    let url = build_url(Some(base_url), DEFAULT_BASE_URL, path);
    let sep = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{sep}key={}", urlencoding::encode(api_key));
    let body = serde_json::to_vec(body).map_err(|err| ProviderError::Other(err.to_string()))?;
    let mut headers = Vec::new();
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

fn vertex_generate_payload(
    path_model: &str,
    body: &gproxy_protocol::gemini::generate_content::request::GenerateContentRequestBody,
) -> ProviderResult<JsonValue> {
    let mut value =
        serde_json::to_value(body).map_err(|err| ProviderError::Other(err.to_string()))?;
    if let JsonValue::Object(map) = &mut value
        && let Some(model) = map.get("model").and_then(|m| m.as_str())
    {
        map.insert(
            "model".to_string(),
            JsonValue::String(normalize_vertex_model_ref(model, path_model)),
        );
    }
    Ok(value)
}

fn vertex_count_tokens_payload(
    path_model: &str,
    body: &gproxy_protocol::gemini::count_tokens::request::CountTokensRequestBody,
) -> JsonValue {
    let mut out = serde_json::Map::new();

    // Vertex countTokens accepts model in publisher format.
    out.insert(
        "model".to_string(),
        JsonValue::String(format!("publishers/google/models/{path_model}")),
    );

    if let Some(contents) = body.contents.as_ref()
        && let Ok(value) = serde_json::to_value(contents)
    {
        out.insert("contents".to_string(), value);
    }

    if let Some(generate) = body.generate_content_request.as_ref() {
        if !out.contains_key("contents")
            && let Some(v) = generate.get("contents")
        {
            out.insert("contents".to_string(), v.clone());
        }
        if let Some(v) = generate.get("instances") {
            out.insert("instances".to_string(), v.clone());
        }
        if let Some(v) = generate.get("tools") {
            out.insert("tools".to_string(), v.clone());
        }
        if let Some(v) = generate
            .get("systemInstruction")
            .or_else(|| generate.get("system_instruction"))
        {
            out.insert("systemInstruction".to_string(), v.clone());
        }
        if let Some(v) = generate
            .get("generationConfig")
            .or_else(|| generate.get("generation_config"))
        {
            out.insert("generationConfig".to_string(), v.clone());
        }
        if let Some(v) = generate.get("model").and_then(|m| m.as_str()) {
            out.insert(
                "model".to_string(),
                JsonValue::String(normalize_vertex_model_ref(v, path_model)),
            );
        }
    }

    JsonValue::Object(out)
}

fn normalize_vertex_model_ref(model: &str, fallback_model: &str) -> String {
    let m = model.trim().trim_start_matches('/');
    if m.is_empty() {
        return format!("publishers/google/models/{fallback_model}");
    }
    if m.starts_with("publishers/") && m.contains("/models/") {
        return m.to_string();
    }
    if let Some(id) = m.strip_prefix("models/") {
        return format!("publishers/google/models/{id}");
    }
    if let Some((publisher, id)) = m.split_once('/')
        && !publisher.is_empty()
        && !id.is_empty()
    {
        return format!("publishers/{publisher}/models/{id}");
    }
    format!("publishers/google/models/{m}")
}

fn append_query(path: &str, query: Option<&str>) -> String {
    let Some(query) = query.map(str::trim).filter(|q| !q.is_empty()) else {
        return path.to_string();
    };
    if path.contains('?') {
        format!("{path}&{query}")
    } else {
        format!("{path}?{query}")
    }
}

static MODELS_CACHE: OnceLock<JsonValue> = OnceLock::new();

fn load_models_value() -> ProviderResult<&'static JsonValue> {
    if let Some(value) = MODELS_CACHE.get() {
        return Ok(value);
    }
    let parsed: JsonValue =
        serde_json::from_str(MODELS_JSON).map_err(|err| ProviderError::Other(err.to_string()))?;
    if parsed.get("models").and_then(|v| v.as_array()).is_none() {
        return Err(ProviderError::Other(
            "vertexexpress_models.json missing models array".to_string(),
        ));
    }
    let _ = MODELS_CACHE.set(parsed);
    Ok(MODELS_CACHE.get().expect("models cache"))
}

fn find_model_value(list: &JsonValue, target: &str) -> Option<JsonValue> {
    let models = list.get("models")?.as_array()?;
    models
        .iter()
        .find(|item| {
            item.get("name")
                .and_then(|value| value.as_str())
                .map(|name| normalize_vertex_model_id(name) == target)
                .unwrap_or(false)
        })
        .cloned()
}

fn normalize_vertex_model_id(model: &str) -> String {
    let model = model.trim_start_matches('/');
    let model = model.strip_prefix("publishers/google/").unwrap_or(model);
    model.to_string()
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

fn build_url(base_url: Option<&str>, default_base: &str, path: &str) -> String {
    let base = base_url.unwrap_or(default_base).trim_end_matches('/');
    let mut path = path.trim_start_matches('/');
    if base.ends_with("/v1") && (path == "v1" || path.starts_with("v1/")) {
        path = path.trim_start_matches("v1/").trim_start_matches("v1");
    }
    if base.ends_with("/v1beta1") && (path == "v1beta1" || path.starts_with("v1beta1/")) {
        path = path
            .trim_start_matches("v1beta1/")
            .trim_start_matches("v1beta1");
    }
    format!("{base}/{path}")
}
