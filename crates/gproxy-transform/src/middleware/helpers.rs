use super::types::{Op, Proto, TransformContext, TransformError};

pub(crate) fn ensure_non_generate(ctx: &TransformContext) -> Result<(), TransformError> {
    if matches!(ctx.src_op, Op::GenerateContent | Op::StreamGenerateContent)
        || matches!(ctx.dst_op, Op::GenerateContent | Op::StreamGenerateContent)
    {
        return Err(TransformError::OpMismatch);
    }
    if ctx.src_op != ctx.dst_op {
        return Err(TransformError::OpMismatch);
    }
    Ok(())
}

pub(crate) fn ensure_basic_proto(proto: Proto) -> Result<(), TransformError> {
    match proto {
        Proto::Claude | Proto::OpenAI | Proto::Gemini => Ok(()),
        _ => Err(TransformError::ProtoMismatch),
    }
}

pub(crate) fn ensure_generate_proto(proto: Proto) -> Result<(), TransformError> {
    match proto {
        Proto::Claude | Proto::OpenAIChat | Proto::OpenAIResponse | Proto::Gemini => Ok(()),
        _ => Err(TransformError::ProtoMismatch),
    }
}
