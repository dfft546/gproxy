use sea_orm::entity::prelude::*;
use time::OffsetDateTime;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "upstream_usages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique_key = "upstream_usage_upstream_request_id")]
    pub upstream_request_id: i64,
    pub trace_id: Option<String>,
    pub at: OffsetDateTime,
    pub user_id: Option<i64>,
    pub user_key_id: Option<i64>,
    pub provider: String,
    pub credential_id: Option<i64>,
    pub internal: bool,
    pub attempt_no: i32,
    pub operation: String,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub created_at: OffsetDateTime,
    #[sea_orm(
        belongs_to,
        from = "upstream_request_id",
        to = "id",
        on_delete = "Cascade"
    )]
    pub upstream_request: HasOne<super::upstream_requests::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
