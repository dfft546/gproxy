use gproxy_protocol::claude::create_message::request::CreateMessageRequest as ClaudeCreateMessageRequest;
use gproxy_protocol::gemini::generate_content::request::GenerateContentRequest as GeminiGenerateContentRequest;
use gproxy_protocol::gemini::stream_content::request::StreamGenerateContentRequest as GeminiStreamGenerateContentRequest;
use gproxy_protocol::openai::create_chat_completions::request::CreateChatCompletionRequest as OpenAIChatCompletionRequest;
use gproxy_protocol::openai::create_chat_completions::types::ChatCompletionStreamOptions;
use gproxy_protocol::openai::create_response::request::CreateResponseRequest as OpenAIResponseRequest;

use super::helpers::ensure_generate_proto;
use super::types::{
    GenerateContentRequest, GenerateContentResponse, Op, Proto, Request, Response,
    TransformContext, TransformError,
};

use crate::generate_content;

pub(crate) fn transform_generate_request(
    ctx: &TransformContext,
    req: GenerateContentRequest,
) -> Result<Request, TransformError> {
    if !matches!(ctx.src_op, Op::GenerateContent | Op::StreamGenerateContent)
        || !matches!(ctx.dst_op, Op::GenerateContent | Op::StreamGenerateContent)
    {
        return Err(TransformError::OpMismatch);
    }
    ensure_generate_proto(ctx.src)?;
    ensure_generate_proto(ctx.dst)?;

    let src_stream = request_is_stream(&req);
    if src_stream != op_is_stream(ctx.src_op) {
        return Err(TransformError::StreamMismatch);
    }
    let dst_stream = op_is_stream(ctx.dst_op);

    let result = match (ctx.src, ctx.dst, req) {
        (Proto::Claude, Proto::Claude, GenerateContentRequest::Claude(mut req)) => {
            set_stream_flag_claude(&mut req, dst_stream);
            GenerateContentRequest::Claude(req)
        }
        (Proto::Claude, Proto::OpenAIChat, GenerateContentRequest::Claude(req)) => {
            let mut out =
                generate_content::claude2openai_chat_completions::request::transform_request(req);
            set_stream_flag_openai_chat(&mut out, dst_stream);
            GenerateContentRequest::OpenAIChat(out)
        }
        (Proto::Claude, Proto::OpenAIResponse, GenerateContentRequest::Claude(req)) => {
            let mut out = generate_content::claude2openai_response::request::transform_request(req);
            set_stream_flag_openai_response(&mut out, dst_stream);
            GenerateContentRequest::OpenAIResponse(out)
        }
        (Proto::Claude, Proto::Gemini, GenerateContentRequest::Claude(req)) => {
            let out = generate_content::claude2gemini::request::transform_request(req);
            gemini_request_with_stream(out, dst_stream)
        }
        (Proto::OpenAIChat, Proto::OpenAIChat, GenerateContentRequest::OpenAIChat(mut req)) => {
            set_stream_flag_openai_chat(&mut req, dst_stream);
            GenerateContentRequest::OpenAIChat(req)
        }
        (Proto::OpenAIChat, Proto::Claude, GenerateContentRequest::OpenAIChat(req)) => {
            let mut out =
                generate_content::openai_chat_completions2claude::request::transform_request(req);
            set_stream_flag_claude(&mut out, dst_stream);
            GenerateContentRequest::Claude(out)
        }
        (Proto::OpenAIChat, Proto::OpenAIResponse, GenerateContentRequest::OpenAIChat(req)) => {
            let mut out = generate_content::openai_chat_completions2openai_response::request::transform_request(req);
            set_stream_flag_openai_response(&mut out, dst_stream);
            GenerateContentRequest::OpenAIResponse(out)
        }
        (Proto::OpenAIChat, Proto::Gemini, GenerateContentRequest::OpenAIChat(req)) => {
            let out =
                generate_content::openai_chat_completions2gemini::request::transform_request(req);
            gemini_request_with_stream(out, dst_stream)
        }
        (
            Proto::OpenAIResponse,
            Proto::OpenAIResponse,
            GenerateContentRequest::OpenAIResponse(mut req),
        ) => {
            set_stream_flag_openai_response(&mut req, dst_stream);
            GenerateContentRequest::OpenAIResponse(req)
        }
        (Proto::OpenAIResponse, Proto::Claude, GenerateContentRequest::OpenAIResponse(req)) => {
            let mut out = generate_content::openai_response2claude::request::transform_request(req);
            set_stream_flag_claude(&mut out, dst_stream);
            GenerateContentRequest::Claude(out)
        }
        (Proto::OpenAIResponse, Proto::OpenAIChat, GenerateContentRequest::OpenAIResponse(req)) => {
            let mut out = generate_content::openai_response2openai_chat_completions::request::transform_request(req);
            set_stream_flag_openai_chat(&mut out, dst_stream);
            GenerateContentRequest::OpenAIChat(out)
        }
        (Proto::OpenAIResponse, Proto::Gemini, GenerateContentRequest::OpenAIResponse(req)) => {
            let out = generate_content::openai_response2gemini::request::transform_request(req);
            gemini_request_with_stream(out, dst_stream)
        }
        (Proto::Gemini, Proto::Gemini, req) => match (req, dst_stream) {
            (GenerateContentRequest::Gemini(base), true) => {
                GenerateContentRequest::GeminiStream(GeminiStreamGenerateContentRequest {
                    path: base.path,
                    body: base.body,
                    query: None,
                })
            }
            (GenerateContentRequest::Gemini(base), false) => GenerateContentRequest::Gemini(base),
            (GenerateContentRequest::GeminiStream(stream), true) => {
                GenerateContentRequest::GeminiStream(stream)
            }
            (GenerateContentRequest::GeminiStream(stream), false) => {
                GenerateContentRequest::Gemini(GeminiGenerateContentRequest {
                    path: stream.path,
                    body: stream.body,
                })
            }
            _ => {
                return Err(TransformError::ProtoMismatch);
            }
        },
        (Proto::Gemini, Proto::Claude, req) => {
            let base = normalize_gemini_request(req)?;
            let mut out = generate_content::gemini2claude::request::transform_request(base);
            set_stream_flag_claude(&mut out, dst_stream);
            GenerateContentRequest::Claude(out)
        }
        (Proto::Gemini, Proto::OpenAIChat, req) => {
            let base = normalize_gemini_request(req)?;
            let mut out =
                generate_content::gemini2openai_chat_completions::request::transform_request(base);
            set_stream_flag_openai_chat(&mut out, dst_stream);
            GenerateContentRequest::OpenAIChat(out)
        }
        (Proto::Gemini, Proto::OpenAIResponse, req) => {
            let base = normalize_gemini_request(req)?;
            let mut out =
                generate_content::gemini2openai_response::request::transform_request(base);
            set_stream_flag_openai_response(&mut out, dst_stream);
            GenerateContentRequest::OpenAIResponse(out)
        }
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Request::GenerateContent(result))
}

pub(crate) fn transform_generate_response(
    ctx: &TransformContext,
    resp: GenerateContentResponse,
) -> Result<Response, TransformError> {
    if op_is_stream(ctx.src_op) || op_is_stream(ctx.dst_op) {
        return Err(TransformError::StreamMismatch);
    }
    ensure_generate_proto(ctx.src)?;
    ensure_generate_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, resp) {
        (Proto::Claude, Proto::Claude, GenerateContentResponse::Claude(resp)) => {
            GenerateContentResponse::Claude(resp)
        }
        (Proto::Claude, Proto::OpenAIChat, GenerateContentResponse::Claude(resp)) => {
            GenerateContentResponse::OpenAIChat(
                generate_content::openai_chat_completions2claude::response::transform_response(resp),
            )
        }
        (Proto::Claude, Proto::OpenAIResponse, GenerateContentResponse::Claude(resp)) => {
            GenerateContentResponse::OpenAIResponse(
                generate_content::claude2openai_response::response::transform_response(resp),
            )
        }
        (Proto::Claude, Proto::Gemini, GenerateContentResponse::Claude(resp)) => {
            GenerateContentResponse::Gemini(
                generate_content::gemini2claude::response::transform_response(resp),
            )
        }
        (Proto::OpenAIChat, Proto::OpenAIChat, GenerateContentResponse::OpenAIChat(resp)) => {
            GenerateContentResponse::OpenAIChat(resp)
        }
        (Proto::OpenAIChat, Proto::Claude, GenerateContentResponse::OpenAIChat(resp)) => {
            GenerateContentResponse::Claude(
                generate_content::claude2openai_chat_completions::response::transform_response(resp),
            )
        }
        (Proto::OpenAIChat, Proto::OpenAIResponse, GenerateContentResponse::OpenAIChat(resp)) => {
            GenerateContentResponse::OpenAIResponse(
                generate_content::openai_response2openai_chat_completions::response::transform_response(resp),
            )
        }
        (Proto::OpenAIChat, Proto::Gemini, GenerateContentResponse::OpenAIChat(resp)) => {
            GenerateContentResponse::Gemini(
                generate_content::gemini2openai_chat_completions::response::transform_response(resp),
            )
        }
        (Proto::OpenAIResponse, Proto::OpenAIResponse, GenerateContentResponse::OpenAIResponse(resp)) => {
            GenerateContentResponse::OpenAIResponse(resp)
        }
        (Proto::OpenAIResponse, Proto::Claude, GenerateContentResponse::OpenAIResponse(resp)) => {
            GenerateContentResponse::Claude(
                generate_content::openai_response2claude::response::transform_response(resp),
            )
        }
        (Proto::OpenAIResponse, Proto::OpenAIChat, GenerateContentResponse::OpenAIResponse(resp)) => {
            GenerateContentResponse::OpenAIChat(
                generate_content::openai_chat_completions2openai_response::response::transform_response(resp),
            )
        }
        (Proto::OpenAIResponse, Proto::Gemini, GenerateContentResponse::OpenAIResponse(resp)) => {
            GenerateContentResponse::Gemini(
                generate_content::gemini2openai_response::response::transform_response(resp),
            )
        }
        (Proto::Gemini, Proto::Gemini, GenerateContentResponse::Gemini(resp)) => {
            GenerateContentResponse::Gemini(resp)
        }
        (Proto::Gemini, Proto::Claude, GenerateContentResponse::Gemini(resp)) => {
            GenerateContentResponse::Claude(
                generate_content::claude2gemini::response::transform_response(resp),
            )
        }
        (Proto::Gemini, Proto::OpenAIChat, GenerateContentResponse::Gemini(resp)) => {
            GenerateContentResponse::OpenAIChat(
                generate_content::openai_chat_completions2gemini::response::transform_response(resp),
            )
        }
        (Proto::Gemini, Proto::OpenAIResponse, GenerateContentResponse::Gemini(resp)) => {
            GenerateContentResponse::OpenAIResponse(
                generate_content::openai_response2gemini::response::transform_response(resp),
            )
        }
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Response::GenerateContent(result))
}

fn request_is_stream(req: &GenerateContentRequest) -> bool {
    match req {
        GenerateContentRequest::Claude(req) => req.body.stream.unwrap_or(false),
        GenerateContentRequest::OpenAIChat(req) => req.body.stream.unwrap_or(false),
        GenerateContentRequest::OpenAIResponse(req) => req.body.stream.unwrap_or(false),
        GenerateContentRequest::Gemini(_) => false,
        GenerateContentRequest::GeminiStream(_) => true,
    }
}

fn op_is_stream(op: Op) -> bool {
    matches!(op, Op::StreamGenerateContent)
}

fn normalize_gemini_request(
    req: GenerateContentRequest,
) -> Result<GeminiGenerateContentRequest, TransformError> {
    match req {
        GenerateContentRequest::Gemini(req) => Ok(req),
        GenerateContentRequest::GeminiStream(req) => Ok(GeminiGenerateContentRequest {
            path: req.path,
            body: req.body,
        }),
        _ => Err(TransformError::ProtoMismatch),
    }
}

fn gemini_request_with_stream(
    req: GeminiGenerateContentRequest,
    stream: bool,
) -> GenerateContentRequest {
    if stream {
        GenerateContentRequest::GeminiStream(GeminiStreamGenerateContentRequest {
            path: req.path,
            body: req.body,
            // For transformed stream requests, force SSE framing so upstream
            // emits event-wise chunks that stream transformers can decode reliably.
            query: Some("alt=sse".to_string()),
        })
    } else {
        GenerateContentRequest::Gemini(req)
    }
}

fn set_stream_flag_claude(req: &mut ClaudeCreateMessageRequest, stream: bool) {
    req.body.stream = Some(stream);
}

fn set_stream_flag_openai_chat(req: &mut OpenAIChatCompletionRequest, stream: bool) {
    req.body.stream = Some(stream);
    if !stream {
        req.body.stream_options = None;
        return;
    }
    let opts = req
        .body
        .stream_options
        .get_or_insert(ChatCompletionStreamOptions {
            include_usage: None,
            include_obfuscation: None,
        });
    if opts.include_usage.is_none() {
        opts.include_usage = Some(true);
    }
}

fn set_stream_flag_openai_response(req: &mut OpenAIResponseRequest, stream: bool) {
    req.body.stream = Some(stream);
    if !stream {
        req.body.stream_options = None;
    }
}
