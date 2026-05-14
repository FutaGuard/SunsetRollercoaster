use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::IntoParams;

use crate::error::{ApiError, ApiResult};
use crate::models::Reservoir;

const SELECT_COLS: &str = r#"id, name, capavailable, "statisticTimeS", "statisticTimeE", "rainFall", "inFlow", "outFlow", waterlevediff, "recordTime", caplevel, currcap, currcapper"#;

const TZ: &str = "Asia/Taipei";

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ReservoirListParams {
    /// Reservoir name (exact match). Combine with `date` to get one reservoir's full day.
    pub name: Option<String>,
    /// Local date (Asia/Taipei, YYYY-MM-DD) to filter `recordTime` by.
    pub date: Option<NaiveDate>,
    /// Hour of day in 0..=23 (Asia/Taipei). Requires `date`.
    /// Combine with `date` (without `name`) to get all reservoirs at one hourly snapshot.
    #[param(minimum = 0, maximum = 23)]
    pub hour: Option<i32>,
    /// Inclusive lower bound on `recordTime` (RFC3339).
    pub start: Option<DateTime<Utc>>,
    /// Inclusive upper bound on `recordTime` (RFC3339).
    pub end: Option<DateTime<Utc>>,
    #[param(default = 200, minimum = 1, maximum = 5000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl ReservoirListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(200).clamp(1, 5000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[utoipa::path(
    get,
    path = "/reservoirs",
    tag = "reservoirs",
    params(ReservoirListParams),
    responses(
        (status = 200, description = "Reservoir records filtered by name / date / hour / time range", body = [Reservoir]),
        (status = 400, description = "Invalid query parameters", body = crate::error::ErrorBody)
    )
)]
pub async fn list_reservoirs(
    State(pool): State<PgPool>,
    Query(params): Query<ReservoirListParams>,
) -> ApiResult<Json<Vec<Reservoir>>> {
    if params.hour.is_some() && params.date.is_none() {
        return Err(ApiError::BadRequest(
            "`hour` requires `date`".to_string(),
        ));
    }
    if let Some(h) = params.hour {
        if !(0..=23).contains(&h) {
            return Err(ApiError::BadRequest(
                "`hour` must be between 0 and 23".to_string(),
            ));
        }
    }

    let mut sql = format!("SELECT {SELECT_COLS} FROM reservoir WHERE 1=1");
    let mut idx: usize = 1;

    if params.name.is_some() {
        sql.push_str(&format!(" AND name = ${idx}"));
        idx += 1;
    }
    if params.date.is_some() {
        sql.push_str(&format!(
            r#" AND ("recordTime" AT TIME ZONE '{TZ}')::date = ${idx}"#
        ));
        idx += 1;
    }
    if params.hour.is_some() {
        sql.push_str(&format!(
            r#" AND EXTRACT(HOUR FROM "recordTime" AT TIME ZONE '{TZ}')::int = ${idx}"#
        ));
        idx += 1;
    }
    if params.start.is_some() {
        sql.push_str(&format!(r#" AND "recordTime" >= ${idx}"#));
        idx += 1;
    }
    if params.end.is_some() {
        sql.push_str(&format!(r#" AND "recordTime" <= ${idx}"#));
        idx += 1;
    }

    sql.push_str(&format!(
        r#" ORDER BY "recordTime" ASC NULLS LAST, name ASC, id ASC LIMIT ${} OFFSET ${}"#,
        idx,
        idx + 1
    ));

    let mut q = sqlx::query_as::<_, Reservoir>(&sql);
    if let Some(n) = &params.name {
        q = q.bind(n);
    }
    if let Some(d) = params.date {
        q = q.bind(d);
    }
    if let Some(h) = params.hour {
        q = q.bind(h);
    }
    if let Some(s) = params.start {
        q = q.bind(s);
    }
    if let Some(e) = params.end {
        q = q.bind(e);
    }
    let rows = q
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(&pool)
        .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/reservoirs/names",
    tag = "reservoirs",
    responses(
        (status = 200, description = "Distinct reservoir names", body = [String])
    )
)]
pub async fn list_reservoir_names(State(pool): State<PgPool>) -> ApiResult<Json<Vec<String>>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT name FROM reservoir WHERE name IS NOT NULL ORDER BY name ASC",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(rows.into_iter().map(|(n,)| n).collect()))
}

#[utoipa::path(
    get,
    path = "/reservoirs/latest",
    tag = "reservoirs",
    responses(
        (status = 200, description = "Latest record for each reservoir name (dashboard snapshot)", body = [Reservoir])
    )
)]
pub async fn latest_reservoirs(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Reservoir>>> {
    let sql = format!(
        r#"SELECT {SELECT_COLS} FROM (
              SELECT *, ROW_NUMBER() OVER (PARTITION BY name ORDER BY "recordTime" DESC NULLS LAST, id DESC) AS rn
              FROM reservoir
              WHERE name IS NOT NULL
           ) r WHERE rn = 1 ORDER BY name ASC"#
    );
    let rows = sqlx::query_as::<_, Reservoir>(&sql).fetch_all(&pool).await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/reservoirs/{id}",
    tag = "reservoirs",
    params(("id" = i32, Path, description = "Reservoir record id")),
    responses(
        (status = 200, description = "Reservoir record detail", body = Reservoir),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub async fn get_reservoir(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Reservoir>> {
    let sql = format!("SELECT {SELECT_COLS} FROM reservoir WHERE id = $1");
    let row = sqlx::query_as::<_, Reservoir>(&sql)
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/reservoirs", get(list_reservoirs))
        .route("/reservoirs/names", get(list_reservoir_names))
        .route("/reservoirs/latest", get(latest_reservoirs))
        .route("/reservoirs/{id}", get(get_reservoir))
}
