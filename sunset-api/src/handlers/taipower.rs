use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::IntoParams;

use crate::error::{ApiError, ApiResult};
use crate::models::{
    TaipowerAreaLoad, TaipowerAreaSnapshot, TaipowerFuelMix, TaipowerGenerator,
    TaipowerOperatingReserve, TaipowerPowerSnapshot,
};

const TZ: &str = "Asia/Taipei";
const POWER_SELECT: &str = "id, published_at, current_load_mw, current_utilization_percent, forecast_max_supply_mw, forecast_peak_demand_mw, forecast_peak_reserve_mw, forecast_peak_reserve_rate_percent, forecast_peak_reserve_indicator, forecast_peak_hour_range, yesterday_date, yesterday_max_supply_mw, yesterday_peak_demand_mw, yesterday_peak_reserve_mw, yesterday_peak_reserve_rate_percent, yesterday_peak_reserve_indicator, realtime_max_supply_mw, realtime_peak_at";
const FUEL_SELECT: &str = "id, observed_at, lng_mw, ipp_lng_mw, coal_mw, ipp_coal_mw, cogeneration_mw, fuel_oil_mw, solar_mw, wind_mw, hydro_mw, energy_storage_mw, other_renewable_mw, energy_storage_load_mw, total_mw";
const AREA_LOAD_SELECT: &str =
    "id, observed_at, north_load_mw, central_load_mw, south_load_mw, east_load_mw, total_load_mw";
const AREA_SNAPSHOT_SELECT: &str = "id, observed_at, north_generation_mw, north_load_mw, central_generation_mw, central_load_mw, south_generation_mw, south_load_mw, east_generation_mw, east_load_mw";
const RESERVE_SELECT: &str =
    "id, date, peak_load_mw, reserve_capacity_mw, reserve_rate_percent, is_forecast, published_at";
const GENERATOR_SELECT: &str = "id, published_at, sequence, category_code, category, subcategory, unit_name, installed_capacity_mw, installed_capacity_percent, net_generation_mw, net_generation_percent, utilization_percent, status, is_summary";

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TimeListParams {
    /// Exact local date in Asia/Taipei (YYYY-MM-DD).
    pub date: Option<NaiveDate>,
    /// Inclusive local date lower bound in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub start: Option<NaiveDate>,
    /// Inclusive local date upper bound in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub end: Option<NaiveDate>,
    #[param(default = 500, minimum = 1, maximum = 5000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl TimeListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(500).clamp(1, 5000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

fn append_time_filters(sql: &mut String, params: &TimeListParams, column: &str) -> usize {
    let mut index = 1;
    if params.date.is_some() {
        sql.push_str(&format!(
            " AND ({column} AT TIME ZONE '{TZ}')::date = ${index}"
        ));
        index += 1;
    } else {
        if params.start.is_some() {
            sql.push_str(&format!(
                " AND ({column} AT TIME ZONE '{TZ}')::date >= ${index}"
            ));
            index += 1;
        }
        if params.end.is_some() {
            sql.push_str(&format!(
                " AND ({column} AT TIME ZONE '{TZ}')::date <= ${index}"
            ));
            index += 1;
        }
    }
    index
}

#[utoipa::path(
    get,
    path = "/taipower/power-snapshots",
    tag = "taipower",
    params(TimeListParams),
    responses(
        (status = 200, description = "List Taipower current-load and peak forecast snapshots; power values are MW", body = [TaipowerPowerSnapshot])
    )
)]
pub async fn list_power_snapshots(
    State(pool): State<PgPool>,
    Query(params): Query<TimeListParams>,
) -> ApiResult<Json<Vec<TaipowerPowerSnapshot>>> {
    let mut sql = format!("SELECT {POWER_SELECT} FROM taipower_power_snapshot WHERE 1=1");
    let index = append_time_filters(&mut sql, &params, "published_at");
    sql.push_str(&format!(
        " ORDER BY published_at DESC, id DESC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, TaipowerPowerSnapshot>(&sql);
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
    path = "/taipower/power-snapshots/latest",
    tag = "taipower",
    responses(
        (status = 200, description = "Latest Taipower current-load and peak forecast snapshot", body = TaipowerPowerSnapshot),
        (status = 404, description = "No data", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_power_snapshot(
    State(pool): State<PgPool>,
) -> ApiResult<Json<TaipowerPowerSnapshot>> {
    let sql = format!(
        "SELECT {POWER_SELECT} FROM taipower_power_snapshot ORDER BY published_at DESC, id DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, TaipowerPowerSnapshot>(&sql)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/taipower/fuel-mix",
    tag = "taipower",
    params(TimeListParams),
    responses(
        (status = 200, description = "List ten-minute fuel-mix curve records; power values are MW", body = [TaipowerFuelMix])
    )
)]
pub async fn list_fuel_mix(
    State(pool): State<PgPool>,
    Query(params): Query<TimeListParams>,
) -> ApiResult<Json<Vec<TaipowerFuelMix>>> {
    let mut sql = format!("SELECT {FUEL_SELECT} FROM taipower_fuel_mix WHERE 1=1");
    let index = append_time_filters(&mut sql, &params, "observed_at");
    sql.push_str(&format!(
        " ORDER BY observed_at DESC, id DESC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, TaipowerFuelMix>(&sql);
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
    path = "/taipower/fuel-mix/latest",
    tag = "taipower",
    responses(
        (status = 200, description = "Latest ten-minute fuel-mix record", body = TaipowerFuelMix),
        (status = 404, description = "No data", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_fuel_mix(State(pool): State<PgPool>) -> ApiResult<Json<TaipowerFuelMix>> {
    let sql = format!(
        "SELECT {FUEL_SELECT} FROM taipower_fuel_mix ORDER BY observed_at DESC, id DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, TaipowerFuelMix>(&sql)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/taipower/area-loads",
    tag = "taipower",
    params(TimeListParams),
    responses(
        (status = 200, description = "List ten-minute regional load curve records; power values are MW", body = [TaipowerAreaLoad])
    )
)]
pub async fn list_area_loads(
    State(pool): State<PgPool>,
    Query(params): Query<TimeListParams>,
) -> ApiResult<Json<Vec<TaipowerAreaLoad>>> {
    let mut sql = format!("SELECT {AREA_LOAD_SELECT} FROM taipower_area_load WHERE 1=1");
    let index = append_time_filters(&mut sql, &params, "observed_at");
    sql.push_str(&format!(
        " ORDER BY observed_at DESC, id DESC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, TaipowerAreaLoad>(&sql);
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
    path = "/taipower/area-loads/latest",
    tag = "taipower",
    responses(
        (status = 200, description = "Latest ten-minute regional load record", body = TaipowerAreaLoad),
        (status = 404, description = "No data", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_area_load(State(pool): State<PgPool>) -> ApiResult<Json<TaipowerAreaLoad>> {
    let sql = format!(
        "SELECT {AREA_LOAD_SELECT} FROM taipower_area_load ORDER BY observed_at DESC, id DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, TaipowerAreaLoad>(&sql)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/taipower/area-snapshots",
    tag = "taipower",
    params(TimeListParams),
    responses(
        (status = 200, description = "List regional generation and load snapshots; power values are MW", body = [TaipowerAreaSnapshot])
    )
)]
pub async fn list_area_snapshots(
    State(pool): State<PgPool>,
    Query(params): Query<TimeListParams>,
) -> ApiResult<Json<Vec<TaipowerAreaSnapshot>>> {
    let mut sql = format!("SELECT {AREA_SNAPSHOT_SELECT} FROM taipower_area_snapshot WHERE 1=1");
    let index = append_time_filters(&mut sql, &params, "observed_at");
    sql.push_str(&format!(
        " ORDER BY observed_at DESC, id DESC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, TaipowerAreaSnapshot>(&sql);
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
    path = "/taipower/area-snapshots/latest",
    tag = "taipower",
    responses(
        (status = 200, description = "Latest regional generation and load snapshot", body = TaipowerAreaSnapshot),
        (status = 404, description = "No data", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_area_snapshot(
    State(pool): State<PgPool>,
) -> ApiResult<Json<TaipowerAreaSnapshot>> {
    let sql = format!(
        "SELECT {AREA_SNAPSHOT_SELECT} FROM taipower_area_snapshot ORDER BY observed_at DESC, id DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, TaipowerAreaSnapshot>(&sql)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ReserveListParams {
    pub date: Option<NaiveDate>,
    /// Inclusive date lower bound. Ignored when `date` is set.
    pub start: Option<NaiveDate>,
    /// Inclusive date upper bound. Ignored when `date` is set.
    pub end: Option<NaiveDate>,
    #[param(default = 500, minimum = 1, maximum = 5000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl ReserveListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(500).clamp(1, 5000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[utoipa::path(
    get,
    path = "/taipower/operating-reserves",
    tag = "taipower",
    params(ReserveListParams),
    responses(
        (status = 200, description = "List daily peak load and operating reserve records; power values are MW", body = [TaipowerOperatingReserve])
    )
)]
pub async fn list_operating_reserves(
    State(pool): State<PgPool>,
    Query(params): Query<ReserveListParams>,
) -> ApiResult<Json<Vec<TaipowerOperatingReserve>>> {
    let mut sql = format!("SELECT {RESERVE_SELECT} FROM taipower_operating_reserve WHERE 1=1");
    let mut index = 1;
    if params.date.is_some() {
        sql.push_str(&format!(" AND date = ${index}"));
        index += 1;
    } else {
        if params.start.is_some() {
            sql.push_str(&format!(" AND date >= ${index}"));
            index += 1;
        }
        if params.end.is_some() {
            sql.push_str(&format!(" AND date <= ${index}"));
            index += 1;
        }
    }
    sql.push_str(&format!(
        " ORDER BY date DESC, id DESC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, TaipowerOperatingReserve>(&sql);
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
    path = "/taipower/operating-reserves/latest",
    tag = "taipower",
    responses(
        (status = 200, description = "Latest operating reserve record (normally today's forecast)", body = TaipowerOperatingReserve),
        (status = 404, description = "No data", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_operating_reserve(
    State(pool): State<PgPool>,
) -> ApiResult<Json<TaipowerOperatingReserve>> {
    let sql = format!(
        "SELECT {RESERVE_SELECT} FROM taipower_operating_reserve ORDER BY date DESC, id DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, TaipowerOperatingReserve>(&sql)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GeneratorListParams {
    /// Exact snapshot date in Asia/Taipei (YYYY-MM-DD).
    pub date: Option<NaiveDate>,
    /// Inclusive local date lower bound in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub start: Option<NaiveDate>,
    /// Inclusive local date upper bound in Asia/Taipei (YYYY-MM-DD). Ignored when `date` is set.
    pub end: Option<NaiveDate>,
    /// Exact source category code, for example `lng`, `coal`, or `solar`.
    pub category_code: Option<String>,
    /// Exact generator or subtotal name.
    pub unit_name: Option<String>,
    /// Filter regular unit rows (`false`) or category subtotal rows (`true`).
    pub is_summary: Option<bool>,
    #[param(default = 500, minimum = 1, maximum = 5000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl GeneratorListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(500).clamp(1, 5000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[utoipa::path(
    get,
    path = "/taipower/generators",
    tag = "taipower",
    params(GeneratorListParams),
    responses(
        (status = 200, description = "List generator output snapshot rows; power values are MW", body = [TaipowerGenerator])
    )
)]
pub async fn list_generators(
    State(pool): State<PgPool>,
    Query(params): Query<GeneratorListParams>,
) -> ApiResult<Json<Vec<TaipowerGenerator>>> {
    let mut sql = format!("SELECT {GENERATOR_SELECT} FROM taipower_generator WHERE 1=1");
    let mut index = 1;
    if params.date.is_some() {
        sql.push_str(&format!(
            " AND (published_at AT TIME ZONE '{TZ}')::date = ${index}"
        ));
        index += 1;
    } else {
        if params.start.is_some() {
            sql.push_str(&format!(
                " AND (published_at AT TIME ZONE '{TZ}')::date >= ${index}"
            ));
            index += 1;
        }
        if params.end.is_some() {
            sql.push_str(&format!(
                " AND (published_at AT TIME ZONE '{TZ}')::date <= ${index}"
            ));
            index += 1;
        }
    }
    if params.category_code.is_some() {
        sql.push_str(&format!(" AND category_code = ${index}"));
        index += 1;
    }
    if params.unit_name.is_some() {
        sql.push_str(&format!(" AND unit_name = ${index}"));
        index += 1;
    }
    if params.is_summary.is_some() {
        sql.push_str(&format!(" AND is_summary = ${index}"));
        index += 1;
    }
    sql.push_str(&format!(
        " ORDER BY published_at DESC, sequence ASC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, TaipowerGenerator>(&sql);
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
    if let Some(category_code) = &params.category_code {
        query = query.bind(category_code);
    }
    if let Some(unit_name) = &params.unit_name {
        query = query.bind(unit_name);
    }
    if let Some(is_summary) = params.is_summary {
        query = query.bind(is_summary);
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
    path = "/taipower/generators/latest",
    tag = "taipower",
    responses(
        (status = 200, description = "All unit and subtotal rows from the latest generator snapshot", body = [TaipowerGenerator])
    )
)]
pub async fn latest_generators(
    State(pool): State<PgPool>,
) -> ApiResult<Json<Vec<TaipowerGenerator>>> {
    let sql = format!(
        "SELECT {GENERATOR_SELECT} FROM taipower_generator \
         WHERE published_at = (SELECT MAX(published_at) FROM taipower_generator) \
         ORDER BY sequence ASC"
    );
    let rows = sqlx::query_as::<_, TaipowerGenerator>(&sql)
        .fetch_all(&pool)
        .await?;
    Ok(Json(rows))
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/taipower/power-snapshots", get(list_power_snapshots))
        .route(
            "/taipower/power-snapshots/latest",
            get(latest_power_snapshot),
        )
        .route("/taipower/fuel-mix", get(list_fuel_mix))
        .route("/taipower/fuel-mix/latest", get(latest_fuel_mix))
        .route("/taipower/area-loads", get(list_area_loads))
        .route("/taipower/area-loads/latest", get(latest_area_load))
        .route("/taipower/area-snapshots", get(list_area_snapshots))
        .route("/taipower/area-snapshots/latest", get(latest_area_snapshot))
        .route("/taipower/operating-reserves", get(list_operating_reserves))
        .route(
            "/taipower/operating-reserves/latest",
            get(latest_operating_reserve),
        )
        .route("/taipower/generators", get(list_generators))
        .route("/taipower/generators/latest", get(latest_generators))
}
