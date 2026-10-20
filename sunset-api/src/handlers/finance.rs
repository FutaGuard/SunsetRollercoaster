use std::sync::Arc;

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
use crate::exchange_rate::{ExchangeRateCache, LiveRates};
use crate::models::{FinanceExchangeRate, LiveExchangeRate};

const TZ: &str = "Asia/Taipei";
const EXCHANGE_RATE_SELECT: &str = "id, base, quote, rate, observed_at";

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ExchangeRateListParams {
    /// ISO 4217 base currency, case-insensitive (for example, `USD`).
    pub base: Option<String>,
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

impl ExchangeRateListParams {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(500).clamp(1, 5000)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    fn base(&self) -> ApiResult<Option<String>> {
        let Some(base) = self.base.as_deref() else {
            return Ok(None);
        };
        if base.len() != 3 || !base.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(ApiError::BadRequest(format!(
                "invalid base currency `{base}`; expected a 3-letter ISO 4217 code"
            )));
        }
        Ok(Some(base.to_ascii_uppercase()))
    }
}

#[utoipa::path(
    get,
    path = "/finance/exchange-rates",
    tag = "finance",
    params(ExchangeRateListParams),
    responses(
        (status = 200, description = "List exchange rate snapshots, newest first", body = [FinanceExchangeRate]),
        (status = 400, description = "Invalid base currency", body = crate::error::ErrorBody)
    )
)]
pub async fn list_exchange_rates(
    State(pool): State<PgPool>,
    Query(params): Query<ExchangeRateListParams>,
) -> ApiResult<Json<Vec<FinanceExchangeRate>>> {
    let base = params.base()?;
    let mut sql = format!("SELECT {EXCHANGE_RATE_SELECT} FROM finance_exchange_rate WHERE 1=1");
    let mut index = 1;
    if base.is_some() {
        sql.push_str(&format!(" AND base = ${index}"));
        index += 1;
    }
    if params.date.is_some() {
        sql.push_str(&format!(
            " AND (observed_at AT TIME ZONE '{TZ}')::date = ${index}"
        ));
        index += 1;
    } else {
        if params.start.is_some() {
            sql.push_str(&format!(
                " AND (observed_at AT TIME ZONE '{TZ}')::date >= ${index}"
            ));
            index += 1;
        }
        if params.end.is_some() {
            sql.push_str(&format!(
                " AND (observed_at AT TIME ZONE '{TZ}')::date <= ${index}"
            ));
            index += 1;
        }
    }
    sql.push_str(&format!(
        " ORDER BY observed_at DESC, base ASC, quote ASC LIMIT ${index} OFFSET ${}",
        index + 1
    ));

    let mut query = sqlx::query_as::<_, FinanceExchangeRate>(&sql);
    if let Some(base) = base {
        query = query.bind(base);
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
    path = "/finance/exchange-rates/latest",
    tag = "finance",
    responses(
        (status = 200, description = "Real-time rate for each currency pair from Google Finance, cached in memory for 180 seconds", body = [LiveExchangeRate]),
        (status = 502, description = "Exchange rate source failed", body = crate::error::ErrorBody),
        (status = 503, description = "Exchange rate source not configured", body = crate::error::ErrorBody)
    )
)]
pub async fn latest_exchange_rates(
    State(cache): State<Arc<ExchangeRateCache>>,
) -> ApiResult<Json<LiveRates>> {
    Ok(Json(cache.get().await?))
}

pub fn router(cache: Arc<ExchangeRateCache>) -> Router<PgPool> {
    Router::new()
        .route("/finance/exchange-rates", get(list_exchange_rates))
        .merge(
            Router::new()
                .route("/finance/exchange-rates/latest", get(latest_exchange_rates))
                .with_state(cache),
        )
}
