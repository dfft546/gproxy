use super::*;

pub(super) fn build_upstream_usage(
    _ctx: &UpstreamCtx,
    config: &ProviderConfig,
    credential: &Credential,
) -> ProviderResult<UpstreamHttpRequest> {
    let base_url = match config {
        ProviderConfig::Codex(cfg) => cfg.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL),
        _ => {
            return Err(ProviderError::InvalidConfig(
                "expected ProviderConfig::Codex".to_string(),
            ));
        }
    };
    let base_url = base_url.trim_end_matches('/');
    let base_url = base_url.strip_suffix("/codex").unwrap_or(base_url);
    let url = format!("{base_url}/wham/usage");

    let (access_token, account_id) = match credential {
        Credential::Codex(cred) => (cred.access_token.as_str(), cred.account_id.as_str()),
        _ => {
            return Err(ProviderError::InvalidConfig(
                "expected Credential::Codex".to_string(),
            ));
        }
    };

    let mut headers = Vec::new();
    auth_extractor::set_bearer(&mut headers, access_token);
    auth_extractor::set_accept_json(&mut headers);
    auth_extractor::set_content_type_json(&mut headers);
    auth_extractor::set_header(&mut headers, "chatgpt-account-id", account_id);

    Ok(UpstreamHttpRequest {
        method: HttpMethod::Get,
        url,
        headers,
        body: None,
        is_stream: false,
    })
}
