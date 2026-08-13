use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaipowerPowerSnapshot {
    pub id: i32,
    pub published_at: DateTime<Utc>,
    /// Current system load in MW.
    pub current_load_mw: Option<f64>,
    pub current_utilization_percent: Option<f64>,
    pub forecast_max_supply_mw: Option<f64>,
    pub forecast_peak_demand_mw: Option<f64>,
    pub forecast_peak_reserve_mw: Option<f64>,
    pub forecast_peak_reserve_rate_percent: Option<f64>,
    /// Taipower's reserve indicator code (for example, `G`).
    pub forecast_peak_reserve_indicator: Option<String>,
    pub forecast_peak_hour_range: Option<String>,
    pub yesterday_date: Option<NaiveDate>,
    pub yesterday_max_supply_mw: Option<f64>,
    pub yesterday_peak_demand_mw: Option<f64>,
    pub yesterday_peak_reserve_mw: Option<f64>,
    pub yesterday_peak_reserve_rate_percent: Option<f64>,
    pub yesterday_peak_reserve_indicator: Option<String>,
    pub realtime_max_supply_mw: Option<f64>,
    pub realtime_peak_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaipowerFuelMix {
    pub id: i32,
    pub observed_at: DateTime<Utc>,
    /// Taipower-owned LNG generation in MW.
    pub lng_mw: f64,
    /// Independent power producer LNG generation in MW.
    pub ipp_lng_mw: f64,
    pub coal_mw: f64,
    pub ipp_coal_mw: f64,
    pub cogeneration_mw: f64,
    pub fuel_oil_mw: f64,
    pub solar_mw: f64,
    pub wind_mw: f64,
    pub hydro_mw: f64,
    pub energy_storage_mw: f64,
    pub other_renewable_mw: f64,
    /// Storage charging load in MW; this is normally zero or negative.
    pub energy_storage_load_mw: f64,
    pub total_mw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaipowerAreaLoad {
    pub id: i32,
    pub observed_at: DateTime<Utc>,
    pub north_load_mw: f64,
    pub central_load_mw: f64,
    pub south_load_mw: f64,
    pub east_load_mw: f64,
    pub total_load_mw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaipowerAreaSnapshot {
    pub id: i32,
    pub observed_at: DateTime<Utc>,
    pub north_generation_mw: f64,
    pub north_load_mw: f64,
    pub central_generation_mw: f64,
    pub central_load_mw: f64,
    pub south_generation_mw: f64,
    pub south_load_mw: f64,
    pub east_generation_mw: f64,
    pub east_load_mw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaipowerOperatingReserve {
    pub id: i32,
    pub date: NaiveDate,
    pub peak_load_mw: f64,
    pub reserve_capacity_mw: f64,
    pub reserve_rate_percent: f64,
    /// True for today's published estimate; false after actual data is available.
    pub is_forecast: bool,
    /// Publish time is present for today's forecast and absent for historical actuals.
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaipowerGenerator {
    pub id: i32,
    pub published_at: DateTime<Utc>,
    /// Row order in the source snapshot.
    pub sequence: i32,
    /// Stable source category code such as `lng`, `coal`, or `solar`.
    pub category_code: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub unit_name: String,
    pub installed_capacity_mw: Option<f64>,
    /// Category share included by Taipower on subtotal rows.
    pub installed_capacity_percent: Option<f64>,
    pub net_generation_mw: Option<f64>,
    /// Category share included by Taipower on subtotal rows.
    pub net_generation_percent: Option<f64>,
    pub utilization_percent: Option<f64>,
    pub status: Option<String>,
    pub is_summary: bool,
}
