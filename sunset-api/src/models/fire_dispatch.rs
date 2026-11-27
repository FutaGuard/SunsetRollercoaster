use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

/// One stored state of a case: a new record each time a case's status or unit changes.
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FireDispatchRecord {
    pub id: i32,
    /// County code (for example, `tainan`).
    pub county: String,
    pub received_at: Option<DateTime<Utc>>,
    pub category: String,
    pub location: String,
    pub unit: Option<String>,
    pub status: Option<String>,
    pub case_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// When the crawler first saw this state.
    pub first_seen_at: DateTime<Utc>,
    /// When the crawler last saw this state in the live list.
    pub last_seen_at: DateTime<Utc>,
}

/// One stored state within a case's history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FireDispatchCaseState {
    /// Record id in `/fire-dispatch/records`.
    pub id: i32,
    pub status: Option<String>,
    pub unit: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

/// Stored records merged into one case; current fields come from the latest-seen state.
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FireDispatchStoredCase {
    pub county: String,
    pub received_at: Option<DateTime<Utc>>,
    pub category: String,
    pub location: String,
    pub case_id: Option<String>,
    /// Latest-seen status.
    pub status: Option<String>,
    /// Latest-seen dispatched station or vehicles.
    pub unit: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// When the crawler first saw any state of the case.
    pub first_seen_at: DateTime<Utc>,
    /// When the crawler last saw any state of the case.
    pub last_seen_at: DateTime<Utc>,
    /// Every state of the case, in the order first seen.
    #[sqlx(json)]
    pub history: Vec<FireDispatchCaseState>,
}
