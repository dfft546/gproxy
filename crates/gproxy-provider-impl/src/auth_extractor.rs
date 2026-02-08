use gproxy_provider_core::{Headers, header_set};

pub fn set_bearer(headers: &mut Headers, access_token: &str) {
    header_set(headers, "Authorization", format!("Bearer {access_token}"));
}

pub fn set_accept_json(headers: &mut Headers) {
    header_set(headers, "Accept", "application/json");
}

pub fn set_content_type_json(headers: &mut Headers) {
    header_set(headers, "Content-Type", "application/json");
}

pub fn set_user_agent(headers: &mut Headers, ua: &str) {
    header_set(headers, "User-Agent", ua);
}

pub fn set_header(headers: &mut Headers, name: &str, value: &str) {
    header_set(headers, name, value);
}
