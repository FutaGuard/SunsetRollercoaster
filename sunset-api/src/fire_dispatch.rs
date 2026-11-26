//! Live fire department dispatch cases (消防局即時案件) read from each county's public
//! case list, normalized to one shape and cached in memory per county.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc};
use scraper::{ElementRef, Html, Node, Selector};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::Instant;

use crate::models::{FireDispatchCase, FireDispatchCounty, FireDispatchCountyInfo};

const CACHE_TTL: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Case lists keep finished and placeholder rows; only cases received this recently are live.
const MAX_CASE_AGE: TimeDelta = TimeDelta::hours(6);
/// Tolerates source clocks that run slightly ahead; anything later is a placeholder row.
const MAX_CASE_SKEW: TimeDelta = TimeDelta::days(1);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
/// www.ntfd.gov.tw chains to TWCA CYBER Root CA, which the bundled webpki roots lack.
/// SHA-256 3F63BB2814BE174EC8B6439CF08D6D56F0B7C405883A5648A334424D6B3EC558, as listed in
/// TWCA's 2025 WebTrust TLS Baseline audit report.
const TWCA_CYBER_ROOT_CA: &[u8] = include_bytes!("../certs/twca-cyber-root-ca.pem");

#[derive(Clone, Copy, Debug)]
enum SourceFormat {
    /// HTML case list: `<table>`, `<div class="tr">`, or `<ul class="rwd-table"><li>`,
    /// with columns identified by header text.
    Grid,
    /// Taipei's JSON case list, requested with POST.
    TaipeiJson,
    /// New Taipei's GeoJSON rescue layer.
    NewTaipeiJson,
}

pub struct County {
    pub code: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    format: SourceFormat,
}

const fn county(
    code: &'static str,
    name: &'static str,
    url: &'static str,
    format: SourceFormat,
) -> County {
    County {
        code,
        name,
        url,
        format,
    }
}

pub const COUNTIES: &[County] = &[
    county(
        "taipei",
        "臺北市",
        "https://service119.tfd.gov.tw/service119/citizenCase/caseList",
        SourceFormat::TaipeiJson,
    ),
    county(
        "new-taipei",
        "新北市",
        "https://e.ntpc.gov.tw/v3/api/map/dynamic/layer/rescue",
        SourceFormat::NewTaipeiJson,
    ),
    county(
        "taoyuan",
        "桃園市",
        "https://www.tyfd.gov.tw/cht/index.php?act=caselist",
        SourceFormat::Grid,
    ),
    county(
        "taichung",
        "臺中市",
        "https://www.fire.taichung.gov.tw/caselist/index.asp?Parser=99,8,226,,,,,,,,1",
        SourceFormat::Grid,
    ),
    county(
        "tainan",
        "臺南市",
        "https://119dts.tncfd.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "kaohsiung",
        "高雄市",
        "https://119dts.fdkc.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "keelung",
        "基隆市",
        "https://klc-dt.klfd.klcg.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "hsinchu-city",
        "新竹市",
        "https://119.hcfd.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "miaoli",
        "苗栗縣",
        "https://119mlfire.mlfd.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "changhua",
        "彰化縣",
        "https://www.chfd.gov.tw/RealInfo/index.aspx?Parser=99,3,29",
        SourceFormat::Grid,
    ),
    county(
        "nantou",
        "南投縣",
        "https://www.ntfd.gov.tw/index.php?act=caselist",
        SourceFormat::Grid,
    ),
    county(
        "yunlin",
        "雲林縣",
        "https://119.ylfire.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "chiayi-city",
        "嘉義市",
        "https://cyfd.chiayi.gov.tw/Common/Getfdcaselist.ashx?mode=Page",
        SourceFormat::Grid,
    ),
    county(
        "chiayi-county",
        "嘉義縣",
        "https://cycfb.cyhg.gov.tw/DisasterPrevent.aspx?n=5F10482409025004&sms=ED4E0CDDC2EA92E6",
        SourceFormat::Grid,
    ),
    county(
        "pingtung",
        "屏東縣",
        "https://ptloc119.ptfire.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "yilan",
        "宜蘭縣",
        "https://nfa119.e-land.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "taitung",
        "臺東縣",
        "https://dt.ttfd.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
    county(
        "penghu",
        "澎湖縣",
        "https://119dts.phfd.gov.tw/DTS/caselist/html",
        SourceFormat::Grid,
    ),
];

pub fn county_list() -> Vec<FireDispatchCountyInfo> {
    COUNTIES
        .iter()
        .map(|county| FireDispatchCountyInfo {
            code: county.code.to_string(),
            name: county.name.to_string(),
            source_url: county.url.to_string(),
        })
        .collect()
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum FireDispatchError {
    #[error("unknown county `{0}`")]
    UnknownCounty(String),
    #[error("fire dispatch source request failed: {0}")]
    Request(Arc<reqwest::Error>),
    #[error("fire dispatch source response is invalid: {0}")]
    Invalid(String),
}

impl From<reqwest::Error> for FireDispatchError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(Arc::new(error))
    }
}

struct Snapshot {
    loaded_at: Instant,
    county: Arc<FireDispatchCounty>,
}

pub struct FireDispatchCache {
    client: reqwest::Client,
    /// One lock per county: concurrent misses share a single source request.
    entries: HashMap<&'static str, Mutex<Option<Snapshot>>>,
}

impl FireDispatchCache {
    pub fn new() -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .add_root_certificate(reqwest::Certificate::from_pem(TWCA_CYBER_ROOT_CA)?)
            .build()?;
        let entries = COUNTIES
            .iter()
            .map(|county| (county.code, Mutex::default()))
            .collect();
        Ok(Self { client, entries })
    }

    /// Cases of one county; failed source requests are not cached.
    pub async fn county(&self, code: &str) -> Result<Arc<FireDispatchCounty>, FireDispatchError> {
        let county = COUNTIES
            .iter()
            .find(|county| county.code == code)
            .ok_or_else(|| FireDispatchError::UnknownCounty(code.to_string()))?;
        let mut entry = self.entries[county.code].lock().await;
        if let Some(snapshot) = entry.as_ref()
            && snapshot.loaded_at.elapsed() < CACHE_TTL
        {
            return Ok(snapshot.county.clone());
        }

        let cases = self.fetch(county).await.inspect_err(|error| {
            tracing::warn!(county = county.code, %error, "fire dispatch refresh failed");
        })?;
        let snapshot = Snapshot {
            loaded_at: Instant::now(),
            county: Arc::new(FireDispatchCounty {
                code: county.code.to_string(),
                name: county.name.to_string(),
                source_url: county.url.to_string(),
                fetched_at: Some(Utc::now()),
                error: None,
                cases,
            }),
        };
        let county = snapshot.county.clone();
        *entry = Some(snapshot);
        Ok(county)
    }

    /// Every county in `COUNTIES` order, fetched concurrently; a failed county carries `error`.
    pub async fn all(self: &Arc<Self>) -> Vec<Arc<FireDispatchCounty>> {
        let mut tasks = JoinSet::new();
        for (index, county) in COUNTIES.iter().enumerate() {
            let cache = self.clone();
            tasks.spawn(async move { (index, cache.county(county.code).await) });
        }
        let mut results: Vec<Option<Arc<FireDispatchCounty>>> = vec![None; COUNTIES.len()];
        while let Some(joined) = tasks.join_next().await {
            let (index, result) = joined.expect("county fetch task panicked");
            let county = &COUNTIES[index];
            results[index] = Some(result.unwrap_or_else(|error| {
                Arc::new(FireDispatchCounty {
                    code: county.code.to_string(),
                    name: county.name.to_string(),
                    source_url: county.url.to_string(),
                    fetched_at: None,
                    error: Some(error.to_string()),
                    cases: Vec::new(),
                })
            }));
        }
        results
            .into_iter()
            .map(|county| county.expect("every task reports"))
            .collect()
    }

    async fn fetch(&self, county: &County) -> Result<Vec<FireDispatchCase>, FireDispatchError> {
        let request = match county.format {
            // The `t` cache-buster mirrors the Taipei page's own request.
            SourceFormat::TaipeiJson => self
                .client
                .post(county.url)
                .form(&[("t", Utc::now().timestamp_millis().to_string())]),
            SourceFormat::Grid | SourceFormat::NewTaipeiJson => self.client.get(county.url),
        };
        let body = request.send().await?.error_for_status()?.text().await?;
        let now = Utc::now();
        let mut cases = match county.format {
            SourceFormat::Grid => parse_grid(&body, now)?,
            SourceFormat::TaipeiJson => parse_taipei(&body, now)?,
            SourceFormat::NewTaipeiJson => parse_new_taipei(&body)?,
        };
        // None sorts first ascending, so descending puts cases without a time last.
        cases.sort_by(|a, b| b.received_at.cmp(&a.received_at));
        Ok(cases)
    }
}

/// Parses an HTML case list whose header row names the columns.
pub fn parse_grid(
    html: &str,
    now: DateTime<Utc>,
) -> Result<Vec<FireDispatchCase>, FireDispatchError> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse("tr, div.tr, ul.rwd-table > li").expect("valid selector");
    let rows: Vec<Vec<String>> = document
        .select(&row_selector)
        .map(row_cells)
        .filter(|cells| !cells.is_empty())
        .collect();

    let header_index = rows
        .iter()
        .position(|cells| cells.iter().any(|cell| is_time_header(&compact(cell))))
        .ok_or_else(|| FireDispatchError::Invalid("case list header not found".to_string()))?;
    let header: Vec<String> = rows[header_index]
        .iter()
        .map(|cell| compact(cell))
        .collect();
    let column = |matches: fn(&str) -> bool| header.iter().position(|cell| matches(cell));
    let time = column(is_time_header).expect("header row has a time column");
    let location = column(|h| h.contains("地點"))
        .ok_or_else(|| FireDispatchError::Invalid("location column not found".to_string()))?;
    let category = column(|h| h.starts_with("案類"));
    let sub_category = column(|h| h == "案別");
    let unit = column(|h| h.contains("派遣分隊"));
    let status = column(|h| h.contains("執行狀況") || h.contains("案件狀態"));
    let case_id = column(|h| h == "編號");

    let mut cases = Vec::new();
    for cells in &rows[header_index + 1..] {
        if cells.len() != header.len() {
            continue;
        }
        let Some(received_at) = parse_local_time(&cells[time]).filter(|t| is_live(*t, now)) else {
            continue;
        };
        let cell = |index: Option<usize>| index.and_then(|i| non_empty(&cells[i]));
        let full_category = match (cell(category), cell(sub_category)) {
            (Some(category), Some(sub)) => format!("{category}-{sub}"),
            (category, sub) => category.or(sub).unwrap_or_default(),
        };
        cases.push(FireDispatchCase {
            received_at: Some(received_at),
            category: full_category,
            location: cells[location].clone(),
            unit: cell(unit),
            status: cell(status),
            case_id: cell(case_id),
            latitude: None,
            longitude: None,
        });
    }
    Ok(cases)
}

#[derive(Deserialize)]
struct TaipeiPayload {
    rows: Vec<TaipeiRow>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaipeiRow {
    in_time: Option<String>,
    cs_kind_name: Option<String>,
    cs_code_name: Option<String>,
    case_status: Option<String>,
    cs_place_fuzzy: Option<String>,
}

pub fn parse_taipei(
    body: &str,
    now: DateTime<Utc>,
) -> Result<Vec<FireDispatchCase>, FireDispatchError> {
    let payload: TaipeiPayload = serde_json::from_str(body).map_err(invalid)?;
    let text = |value: &Option<String>| value.as_deref().and_then(non_empty);
    Ok(payload
        .rows
        .iter()
        .filter_map(|row| {
            let received_at =
                parse_local_time(row.in_time.as_deref()?).filter(|t| is_live(*t, now))?;
            let category = match (text(&row.cs_kind_name), text(&row.cs_code_name)) {
                (Some(kind), Some(code)) => format!("{kind}-{code}"),
                (kind, code) => kind.or(code).unwrap_or_default(),
            };
            Some(FireDispatchCase {
                received_at: Some(received_at),
                category,
                location: text(&row.cs_place_fuzzy).unwrap_or_default(),
                unit: None,
                status: text(&row.case_status),
                case_id: None,
                latitude: None,
                longitude: None,
            })
        })
        .collect())
}

#[derive(Deserialize)]
struct NewTaipeiEnvelope {
    status: u16,
    message: Option<String>,
    /// GeoJSON serialized as a string.
    data: Option<String>,
}

#[derive(Deserialize)]
struct NewTaipeiCollection {
    features: Vec<NewTaipeiFeature>,
}

#[derive(Deserialize)]
struct NewTaipeiFeature {
    properties: NewTaipeiProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewTaipeiProperties {
    feature_id: Option<String>,
    fire_type: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    end_point_info: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    #[serde(default)]
    case_list: Vec<NewTaipeiVehicle>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewTaipeiVehicle {
    start_point_info: Option<String>,
}

/// New Taipei lists only active cases and publishes no receive time.
pub fn parse_new_taipei(body: &str) -> Result<Vec<FireDispatchCase>, FireDispatchError> {
    let envelope: NewTaipeiEnvelope = serde_json::from_str(body).map_err(invalid)?;
    let data = match envelope.data {
        Some(data) if envelope.status == 200 => data,
        _ => {
            return Err(FireDispatchError::Invalid(format!(
                "source status {}: {}",
                envelope.status,
                envelope.message.unwrap_or_default()
            )));
        }
    };
    let collection: NewTaipeiCollection = serde_json::from_str(&data).map_err(invalid)?;
    Ok(collection
        .features
        .into_iter()
        .filter_map(|feature| {
            let props = feature.properties;
            let case_id = props.feature_id.as_deref().and_then(non_empty)?;
            let category = match props.fire_type.as_deref().and_then(non_empty) {
                Some(fire_type) => fire_type,
                None if props.kind.as_deref() == Some("AmbulanceBack") => "救護案件".to_string(),
                None => String::new(),
            };
            let mut vehicles: Vec<String> = Vec::new();
            for vehicle in props
                .case_list
                .iter()
                .filter_map(|v| v.start_point_info.as_deref().and_then(non_empty))
            {
                if !vehicles.contains(&vehicle) {
                    vehicles.push(vehicle);
                }
            }
            let (latitude, longitude) = match (props.lat, props.lng) {
                (Some(lat), Some(lng)) => (Some(lat), Some(lng)),
                _ => (None, None),
            };
            Some(FireDispatchCase {
                received_at: None,
                category,
                location: props
                    .end_point_info
                    .as_deref()
                    .and_then(non_empty)
                    .unwrap_or_default(),
                unit: (!vehicles.is_empty()).then(|| vehicles.join("、")),
                status: None,
                case_id: Some(case_id),
                latitude,
                longitude,
            })
        })
        .collect())
}

fn invalid(error: serde_json::Error) -> FireDispatchError {
    FireDispatchError::Invalid(error.to_string())
}

fn row_cells(row: ElementRef) -> Vec<String> {
    let row_tag = row.value().name();
    row.children()
        .filter_map(ElementRef::wrap)
        .filter(|cell| {
            let element = cell.value();
            match (row_tag, element.name()) {
                ("tr", "th" | "td") | ("li", "span") => true,
                ("div", "div") => element
                    .classes()
                    .any(|class| class == "th" || class == "td"),
                _ => false,
            }
        })
        .map(|cell| {
            let mut text = String::new();
            collect_text(cell, &mut text);
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .collect()
}

/// Cell text without screen-reader labels, inline field titles, map buttons, and dialogs.
fn collect_text(element: ElementRef, out: &mut String) {
    for child in element.children() {
        match child.value() {
            Node::Text(text) => out.push_str(text),
            Node::Element(inner) => {
                let skipped = matches!(inner.name(), "script" | "style" | "button")
                    || inner
                        .classes()
                        .any(|class| matches!(class, "sr-only" | "title" | "modal"));
                if !skipped && let Some(inner) = ElementRef::wrap(child) {
                    collect_text(inner, out);
                }
            }
            _ => {}
        }
    }
}

/// Header text without spaces; some sources write `序 號`.
fn compact(text: &str) -> String {
    text.split_whitespace().collect()
}

fn is_time_header(header: &str) -> bool {
    header.contains("受理") || header.contains("發生時間")
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn is_live(received_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - received_at <= MAX_CASE_AGE && received_at - now <= MAX_CASE_SKEW
}

/// Parses Taipei local time such as `2026/11/25 09:45:11`, `2026-11-25 08:21`, or the
/// ROC-year `115-11-25 09:45:11`.
fn parse_local_time(text: &str) -> Option<DateTime<Utc>> {
    let mut parts = text.split_whitespace();
    let (date, time) = (parts.next()?, parts.next()?);
    let mut fields = date.split(['/', '-']);
    let mut year: i32 = fields.next()?.parse().ok()?;
    let month: u32 = fields.next()?.parse().ok()?;
    let day: u32 = fields.next()?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    if year < 1911 {
        year += 1911;
    }
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let time = NaiveTime::parse_from_str(time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(time, "%H:%M"))
        .ok()?;
    let taipei = FixedOffset::east_opt(8 * 3600).expect("valid offset");
    taipei
        .from_local_datetime(&date.and_time(time))
        .single()
        .map(|local| local.with_timezone(&Utc))
}
