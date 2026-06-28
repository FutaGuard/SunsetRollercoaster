use axum::{Json, Router, response::Html, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};
use utoipa::OpenApi;

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
        .route("/swagger-ui", get(swagger_ui))
        .route("/swagger-ui/", get(swagger_ui))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(api)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .expose_headers(Any),
        )
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

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn swagger_ui() -> Html<&'static str> {
    Html(SWAGGER_UI)
}

const SWAGGER_UI: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Sunset Rollercoaster API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  <style>
    :root {
      color-scheme: light dark;
      --page: #f5f7f8;
      --panel: #ffffff;
      --panel-muted: #eef2f4;
      --text: #1d252b;
      --muted: #66737d;
      --border: #d8e0e4;
      --accent: #1f7a8c;
      --accent-strong: #155e6d;
      --code: #22313a;
    }

    @media (prefers-color-scheme: dark) {
      :root {
        --page: #1f262b;
        --panel: #283137;
        --panel-muted: #222b31;
        --text: #edf2f4;
        --muted: #b3c0c8;
        --border: #3d4a52;
        --accent: #72c7d5;
        --accent-strong: #9edbe4;
        --code: #d8edf1;
      }
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      background: var(--page);
      color: var(--text);
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    .swagger-ui {
      color: var(--text);
    }

    .swagger-ui .topbar {
      display: none;
    }

    .swagger-ui .wrapper {
      max-width: 1180px;
      padding: 32px 24px;
    }

    .swagger-ui .info {
      margin: 24px 0 28px;
    }

    .swagger-ui .info .title,
    .swagger-ui .info p,
    .swagger-ui .info li,
    .swagger-ui .info table,
    .swagger-ui .info a,
    .swagger-ui .opblock-tag,
    .swagger-ui .opblock .opblock-summary-description,
    .swagger-ui .opblock .opblock-summary-path,
    .swagger-ui .opblock .opblock-summary-path__deprecated,
    .swagger-ui .responses-inner h4,
    .swagger-ui .responses-inner h5,
    .swagger-ui .parameter__name,
    .swagger-ui .parameter__type,
    .swagger-ui .tab li,
    .swagger-ui table thead tr td,
    .swagger-ui table thead tr th,
    .swagger-ui .response-col_status,
    .swagger-ui .response-col_description,
    .swagger-ui .model-title,
    .swagger-ui .model,
    .swagger-ui .prop-format,
    .swagger-ui .prop-type {
      color: var(--text);
    }

    .swagger-ui .info .title small {
      background: var(--accent);
      color: #ffffff;
    }

    .swagger-ui .scheme-container,
    .swagger-ui .opblock,
    .swagger-ui .models,
    .swagger-ui section.models {
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      box-shadow: none;
    }

    .swagger-ui .opblock .opblock-summary,
    .swagger-ui section.models h4 {
      border-color: var(--border);
    }

    .swagger-ui .opblock-tag {
      border-bottom-color: var(--border);
    }

    .swagger-ui .opblock .opblock-section-header {
      background: var(--panel-muted);
      box-shadow: none;
    }

    .swagger-ui textarea,
    .swagger-ui select,
    .swagger-ui input[type=text],
    .swagger-ui input[type=password],
    .swagger-ui input[type=search],
    .swagger-ui input[type=email],
    .swagger-ui input[type=file] {
      background: var(--panel-muted);
      border: 1px solid var(--border);
      color: var(--text);
    }

    .swagger-ui .btn {
      border-color: var(--accent);
      color: var(--accent-strong);
      box-shadow: none;
    }

    .swagger-ui .btn.authorize {
      border-color: var(--accent);
      color: var(--accent-strong);
    }

    .swagger-ui .btn.execute {
      background: var(--accent);
      border-color: var(--accent);
      color: #ffffff;
    }

    .swagger-ui .highlight-code,
    .swagger-ui .microlight {
      background: var(--panel-muted) !important;
      color: var(--code) !important;
    }

    .swagger-ui .opblock.opblock-get,
    .swagger-ui .opblock.opblock-post,
    .swagger-ui .opblock.opblock-put,
    .swagger-ui .opblock.opblock-delete,
    .swagger-ui .opblock.opblock-patch {
      background: var(--panel);
      border-color: var(--border);
    }

    .swagger-ui .opblock.opblock-get .opblock-summary {
      border-color: var(--border);
    }

    .swagger-ui .opblock.opblock-get .opblock-summary-method {
      background: #2f7d9a;
    }

    .swagger-ui .opblock.opblock-post .opblock-summary-method {
      background: #4f8f65;
    }

    .swagger-ui .opblock.opblock-put .opblock-summary-method {
      background: #b17b35;
    }

    .swagger-ui .opblock.opblock-delete .opblock-summary-method {
      background: #a65055;
    }

    .swagger-ui .opblock.opblock-patch .opblock-summary-method {
      background: #7d6aa8;
    }

    .swagger-ui .model-box,
    .swagger-ui .model-container {
      background: var(--panel-muted);
      border-color: var(--border);
    }

    .swagger-ui table tbody tr td {
      border-color: var(--border);
    }

    .swagger-ui .dialog-ux .modal-ux {
      background: var(--panel);
      border: 1px solid var(--border);
      box-shadow: 0 16px 40px rgba(12, 18, 24, 0.22);
    }

    .swagger-ui .dialog-ux .modal-ux-header {
      border-bottom-color: var(--border);
    }
  </style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({
      url: "/api-docs/openapi.json",
      dom_id: "#swagger-ui",
      deepLinking: true,
      layout: "BaseLayout",
      presets: [SwaggerUIBundle.presets.apis],
    });
  </script>
</body>
</html>
"##;
