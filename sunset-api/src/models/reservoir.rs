use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Reservoir {
    pub id: i32,
    pub name: Option<String>,
    pub capavailable: Option<f64>,
    #[sqlx(rename = "statisticTimeS")]
    #[serde(rename = "statisticTimeS")]
    pub statistic_time_s: Option<DateTime<Utc>>,
    #[sqlx(rename = "statisticTimeE")]
    #[serde(rename = "statisticTimeE")]
    pub statistic_time_e: Option<DateTime<Utc>>,
    #[sqlx(rename = "rainFall")]
    #[serde(rename = "rainFall")]
    pub rain_fall: Option<f64>,
    #[sqlx(rename = "inFlow")]
    #[serde(rename = "inFlow")]
    pub in_flow: Option<f64>,
    #[sqlx(rename = "outFlow")]
    #[serde(rename = "outFlow")]
    pub out_flow: Option<f64>,
    pub waterlevediff: Option<f64>,
    #[sqlx(rename = "recordTime")]
    #[serde(rename = "recordTime")]
    pub record_time: Option<DateTime<Utc>>,
    pub caplevel: Option<f64>,
    pub currcap: Option<f64>,
    pub currcapper: Option<f64>,
}
