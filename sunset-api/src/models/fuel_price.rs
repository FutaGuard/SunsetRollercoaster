use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NationwideFuelPrice {
    pub id: i32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub unleaded_92: f64,
    pub unleaded_95: f64,
    pub unleaded_98: f64,
    pub super_diesel: f64,
    /// Weekly average West Texas Intermediate price in USD per barrel.
    pub west_texas: Option<f64>,
    /// Weekly average Dubai crude price in USD per barrel.
    pub dubai: Option<f64>,
    /// Weekly average North Sea Brent price in USD per barrel.
    pub brent: Option<f64>,
}
