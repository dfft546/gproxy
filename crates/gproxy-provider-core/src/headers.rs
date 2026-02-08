pub type Headers = Vec<(String, String)>;

pub fn header_set(headers: &mut Headers, name: impl Into<String>, value: impl Into<String>) {
    let name = name.into();
    let value = value.into();
    let key = name.to_ascii_lowercase();
    if let Some((_, v)) = headers
        .iter_mut()
        .find(|(k, _)| k.to_ascii_lowercase() == key)
    {
        *v = value;
        return;
    }
    headers.push((name, value));
}

pub fn header_get<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
    let key = name.to_ascii_lowercase();
    headers
        .iter()
        .find(|(k, _)| k.to_ascii_lowercase() == key)
        .map(|(_, v)| v.as_str())
}

pub fn header_remove(headers: &mut Headers, name: &str) -> Option<String> {
    let key = name.to_ascii_lowercase();
    let idx = headers
        .iter()
        .position(|(k, _)| k.to_ascii_lowercase() == key)?;
    Some(headers.remove(idx).1)
}
