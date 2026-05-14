use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use sunset_api::config::AppConfig;
use sunset_api::openapi::ApiDoc;
use sunset_api::{db, handlers};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = AppConfig::load()?;
    let pool = db::create_pool(&cfg.database).await?;

    let api = Router::new()
        .merge(handlers::invoice::router())
        .merge(handlers::reservoir::router())
        .with_state(pool);

    let app = Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(api)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr} (swagger: /swagger-ui)");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> &'static str {
    "(´・ω・`)"
}

async fn healthz() -> &'static str {
    "ok"
}
