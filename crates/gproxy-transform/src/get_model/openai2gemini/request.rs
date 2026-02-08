use gproxy_protocol::gemini::get_model::request::{
    GetModelPath as GeminiGetModelPath, GetModelRequest as GeminiGetModelRequest,
};
use gproxy_protocol::openai::get_model::request::GetModelRequest as OpenAIGetModelRequest;

/// Convert an OpenAI get-model request into a Gemini get-model request.
pub fn transform_request(request: OpenAIGetModelRequest) -> GeminiGetModelRequest {
    let name = request.path.model;
    let name = if name.starts_with("models/") {
        name
    } else {
        format!("models/{name}")
    };

    GeminiGetModelRequest {
        path: GeminiGetModelPath { name },
    }
}
