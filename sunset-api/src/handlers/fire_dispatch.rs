use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, FixedOffset, NaiveDate, TimeDelta, TimeZone, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::IntoParams;

use crate::error::{ApiError, ApiResult};
use crate::fire_dispatch::{self, COUNTIES, FireDispatchCache};
use crate::models::{
    FireDispatchCounty, FireDispatchCountyInfo, FireDispatchRecord, FireDispatchStoredCase,
};

const TZ: &str = "Asia/Taipei";
const RECORD_SELECT: &str = "id, county, received_at, category, location, unit, status, case_id, latitude, longitude, first_seen_at, last_seen_at";
/// A case stays in the live lists for hours, never a full day; cases are grouped from rows
/// within this margin of the requested window so no case loses its earlier or later states.
const CASE_SPAN: TimeDelta = TimeDelta::days(1);
/// Window of `/fire-dispatch/cases` when no date, range, or `since` is given.
const DEFAULT_CASE_WINDOW: TimeDelta = TimeDelta::hours(24);

fn validate_county(county: Option<&str>) -> ApiResult<Option<&str>> {
    let Some(county) = county else {
        return Ok(None);
    };
    if !COUNTIES.iter().any(|known| known.code == county) {
        return Err(ApiError::BadRequest(format!(
            "unknown county `{county}`; see /fire-dispatch/counties"
        )));
    }
    Ok(Some(county))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FireDispatchRecordListParams {
    /// County code from `/fire-dispatch/counties` (for example, `tainan`).
    pub county: Option<String>,
    /// Exact local date of `first_seen_at` in Asia/Taipei (YYYY-MM-DD).
    pub date: Option<NaiveDate>,
    /// Inclusive local date lower bound on `first_seen_at` in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub start: Option<NaiveDate>,
    /// Inclusive local date upper bound on `first_seen_at` in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub end: Option<NaiveDate>,
    /// RFC 3339 time (for example, `2026-11-27T12:00:00Z`): only records added or updated at
    /// or after it (`last_seen_at >= since`), oldest `last_seen_at` first. Inclusive, so pass the
    /// largest `last_seen_at` already received and drop repeats by `id`.
    pub since: Option<DateTime<Utc>>,
    #[param(default = 500, minimum = 1, maximum = 5000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl FireDispatchRecordListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(500).clamp(1, 5000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/// Stored case states
///
/// Every status or dispatched-unit change of a case is its own record. For incremental
/// polling, pass the largest `last_seen_at` received as `since` and drop repeated `id`s.
#[utoipa::path(
    get,
    path = "/fire-dispatch/records",
    tag = "fire-dispatch",
    params(FireDispatchRecordListParams),
    responses(
        (status = 200, description = "Stored case states, newest first_seen_at first (oldest last_seen_at first with `since`). The crawler stores `/fire-dispatch/latest` every 30 seconds; each status or unit change of a case is a separate record.", body = [FireDispatchRecord]),
        (status = 400, description = "Unknown county code", body = crate::error::ErrorBody)
    )
)]
pub async fn list_records(
    State(pool): State<PgPool>,
    Query(params): Query<FireDispatchRecordListParams>,
) -> ApiResult<Json<Vec<FireDispatchRecord>>> {
    let county = validate_county(params.county.as_deref())?;
    let mut sql = format!("SELECT {RECORD_SELECT} FROM fire_dispatch_record WHERE 1=1");
    let mut index = 1;
    if county.is_some() {
        sql.push_str(&format!(" AND county = ${index}"));
        index += 1;
    }
    if params.date.is_some() {
        sql.push_str(&format!(
            " AND (first_seen_at AT TIME ZONE '{TZ}')::date = ${index}"
        ));
        index += 1;
    } else {
        if params.start.is_some() {
            sql.push_str(&format!(
                " AND (first_seen_at AT TIME ZONE '{TZ}')::date >= ${index}"
            ));
            index += 1;
        }
        if params.end.is_some() {
            sql.push_str(&format!(
                " AND (first_seen_at AT TIME ZONE '{TZ}')::date <= ${index}"
            ));
            index += 1;
        }
    }
    if params.since.is_some() {
        sql.push_str(&format!(" AND last_seen_at >= ${index}"));
        index += 1;
    }
    let order = if params.since.is_some() {
        "last_seen_at ASC, id ASC"
    } else {
        "first_seen_at DESC, id DESC"
    };
    sql.push_str(&format!(
        " ORDER BY {order} LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, FireDispatchRecord>(&sql);
    if let Some(county) = county {
        query = query.bind(county);
    }
    if let Some(date) = params.date {
        query = query.bind(date);
    } else {
        if let Some(start) = params.start {
            query = query.bind(start);
        }
        if let Some(end) = params.end {
            query = query.bind(end);
        }
    }
    if let Some(since) = params.since {
        query = query.bind(since);
    }
    let rows = query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(&pool)
        .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FireDispatchCaseListParams {
    /// County code from `/fire-dispatch/counties` (for example, `tainan`).
    pub county: Option<String>,
    /// Exact local date of the case's `first_seen_at` in Asia/Taipei (YYYY-MM-DD).
    pub date: Option<NaiveDate>,
    /// Inclusive local date lower bound on the case's `first_seen_at` in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub start: Option<NaiveDate>,
    /// Inclusive local date upper bound on the case's `first_seen_at` in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub end: Option<NaiveDate>,
    /// RFC 3339 time: only cases with a state added or updated at or after it
    /// (`last_seen_at >= since`), oldest `last_seen_at` first. Without `date`, `start`, `end`,
    /// or `since`, only cases first seen in the last 24 hours are listed.
    pub since: Option<DateTime<Utc>>,
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl FireDispatchCaseListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(100).clamp(1, 1000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/// Start of a Taipei local date as UTC.
fn taipei_midnight(date: NaiveDate) -> DateTime<Utc> {
    let taipei = FixedOffset::east_opt(8 * 3600).expect("valid offset");
    taipei
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("valid time"))
        .single()
        .expect("fixed offset has no gaps")
        .with_timezone(&Utc)
}

/// Groups records into cases: by case number where the source publishes one, otherwise by
/// county, receive time, location, and category. The latest-seen state supplies the current
/// fields; `history` lists every state in the order first seen.
const CASE_SELECT: &str = r#"
SELECT
    county,
    (array_agg(received_at ORDER BY last_seen_at DESC, id DESC))[1] AS received_at,
    (array_agg(category ORDER BY last_seen_at DESC, id DESC))[1] AS category,
    (array_agg(location ORDER BY last_seen_at DESC, id DESC))[1] AS location,
    case_id,
    (array_agg(status ORDER BY last_seen_at DESC, id DESC))[1] AS status,
    (array_agg(unit ORDER BY last_seen_at DESC, id DESC))[1] AS unit,
    (array_agg(latitude ORDER BY last_seen_at DESC, id DESC))[1] AS latitude,
    (array_agg(longitude ORDER BY last_seen_at DESC, id DESC))[1] AS longitude,
    MIN(first_seen_at) AS first_seen_at,
    MAX(last_seen_at) AS last_seen_at,
    json_agg(json_build_object(
        'id', id,
        'status', status,
        'unit', unit,
        'first_seen_at', first_seen_at,
        'last_seen_at', last_seen_at
    ) ORDER BY first_seen_at, id) AS history
FROM fire_dispatch_record"#;
const CASE_GROUP_BY: &str = "GROUP BY county, case_id, \
    CASE WHEN case_id IS NULL THEN received_at END, \
    CASE WHEN case_id IS NULL THEN location END, \
    CASE WHEN case_id IS NULL THEN category END";

/// Stored cases with status history
///
/// Records merged into cases. Poll with `since` and replace each returned case by its key
/// (`county` + `case_id`, or `county` + `received_at` + `location` + `category`).
#[utoipa::path(
    get,
    path = "/fire-dispatch/cases",
    tag = "fire-dispatch",
    params(FireDispatchCaseListParams),
    responses(
        (status = 200, description = "Stored records merged into cases, newest first_seen_at first (oldest last_seen_at first with `since`). Most counties publish no case number, so records sharing county, receive time, location, and category count as one case (simultaneous dispatches of several stations included); Tainan and New Taipei use their case numbers.", body = [FireDispatchStoredCase]),
        (status = 400, description = "Unknown county code", body = crate::error::ErrorBody)
    )
)]
pub async fn list_cases(
    State(pool): State<PgPool>,
    Query(params): Query<FireDispatchCaseListParams>,
) -> ApiResult<Json<Vec<FireDispatchStoredCase>>> {
    let county = validate_county(params.county.as_deref())?;

    // Case-level bounds on first_seen_at.
    let (first_from, first_until) = match (params.date, params.start, params.end) {
        (Some(date), _, _) => (
            Some(taipei_midnight(date)),
            Some(taipei_midnight(date) + TimeDelta::days(1)),
        ),
        (None, start, end) => (
            start.map(taipei_midnight),
            end.map(|end| taipei_midnight(end) + TimeDelta::days(1)),
        ),
    };
    let first_from = match (first_from, first_until, params.since) {
        (None, None, None) => Some(Utc::now() - DEFAULT_CASE_WINDOW),
        _ => first_from,
    };
    // Row-level bounds wide enough to keep every state of the cases in range.
    let rows_from = [
        first_from.map(|t| t - CASE_SPAN),
        params.since.map(|t| t - CASE_SPAN),
    ]
    .into_iter()
    .flatten()
    .max();
    let rows_until = first_until.map(|t| t + CASE_SPAN);

    let mut binds: Vec<DateTime<Utc>> = Vec::new();
    let mut row_filter = String::from(" WHERE 1=1");
    let mut index = 1;
    if county.is_some() {
        row_filter.push_str(&format!(" AND county = ${index}"));
        index += 1;
    }
    for (bound, op) in [(rows_from, ">="), (rows_until, "<")] {
        if let Some(bound) = bound {
            row_filter.push_str(&format!(" AND first_seen_at {op} ${index}"));
            binds.push(bound);
            index += 1;
        }
    }
    let mut case_filter = String::from(" WHERE 1=1");
    for (bound, condition) in [
        (first_from, "first_seen_at >="),
        (first_until, "first_seen_at <"),
        (params.since, "last_seen_at >="),
    ] {
        if let Some(bound) = bound {
            case_filter.push_str(&format!(" AND {condition} ${index}"));
            binds.push(bound);
            index += 1;
        }
    }
    let order = if params.since.is_some() {
        "last_seen_at ASC, first_seen_at ASC"
    } else {
        "first_seen_at DESC, last_seen_at DESC"
    };
    let sql = format!(
        "SELECT * FROM ({CASE_SELECT}{row_filter} {CASE_GROUP_BY}) cases{case_filter} \
         ORDER BY {order} LIMIT ${index} OFFSET ${}",
        index + 1
    );

    let mut query = sqlx::query_as::<_, FireDispatchStoredCase>(&sql);
    if let Some(county) = county {
        query = query.bind(county);
    }
    for bound in binds {
        query = query.bind(bound);
    }
    let cases = query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(&pool)
        .await?;
    Ok(Json(cases))
}

/// Supported counties
#[utoipa::path(
    get,
    path = "/fire-dispatch/counties",
    tag = "fire-dispatch",
    responses(
        (status = 200, description = "Supported counties and the case list each is read from", body = [FireDispatchCountyInfo])
    )
)]
pub async fn list_counties() -> Json<Vec<FireDispatchCountyInfo>> {
    Json(fire_dispatch::county_list())
}

/// Live cases of every county
#[utoipa::path(
    get,
    path = "/fire-dispatch/latest",
    tag = "fire-dispatch",
    responses(
        (status = 200, description = "Live cases grouped by county, fetched concurrently and cached 60 seconds per county. A county whose source failed has `error` set and no cases.", body = [FireDispatchCounty])
    )
)]
pub async fn latest_all(
    State(cache): State<Arc<FireDispatchCache>>,
) -> Json<Vec<Arc<FireDispatchCounty>>> {
    Json(cache.all().await)
}

/// Live cases of one county
#[utoipa::path(
    get,
    path = "/fire-dispatch/latest/{county}",
    tag = "fire-dispatch",
    params(("county" = String, Path, description = "County code from `/fire-dispatch/counties`, for example `tainan`")),
    responses(
        (status = 200, description = "Live cases of one county, cached 60 seconds", body = FireDispatchCounty),
        (status = 404, description = "Unknown county code", body = crate::error::ErrorBody),
        (status = 502, description = "The county's source failed or returned an unrecognized page", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_county(
    State(cache): State<Arc<FireDispatchCache>>,
    Path(county): Path<String>,
) -> ApiResult<Json<Arc<FireDispatchCounty>>> {
    Ok(Json(cache.county(&county).await?))
}

pub fn router(cache: Arc<FireDispatchCache>) -> Router<PgPool> {
    Router::new()
        .route("/fire-dispatch/records", get(list_records))
        .route("/fire-dispatch/cases", get(list_cases))
        .merge(
            Router::new()
                .route("/fire-dispatch/counties", get(list_counties))
                .route("/fire-dispatch/latest", get(latest_all))
                .route("/fire-dispatch/latest/{county}", get(latest_county))
                .with_state(cache),
        )
}
