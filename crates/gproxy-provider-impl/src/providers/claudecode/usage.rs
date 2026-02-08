use super::*;

pub(super) fn build_upstream_usage(
    _ctx: &UpstreamCtx,
    config: &ProviderConfig,
    credential: &Credential,
) -> ProviderResult<UpstreamHttpRequest> {
    let platform_base = claudecode_platform_base_url(config)?;
    let platform_base = platform_base.trim_end_matches('/');
    let url = format!("{platform_base}/api/oauth/usage");

    let access_token = claudecode_access_token(config, credential)?;

    let mut headers = Vec::new();
    auth_extractor::set_bearer(&mut headers, &access_token);
    auth_extractor::set_accept_json(&mut headers);
    auth_extractor::set_content_type_json(&mut headers);
    auth_extractor::set_user_agent(&mut headers, CLAUDE_CODE_UA);
    auth_extractor::set_header(&mut headers, HEADER_BETA, OAUTH_BETA);

    Ok(UpstreamHttpRequest {
        method: HttpMethod::Get,
        url,
        headers,
        body: None,
        is_stream: false,
    })
}
