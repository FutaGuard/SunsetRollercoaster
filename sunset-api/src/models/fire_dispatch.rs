use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// One live case from a county fire department's public case list.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct FireDispatchCase {
    /// When the dispatch center received the case; null when the source omits it (New Taipei).
    pub received_at: Option<DateTime<Utc>>,
    /// Case category, joined as `案類-案別` when the source splits them (for example, `緊急救護-車禍`).
    pub category: String,
    /// Incident location as published; ambulance cases are often truncated to the district.
    pub location: String,
    /// Dispatched station or vehicles (派遣分隊), when published.
    pub unit: Option<String>,
    /// Current status (執行狀況), for example `已出動`, when published.
    pub status: Option<String>,
    /// Source case number, when published (Tainan, New Taipei).
    pub case_id: Option<String>,
    /// WGS84 latitude, when published (New Taipei).
    pub latitude: Option<f64>,
    /// WGS84 longitude, when published (New Taipei).
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FireDispatchCountyInfo {
    /// County code used in `/fire-dispatch/latest/{county}` (for example, `tainan`).
    pub code: String,
    /// County name (for example, `臺南市`).
    pub name: String,
    /// Fire department case list the cases are read from.
    pub source_url: String,
}

/// Live cases of one county, fetched on demand and cached for 60 seconds.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FireDispatchCounty {
    pub code: String,
    pub name: String,
    pub source_url: String,
    /// When the source was read; null when the request failed.
    pub fetched_at: Option<DateTime<Utc>>,
    /// Why the source could not be read (only in `/fire-dispatch/latest`); `cases` is then empty.
    pub error: Option<String>,
    /// Cases received in the last 6 hours, newest first; cases without `received_at` last.
    pub cases: Vec<FireDispatchCase>,
}
