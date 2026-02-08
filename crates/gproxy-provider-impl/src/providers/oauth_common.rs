use std::future::Future;

pub(crate) fn parse_query_value(query: Option<&str>, key: &str) -> Option<String> {
    let raw = query?;
    if raw.is_empty() {
        return None;
    }
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut iter = pair.splitn(2, '=');
        let k = iter.next().unwrap_or_default();
        if k.is_empty() || k != key {
            continue;
        }
        let v = iter.next().unwrap_or_default();
        let decoded = urlencoding::decode(v).ok()?;
        let value = decoded.trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
    None
}

pub(crate) fn extract_code_state_from_callback_url(
    callback_url: &str,
) -> (Option<String>, Option<String>) {
    let raw = callback_url.trim();
    if raw.is_empty() {
        return (None, None);
    }
    let query = if let Some(idx) = raw.find('?') {
        &raw[idx + 1..]
    } else {
        raw
    };
    let query = query.split('#').next().unwrap_or(query);
    if query.is_empty() {
        return (None, None);
    }
    (
        parse_query_value(Some(query), "code"),
        parse_query_value(Some(query), "state"),
    )
}

pub(crate) fn resolve_manual_code_and_state(
    query: Option<&str>,
) -> Result<(String, Option<String>), &'static str> {
    let mut code = parse_query_value(query, "code");
    let mut state = parse_query_value(query, "state");
    if let Some(callback_url) = parse_query_value(query, "callback_url") {
        let (code_from_callback, state_from_callback) =
            extract_code_state_from_callback_url(&callback_url);
        if code.is_none() {
            code = code_from_callback;
        }
        if state.is_none() {
            state = state_from_callback;
        }
    }
    let Some(code) = code else {
        return Err("missing code");
    };
    Ok((code, state))
}

pub(crate) fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
            .block_on(future)
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_code_state_from_callback_url, resolve_manual_code_and_state};

    #[test]
    fn callback_url_extracts_code_and_state() {
        let (code, state) = extract_code_state_from_callback_url(
            "http://localhost:8787/cb?code=abc-123&state=st_1",
        );
        assert_eq!(code.as_deref(), Some("abc-123"));
        assert_eq!(state.as_deref(), Some("st_1"));
    }

    #[test]
    fn callback_url_query_string_only_is_supported() {
        let (code, state) = extract_code_state_from_callback_url("code=opaque%2Bvalue&state=s1");
        assert_eq!(code.as_deref(), Some("opaque+value"));
        assert_eq!(state.as_deref(), Some("s1"));
    }

    #[test]
    fn manual_code_is_preferred_over_callback_url_code() {
        let parsed = resolve_manual_code_and_state(Some(
            "code=direct-code&callback_url=http%3A%2F%2Flocalhost%2Fcb%3Fcode%3Dother%26state%3Ds2",
        ))
        .expect("manual parse should succeed");
        assert_eq!(parsed.0, "direct-code");
        assert_eq!(parsed.1.as_deref(), Some("s2"));
    }

    #[test]
    fn manual_callback_url_is_used_when_code_missing() {
        let parsed = resolve_manual_code_and_state(Some(
            "callback_url=http%3A%2F%2Flocalhost%2Fcb%3Fcode%3Dfrom-url%26state%3Dst",
        ))
        .expect("manual parse should succeed");
        assert_eq!(parsed.0, "from-url");
        assert_eq!(parsed.1.as_deref(), Some("st"));
    }

    #[test]
    fn manual_parse_requires_code() {
        let parsed = resolve_manual_code_and_state(Some("state=only-state"));
        assert_eq!(parsed, Err("missing code"));
    }
}
