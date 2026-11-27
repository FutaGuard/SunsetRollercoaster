use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::IntoParams;

use crate::error::{ApiError, ApiResult};
use crate::fire_dispatch::{self, COUNTIES, FireDispatchCache};
use crate::models::{FireDispatchCounty, FireDispatchCountyInfo, FireDispatchRecord};

const TZ: &str = "Asia/Taipei";
const RECORD_SELECT: &str = "id, county, received_at, category, location, unit, status, case_id, latitude, longitude, first_seen_at, last_seen_at";

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

    fn county(&self) -> ApiResult<Option<&str>> {
        let Some(county) = self.county.as_deref() else {
            return Ok(None);
        };
        if !COUNTIES.iter().any(|known| known.code == county) {
            return Err(ApiError::BadRequest(format!(
                "unknown county `{county}`; see /fire-dispatch/counties"
            )));
        }
        Ok(Some(county))
    }
}

#[utoipa::path(
    get,
    path = "/fire-dispatch/records",
    tag = "fire-dispatch",
    params(FireDispatchRecordListParams),
    responses(
        (status = 200, description = "Stored case states, newest first_seen_at first. The crawler stores `/fire-dispatch/latest` every minute; each status or unit change of a case is a separate record.", body = [FireDispatchRecord]),
        (status = 400, description = "Unknown county code", body = crate::error::ErrorBody)
    )
)]
pub async fn list_records(
    State(pool): State<PgPool>,
    Query(params): Query<FireDispatchRecordListParams>,
) -> ApiResult<Json<Vec<FireDispatchRecord>>> {
    let county = params.county()?;
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
    sql.push_str(&format!(
        " ORDER BY first_seen_at DESC, id DESC LIMIT ${index} OFFSET ${}",
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
    let rows = query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(&pool)
        .await?;
    Ok(Json(rows))
}

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
        .merge(
            Router::new()
                .route("/fire-dispatch/counties", get(list_counties))
                .route("/fire-dispatch/latest", get(latest_all))
                .route("/fire-dispatch/latest/{county}", get(latest_county))
                .with_state(cache),
        )
}
