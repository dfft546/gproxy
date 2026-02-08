use super::generate::{transform_generate_request, transform_generate_response};
use super::helpers::{ensure_basic_proto, ensure_non_generate};
use super::types::{
    CountTokensRequest, CountTokensResponse, ModelGetRequest, ModelGetResponse, ModelListRequest,
    ModelListResponse, Op, Request, Response, TransformContext, TransformError,
};

use crate::count_tokens;
use crate::get_model;
use crate::list_models;

pub fn transform_request(ctx: &TransformContext, req: Request) -> Result<Request, TransformError> {
    match (ctx.src_op, req) {
        (Op::ModelList, Request::ModelList(req)) => transform_model_list_request(ctx, req),
        (Op::ModelGet, Request::ModelGet(req)) => transform_model_get_request(ctx, req),
        (Op::CountTokens, Request::CountTokens(req)) => transform_count_tokens_request(ctx, req),
        (Op::GenerateContent | Op::StreamGenerateContent, Request::GenerateContent(req)) => {
            transform_generate_request(ctx, req)
        }
        _ => Err(TransformError::OpMismatch),
    }
}

pub fn transform_response(
    ctx: &TransformContext,
    resp: Response,
) -> Result<Response, TransformError> {
    match (ctx.src_op, resp) {
        (Op::ModelList, Response::ModelList(resp)) => transform_model_list_response(ctx, resp),
        (Op::ModelGet, Response::ModelGet(resp)) => transform_model_get_response(ctx, resp),
        (Op::CountTokens, Response::CountTokens(resp)) => {
            transform_count_tokens_response(ctx, resp)
        }
        (Op::GenerateContent, Response::GenerateContent(resp)) => {
            transform_generate_response(ctx, resp)
        }
        _ => Err(TransformError::OpMismatch),
    }
}

fn transform_model_list_request(
    ctx: &TransformContext,
    req: ModelListRequest,
) -> Result<Request, TransformError> {
    ensure_non_generate(ctx)?;
    ensure_basic_proto(ctx.src)?;
    ensure_basic_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, req) {
        (
            super::types::Proto::Claude,
            super::types::Proto::Claude,
            ModelListRequest::Claude(req),
        ) => ModelListRequest::Claude(req),
        (
            super::types::Proto::Claude,
            super::types::Proto::OpenAI,
            ModelListRequest::Claude(req),
        ) => ModelListRequest::OpenAI(list_models::claude2openai::request::transform_request(req)),
        (
            super::types::Proto::Claude,
            super::types::Proto::Gemini,
            ModelListRequest::Claude(req),
        ) => ModelListRequest::Gemini(list_models::claude2gemini::request::transform_request(req)),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::OpenAI,
            ModelListRequest::OpenAI(req),
        ) => ModelListRequest::OpenAI(req),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Claude,
            ModelListRequest::OpenAI(req),
        ) => ModelListRequest::Claude(list_models::openai2claude::request::transform_request(req)),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Gemini,
            ModelListRequest::OpenAI(req),
        ) => ModelListRequest::Gemini(list_models::openai2gemini::request::transform_request(req)),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Gemini,
            ModelListRequest::Gemini(req),
        ) => ModelListRequest::Gemini(req),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Claude,
            ModelListRequest::Gemini(req),
        ) => ModelListRequest::Claude(list_models::gemini2claude::request::transform_request(req)),
        (
            super::types::Proto::Gemini,
            super::types::Proto::OpenAI,
            ModelListRequest::Gemini(req),
        ) => ModelListRequest::OpenAI(list_models::gemini2openai::request::transform_request(req)),
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Request::ModelList(result))
}

fn transform_model_list_response(
    ctx: &TransformContext,
    resp: ModelListResponse,
) -> Result<Response, TransformError> {
    ensure_non_generate(ctx)?;
    ensure_basic_proto(ctx.src)?;
    ensure_basic_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, resp) {
        (
            super::types::Proto::Claude,
            super::types::Proto::Claude,
            ModelListResponse::Claude(resp),
        ) => ModelListResponse::Claude(resp),
        (
            super::types::Proto::Claude,
            super::types::Proto::OpenAI,
            ModelListResponse::Claude(resp),
        ) => ModelListResponse::OpenAI(list_models::openai2claude::response::transform_response(
            resp,
        )),
        (
            super::types::Proto::Claude,
            super::types::Proto::Gemini,
            ModelListResponse::Claude(resp),
        ) => ModelListResponse::Gemini(list_models::gemini2claude::response::transform_response(
            resp,
        )),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::OpenAI,
            ModelListResponse::OpenAI(resp),
        ) => ModelListResponse::OpenAI(resp),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Claude,
            ModelListResponse::OpenAI(resp),
        ) => ModelListResponse::Claude(list_models::claude2openai::response::transform_response(
            resp,
        )),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Gemini,
            ModelListResponse::OpenAI(resp),
        ) => ModelListResponse::Gemini(list_models::gemini2openai::response::transform_response(
            resp,
        )),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Gemini,
            ModelListResponse::Gemini(resp),
        ) => ModelListResponse::Gemini(resp),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Claude,
            ModelListResponse::Gemini(resp),
        ) => ModelListResponse::Claude(list_models::claude2gemini::response::transform_response(
            resp,
        )),
        (
            super::types::Proto::Gemini,
            super::types::Proto::OpenAI,
            ModelListResponse::Gemini(resp),
        ) => ModelListResponse::OpenAI(list_models::openai2gemini::response::transform_response(
            resp,
        )),
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Response::ModelList(result))
}

fn transform_model_get_request(
    ctx: &TransformContext,
    req: ModelGetRequest,
) -> Result<Request, TransformError> {
    ensure_non_generate(ctx)?;
    ensure_basic_proto(ctx.src)?;
    ensure_basic_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, req) {
        (
            super::types::Proto::Claude,
            super::types::Proto::Claude,
            ModelGetRequest::Claude(req),
        ) => ModelGetRequest::Claude(req),
        (
            super::types::Proto::Claude,
            super::types::Proto::OpenAI,
            ModelGetRequest::Claude(req),
        ) => ModelGetRequest::OpenAI(get_model::claude2openai::request::transform_request(req)),
        (
            super::types::Proto::Claude,
            super::types::Proto::Gemini,
            ModelGetRequest::Claude(req),
        ) => ModelGetRequest::Gemini(get_model::claude2gemini::request::transform_request(req)),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::OpenAI,
            ModelGetRequest::OpenAI(req),
        ) => ModelGetRequest::OpenAI(req),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Claude,
            ModelGetRequest::OpenAI(req),
        ) => ModelGetRequest::Claude(get_model::openai2claude::request::transform_request(req)),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Gemini,
            ModelGetRequest::OpenAI(req),
        ) => ModelGetRequest::Gemini(get_model::openai2gemini::request::transform_request(req)),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Gemini,
            ModelGetRequest::Gemini(req),
        ) => ModelGetRequest::Gemini(req),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Claude,
            ModelGetRequest::Gemini(req),
        ) => ModelGetRequest::Claude(get_model::gemini2claude::request::transform_request(req)),
        (
            super::types::Proto::Gemini,
            super::types::Proto::OpenAI,
            ModelGetRequest::Gemini(req),
        ) => ModelGetRequest::OpenAI(get_model::gemini2openai::request::transform_request(req)),
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Request::ModelGet(result))
}

fn transform_model_get_response(
    ctx: &TransformContext,
    resp: ModelGetResponse,
) -> Result<Response, TransformError> {
    ensure_non_generate(ctx)?;
    ensure_basic_proto(ctx.src)?;
    ensure_basic_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, resp) {
        (
            super::types::Proto::Claude,
            super::types::Proto::Claude,
            ModelGetResponse::Claude(resp),
        ) => ModelGetResponse::Claude(resp),
        (
            super::types::Proto::Claude,
            super::types::Proto::OpenAI,
            ModelGetResponse::Claude(resp),
        ) => ModelGetResponse::OpenAI(get_model::openai2claude::response::transform_response(resp)),
        (
            super::types::Proto::Claude,
            super::types::Proto::Gemini,
            ModelGetResponse::Claude(resp),
        ) => ModelGetResponse::Gemini(get_model::gemini2claude::response::transform_response(resp)),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::OpenAI,
            ModelGetResponse::OpenAI(resp),
        ) => ModelGetResponse::OpenAI(resp),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Claude,
            ModelGetResponse::OpenAI(resp),
        ) => ModelGetResponse::Claude(get_model::claude2openai::response::transform_response(resp)),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Gemini,
            ModelGetResponse::OpenAI(resp),
        ) => ModelGetResponse::Gemini(get_model::gemini2openai::response::transform_response(resp)),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Gemini,
            ModelGetResponse::Gemini(resp),
        ) => ModelGetResponse::Gemini(resp),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Claude,
            ModelGetResponse::Gemini(resp),
        ) => ModelGetResponse::Claude(get_model::claude2gemini::response::transform_response(resp)),
        (
            super::types::Proto::Gemini,
            super::types::Proto::OpenAI,
            ModelGetResponse::Gemini(resp),
        ) => ModelGetResponse::OpenAI(get_model::openai2gemini::response::transform_response(resp)),
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Response::ModelGet(result))
}

fn transform_count_tokens_request(
    ctx: &TransformContext,
    req: CountTokensRequest,
) -> Result<Request, TransformError> {
    ensure_non_generate(ctx)?;
    ensure_basic_proto(ctx.src)?;
    ensure_basic_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, req) {
        (
            super::types::Proto::Claude,
            super::types::Proto::Claude,
            CountTokensRequest::Claude(req),
        ) => CountTokensRequest::Claude(req),
        (
            super::types::Proto::Claude,
            super::types::Proto::OpenAI,
            CountTokensRequest::Claude(req),
        ) => {
            CountTokensRequest::OpenAI(count_tokens::claude2openai::request::transform_request(req))
        }
        (
            super::types::Proto::Claude,
            super::types::Proto::Gemini,
            CountTokensRequest::Claude(req),
        ) => {
            CountTokensRequest::Gemini(count_tokens::claude2gemini::request::transform_request(req))
        }
        (
            super::types::Proto::OpenAI,
            super::types::Proto::OpenAI,
            CountTokensRequest::OpenAI(req),
        ) => CountTokensRequest::OpenAI(req),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Claude,
            CountTokensRequest::OpenAI(req),
        ) => {
            CountTokensRequest::Claude(count_tokens::openai2claude::request::transform_request(req))
        }
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Gemini,
            CountTokensRequest::OpenAI(req),
        ) => {
            CountTokensRequest::Gemini(count_tokens::openai2gemini::request::transform_request(req))
        }
        (
            super::types::Proto::Gemini,
            super::types::Proto::Gemini,
            CountTokensRequest::Gemini(req),
        ) => CountTokensRequest::Gemini(req),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Claude,
            CountTokensRequest::Gemini(req),
        ) => {
            CountTokensRequest::Claude(count_tokens::gemini2claude::request::transform_request(req))
        }
        (
            super::types::Proto::Gemini,
            super::types::Proto::OpenAI,
            CountTokensRequest::Gemini(req),
        ) => {
            CountTokensRequest::OpenAI(count_tokens::gemini2openai::request::transform_request(req))
        }
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Request::CountTokens(result))
}

fn transform_count_tokens_response(
    ctx: &TransformContext,
    resp: CountTokensResponse,
) -> Result<Response, TransformError> {
    ensure_non_generate(ctx)?;
    ensure_basic_proto(ctx.src)?;
    ensure_basic_proto(ctx.dst)?;

    let result = match (ctx.src, ctx.dst, resp) {
        (
            super::types::Proto::Claude,
            super::types::Proto::Claude,
            CountTokensResponse::Claude(resp),
        ) => CountTokensResponse::Claude(resp),
        (
            super::types::Proto::Claude,
            super::types::Proto::OpenAI,
            CountTokensResponse::Claude(resp),
        ) => CountTokensResponse::OpenAI(
            count_tokens::openai2claude::response::transform_response(resp),
        ),
        (
            super::types::Proto::Claude,
            super::types::Proto::Gemini,
            CountTokensResponse::Claude(resp),
        ) => CountTokensResponse::Gemini(
            count_tokens::gemini2claude::response::transform_response(resp),
        ),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::OpenAI,
            CountTokensResponse::OpenAI(resp),
        ) => CountTokensResponse::OpenAI(resp),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Claude,
            CountTokensResponse::OpenAI(resp),
        ) => CountTokensResponse::Claude(
            count_tokens::claude2openai::response::transform_response(resp),
        ),
        (
            super::types::Proto::OpenAI,
            super::types::Proto::Gemini,
            CountTokensResponse::OpenAI(resp),
        ) => CountTokensResponse::Gemini(
            count_tokens::gemini2openai::response::transform_response(resp),
        ),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Gemini,
            CountTokensResponse::Gemini(resp),
        ) => CountTokensResponse::Gemini(resp),
        (
            super::types::Proto::Gemini,
            super::types::Proto::Claude,
            CountTokensResponse::Gemini(resp),
        ) => CountTokensResponse::Claude(
            count_tokens::claude2gemini::response::transform_response(resp),
        ),
        (
            super::types::Proto::Gemini,
            super::types::Proto::OpenAI,
            CountTokensResponse::Gemini(resp),
        ) => CountTokensResponse::OpenAI(
            count_tokens::openai2gemini::response::transform_response(resp),
        ),
        _ => {
            return Err(TransformError::ProtoMismatch);
        }
    };

    Ok(Response::CountTokens(result))
}
