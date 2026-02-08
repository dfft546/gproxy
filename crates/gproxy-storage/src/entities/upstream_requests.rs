use sea_orm::entity::prelude::*;
use time::OffsetDateTime;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "upstream_requests")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub trace_id: Option<String>,
    pub at: OffsetDateTime,
    pub user_id: Option<i64>,
    pub user_key_id: Option<i64>,
    pub provider: String,
    pub credential_id: Option<i64>,
    pub internal: bool,
    pub attempt_no: i32,
    pub operation: String,
    pub request_method: String,
    pub request_headers_json: Json,
    pub request_path: String,
    pub request_query: Option<String>,
    pub request_body: Option<Vec<u8>>,
    pub response_status: Option<i32>,
    pub response_headers_json: Json,
    pub response_body: Option<Vec<u8>>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub transport_kind: Option<String>,
    pub created_at: OffsetDateTime,
}

impl ActiveModelBehavior for ActiveModel {}
