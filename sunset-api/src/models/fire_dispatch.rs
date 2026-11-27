use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// One live case from a county fire department's public case list.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct FireDispatchCase {
    /// When the dispatch center received the case; null when the source omits it (New Taipei).
    #[schema(example = "2026-11-27T13:53:20Z")]
    pub received_at: Option<DateTime<Utc>>,
    /// Case category, joined as `案類-案別` when the source splits them.
    #[schema(example = "緊急救護-車禍")]
    pub category: String,
    /// Incident location as published; ambulance cases are often truncated to the district.
    #[schema(example = "台南市永康區")]
    pub location: String,
    /// Dispatched station or vehicles (派遣分隊), when published.
    #[schema(example = "鹽行分隊")]
    pub unit: Option<String>,
    /// Current status (執行狀況), when published.
    #[schema(example = "已出動")]
    pub status: Option<String>,
    /// Source case number, when published (Tainan, New Taipei).
    #[schema(example = "215320")]
    pub case_id: Option<String>,
    /// WGS84 latitude, when published (New Taipei).
    #[schema(example = json!(null))]
    pub latitude: Option<f64>,
    /// WGS84 longitude, when published (New Taipei).
    #[schema(example = json!(null))]
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FireDispatchCountyInfo {
    /// County code used by the other fire dispatch endpoints.
    #[schema(example = "tainan")]
    pub code: String,
    /// County name.
    #[schema(example = "臺南市")]
    pub name: String,
    /// Fire department case list the cases are read from.
    #[schema(example = "https://119dts.tncfd.gov.tw/DTS/caselist/html")]
    pub source_url: String,
}

/// Live cases of one county, fetched on demand and cached for 60 seconds.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FireDispatchCounty {
    /// County code.
    #[schema(example = "tainan")]
    pub code: String,
    /// County name.
    #[schema(example = "臺南市")]
    pub name: String,
    /// Fire department case list the cases are read from.
    #[schema(example = "https://119dts.tncfd.gov.tw/DTS/caselist/html")]
    pub source_url: String,
    /// When the source was read; null when the request failed.
    #[schema(example = "2026-11-27T13:54:37Z")]
    pub fetched_at: Option<DateTime<Utc>>,
    /// Why the source could not be read (only in `/fire-dispatch/latest`); `cases` is then empty.
    #[schema(example = json!(null))]
    pub error: Option<String>,
    /// Cases received in the last 6 hours, newest first; cases without `received_at` last.
    pub cases: Vec<FireDispatchCase>,
}

/// One stored state of a case: a new record each time a case's status or unit changes.
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FireDispatchRecord {
    /// Record id; stable, use it to drop repeats when polling with `since`.
    #[schema(example = 19297)]
    pub id: i32,
    /// County code.
    #[schema(example = "tainan")]
    pub county: String,
    /// When the dispatch center received the case; null for New Taipei.
    #[schema(example = "2026-11-27T13:53:20Z")]
    pub received_at: Option<DateTime<Utc>>,
    /// Case category (`案類-案別`).
    #[schema(example = "緊急救護")]
    pub category: String,
    /// Incident location as published.
    #[schema(example = "台南市永康區")]
    pub location: String,
    /// Dispatched station or vehicles in this state.
    #[schema(example = "鹽行分隊")]
    pub unit: Option<String>,
    /// Status in this state.
    #[schema(example = "已出動")]
    pub status: Option<String>,
    /// Source case number (Tainan, New Taipei).
    #[schema(example = "215320")]
    pub case_id: Option<String>,
    /// WGS84 latitude (New Taipei).
    #[schema(example = json!(null))]
    pub latitude: Option<f64>,
    /// WGS84 longitude (New Taipei).
    #[schema(example = json!(null))]
    pub longitude: Option<f64>,
    /// When the crawler first saw this state.
    #[schema(example = "2026-11-27T13:54:37.205116Z")]
    pub first_seen_at: DateTime<Utc>,
    /// When the crawler last saw this state in the live list.
    #[schema(example = "2026-11-27T13:58:07.512094Z")]
    pub last_seen_at: DateTime<Utc>,
}

/// One stored state within a case's history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FireDispatchCaseState {
    /// Record id in `/fire-dispatch/records`.
    #[schema(example = 19297)]
    pub id: i32,
    /// Status in this state.
    #[schema(example = "已出動")]
    pub status: Option<String>,
    /// Dispatched station or vehicles in this state.
    #[schema(example = "鹽行分隊")]
    pub unit: Option<String>,
    /// When the crawler first saw this state.
    #[schema(example = "2026-11-27T13:54:37.205116Z")]
    pub first_seen_at: DateTime<Utc>,
    /// When the crawler last saw this state.
    #[schema(example = "2026-11-27T13:58:07.512094Z")]
    pub last_seen_at: DateTime<Utc>,
}

/// Stored records merged into one case; current fields come from the latest-seen state.
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FireDispatchStoredCase {
    /// County code.
    #[schema(example = "tainan")]
    pub county: String,
    /// When the dispatch center received the case; null for New Taipei.
    #[schema(example = "2026-11-27T13:53:20Z")]
    pub received_at: Option<DateTime<Utc>>,
    /// Case category (`案類-案別`).
    #[schema(example = "緊急救護")]
    pub category: String,
    /// Incident location as published.
    #[schema(example = "台南市永康區")]
    pub location: String,
    /// Source case number (Tainan, New Taipei); the case key with `county` when present.
    #[schema(example = "215320")]
    pub case_id: Option<String>,
    /// Latest-seen status.
    #[schema(example = "已到達")]
    pub status: Option<String>,
    /// Latest-seen dispatched station or vehicles.
    #[schema(example = "鹽行分隊")]
    pub unit: Option<String>,
    /// Latest-seen WGS84 latitude (New Taipei).
    #[schema(example = json!(null))]
    pub latitude: Option<f64>,
    /// Latest-seen WGS84 longitude (New Taipei).
    #[schema(example = json!(null))]
    pub longitude: Option<f64>,
    /// When the crawler first saw any state of the case.
    #[schema(example = "2026-11-27T13:54:37.205116Z")]
    pub first_seen_at: DateTime<Utc>,
    /// When the crawler last saw any state of the case.
    #[schema(example = "2026-11-27T14:02:38.004417Z")]
    pub last_seen_at: DateTime<Utc>,
    /// Every state of the case, in the order first seen.
    #[sqlx(json)]
    pub history: Vec<FireDispatchCaseState>,
}
