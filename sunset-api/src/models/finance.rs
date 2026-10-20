use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FinanceExchangeRate {
    pub id: i32,
    /// ISO 4217 currency being priced (for example, `USD`).
    pub base: String,
    /// ISO 4217 currency the rate is expressed in; currently always `TWD`.
    pub quote: String,
    /// Amount of `quote` for one unit of `base`, from Google Finance.
    pub rate: f64,
    pub observed_at: DateTime<Utc>,
}

/// Real-time rate fetched from the Apps Script source and cached in memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LiveExchangeRate {
    /// ISO 4217 currency being priced (for example, `USD`).
    pub base: String,
    /// ISO 4217 currency the rate is expressed in; currently always `TWD`.
    pub quote: String,
    /// Amount of `quote` for one unit of `base`, from Google Finance.
    pub rate: f64,
    /// When the Apps Script source read the rates.
    pub observed_at: DateTime<Utc>,
}
