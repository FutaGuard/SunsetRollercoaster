use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::DatabaseConfig;

pub async fn create_pool(cfg: &DatabaseConfig) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.url())
        .await?;
    Ok(pool)
}
