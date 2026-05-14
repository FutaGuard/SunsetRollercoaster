pub mod invoice;
pub mod reservoir;

use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

impl Pagination {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(100).clamp(1, 1000)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}
