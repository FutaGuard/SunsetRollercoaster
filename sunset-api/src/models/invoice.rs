use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Invoice {
    pub id: i32,
    pub date: NaiveDate,
    pub special_prize: i32,
    pub grand_prize: i32,
    #[schema(value_type = Vec<i32>)]
    pub first_prize: serde_json::Value,
}
