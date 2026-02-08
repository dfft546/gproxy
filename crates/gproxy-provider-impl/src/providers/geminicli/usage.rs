use super::*;

pub(super) fn build_upstream_usage(
    _ctx: &UpstreamCtx,
    config: &ProviderConfig,
    credential: &Credential,
) -> ProviderResult<UpstreamHttpRequest> {
    let project_id = geminicli_project_id(credential)?;
    let body = serde_json::json!({
        "project": project_id,
    });
    build_gemini_request(
        config,
        credential,
        "/v1internal:retrieveUserQuota",
        &body,
        false,
    )
}
