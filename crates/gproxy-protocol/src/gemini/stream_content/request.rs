use crate::gemini::generate_content::request::{GenerateContentPath, GenerateContentRequestBody};

#[derive(Debug, Clone)]
pub struct StreamGenerateContentRequest {
    pub path: GenerateContentPath,
    pub body: GenerateContentRequestBody,
    /// Raw downstream query string for stream shape hints (e.g. `alt=sse`).
    pub query: Option<String>,
}
