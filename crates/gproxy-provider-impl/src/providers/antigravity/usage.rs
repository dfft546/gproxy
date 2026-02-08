use super::*;

pub(super) fn build_upstream_usage(
    _ctx: &UpstreamCtx,
    config: &ProviderConfig,
    credential: &Credential,
) -> ProviderResult<UpstreamHttpRequest> {
    let base_url = match config {
        ProviderConfig::Antigravity(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
        _ => {
            return Err(ProviderError::InvalidConfig(
                "expected ProviderConfig::Antigravity".to_string(),
            ));
        }
    };
    let base_url = base_url.trim_end_matches('/');
    let url = format!("{base_url}/v1internal:fetchAvailableModels");

    let access_token = match credential {
        Credential::Antigravity(cred) => cred.access_token.as_str(),
        _ => {
            return Err(ProviderError::InvalidConfig(
                "expected Credential::Antigravity".to_string(),
            ));
        }
    };

    let mut headers = Vec::new();
    auth_extractor::set_bearer(&mut headers, access_token);
    auth_extractor::set_accept_json(&mut headers);
    auth_extractor::set_content_type_json(&mut headers);
    auth_extractor::set_user_agent(&mut headers, ANTIGRAVITY_USER_AGENT);
    auth_extractor::set_header(&mut headers, "Accept-Encoding", "gzip");
    auth_extractor::set_header(&mut headers, "requestid", &make_request_id());

    Ok(UpstreamHttpRequest {
        method: HttpMethod::Post,
        url,
        headers,
        body: Some(Bytes::from_static(b"{}")),
        is_stream: false,
    })
}
