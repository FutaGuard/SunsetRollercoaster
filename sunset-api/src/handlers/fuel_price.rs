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
use crate::handlers::Pagination;
use crate::models::NationwideFuelPrice;

const SELECT_COLS: &str = "id, period_start, period_end, unleaded_92, unleaded_95, unleaded_98, super_diesel, west_texas, dubai, brent";
const TZ: &str = "Asia/Taipei";

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FuelPriceListParams {
    /// Exact period start date in Asia/Taipei (YYYY-MM-DD).
    pub date: Option<NaiveDate>,
    /// Inclusive lower bound on the period start date in Asia/Taipei (YYYY-MM-DD).
    pub start: Option<NaiveDate>,
    /// Inclusive upper bound on the period start date in Asia/Taipei (YYYY-MM-DD).
    pub end: Option<NaiveDate>,
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl FuelPriceListParams {
    fn pagination(&self) -> Pagination {
        Pagination {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

#[utoipa::path(
    get,
    path = "/fuel-prices",
    tag = "fuel-prices",
    params(FuelPriceListParams),
    responses(
        (status = 200, description = "List weekly Taiwan fuel and international crude oil prices filtered by period start date", body = [NationwideFuelPrice])
    )
)]
pub async fn list_fuel_prices(
    State(pool): State<PgPool>,
    Query(params): Query<FuelPriceListParams>,
) -> ApiResult<Json<Vec<NationwideFuelPrice>>> {
    let page = params.pagination();
    let mut sql = format!("SELECT {SELECT_COLS} FROM nationwide_fuel_price WHERE 1=1");

    if params.date.is_some() {
        sql.push_str(&format!(
            r#" AND (period_start AT TIME ZONE '{TZ}')::date = $1"#
        ));
    } else {
        if params.start.is_some() {
            sql.push_str(&format!(
                r#" AND (period_start AT TIME ZONE '{TZ}')::date >= $1"#
            ));
        }
        if params.end.is_some() {
            let index = if params.start.is_some() { 2 } else { 1 };
            sql.push_str(&format!(
                r#" AND (period_start AT TIME ZONE '{TZ}')::date <= ${index}"#
            ));
        }
    }

    let next_index = 1
        + params.date.is_some() as usize
        + (params.date.is_none() && params.start.is_some()) as usize
        + (params.date.is_none() && params.end.is_some()) as usize;
    sql.push_str(&format!(
        " ORDER BY period_start DESC, id DESC LIMIT ${next_index} OFFSET ${}",
        next_index + 1
    ));

    let mut query = sqlx::query_as::<_, NationwideFuelPrice>(&sql);
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
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&pool)
        .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/fuel-prices/latest",
    tag = "fuel-prices",
    responses(
        (status = 200, description = "Most recent weekly Taiwan fuel and international crude oil prices", body = NationwideFuelPrice),
        (status = 404, description = "No data", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_fuel_price(State(pool): State<PgPool>) -> ApiResult<Json<NationwideFuelPrice>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM nationwide_fuel_price ORDER BY period_start DESC, id DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, NationwideFuelPrice>(&sql)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/fuel-prices/{id}",
    tag = "fuel-prices",
    params(("id" = i32, Path, description = "Fuel price record id")),
    responses(
        (status = 200, description = "Fuel price record detail", body = NationwideFuelPrice),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub async fn get_fuel_price(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<NationwideFuelPrice>> {
    let sql = format!("SELECT {SELECT_COLS} FROM nationwide_fuel_price WHERE id = $1");
    let row = sqlx::query_as::<_, NationwideFuelPrice>(&sql)
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/fuel-prices", get(list_fuel_prices))
        .route("/fuel-prices/latest", get(latest_fuel_price))
        .route("/fuel-prices/{id}", get(get_fuel_price))
}
