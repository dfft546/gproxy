use sea_orm::entity::prelude::*;
use time::OffsetDateTime;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "internal_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub event_type: String,
    pub payload_json: Json,
    pub at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

impl ActiveModelBehavior for ActiveModel {}
