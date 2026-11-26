use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::PgPool;

use crate::error::ApiResult;
use crate::fire_dispatch::{self, FireDispatchCache};
use crate::models::{FireDispatchCounty, FireDispatchCountyInfo};

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
        .route("/fire-dispatch/counties", get(list_counties))
        .route("/fire-dispatch/latest", get(latest_all))
        .route("/fire-dispatch/latest/{county}", get(latest_county))
        .with_state(cache)
}
