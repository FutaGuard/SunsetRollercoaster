use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::{Json, Router, extract::State, routing::get};
use serde_json::{Value, json};
use sunset_api::exchange_rate::{ExchangeRateCache, ExchangeRateError};
use tokio::sync::Mutex;

#[derive(Clone)]
struct Upstream {
    hits: Arc<AtomicUsize>,
    body: Arc<Mutex<Value>>,
}

fn rates_body(usd: f64) -> Value {
    json!({
        "quote": "TWD",
        "fetched_at": "2026-10-20T03:06:33.294Z",
        "rates": [
            {"base": "USD", "quote": "TWD", "rate": usd},
            {"base": "CHF", "quote": "TWD", "rate": 39.508520000000004},
            {"base": "VND", "quote": "TWD", "rate": null}
        ]
    })
}

async fn serve(body: Value) -> (String, Upstream) {
    let upstream = Upstream {
        hits: Arc::new(AtomicUsize::new(0)),
        body: Arc::new(Mutex::new(body)),
    };
    let app = Router::new()
        .route(
            "/exec",
            get(|State(upstream): State<Upstream>| async move {
                upstream.hits.fetch_add(1, Ordering::SeqCst);
                Json(upstream.body.lock().await.clone())
            }),
        )
        .with_state(upstream.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/exec", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, upstream)
}

#[tokio::test]
async fn serves_cached_rates_until_ttl_expires() {
    let (url, upstream) = serve(rates_body(32.5885)).await;
    let cache = ExchangeRateCache::new(Some(url), Duration::from_millis(300)).unwrap();

    let first = cache.get().await.unwrap();
    *upstream.body.lock().await = rates_body(33.0);
    let cached = cache.get().await.unwrap();

    assert_eq!(upstream.hits.load(Ordering::SeqCst), 1);
    assert_eq!(cached, first);
    let pairs: Vec<_> = first.iter().map(|r| (r.base.as_str(), r.rate)).collect();
    assert_eq!(pairs, [("USD", 32.5885), ("CHF", 39.50852)]);

    tokio::time::sleep(Duration::from_millis(350)).await;
    let refreshed = cache.get().await.unwrap();

    assert_eq!(upstream.hits.load(Ordering::SeqCst), 2);
    assert_eq!(refreshed[0].rate, 33.0);
}

#[tokio::test]
async fn concurrent_misses_share_one_upstream_request() {
    let (url, upstream) = serve(rates_body(32.5885)).await;
    let cache = Arc::new(ExchangeRateCache::new(Some(url), Duration::from_secs(180)).unwrap());

    let requests: Vec<_> = (0..8)
        .map(|_| {
            let cache = cache.clone();
            tokio::spawn(async move { cache.get().await.unwrap() })
        })
        .collect();
    for request in requests {
        request.await.unwrap();
    }

    assert_eq!(upstream.hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn source_errors_are_not_cached() {
    let (url, upstream) = serve(json!({"error": "Lock timeout"})).await;
    let cache = ExchangeRateCache::new(Some(url), Duration::from_secs(180)).unwrap();

    let error = cache.get().await.unwrap_err();
    assert!(matches!(&error, ExchangeRateError::Source(message) if message == "Lock timeout"));

    *upstream.body.lock().await = rates_body(32.5885);
    let rates = cache.get().await.unwrap();

    assert_eq!(upstream.hits.load(Ordering::SeqCst), 2);
    assert_eq!(rates[0].rate, 32.5885);
}

#[tokio::test]
async fn request_errors_do_not_expose_source_url() {
    let (url, _upstream) = serve(rates_body(32.5885)).await;
    let missing = url.replace("/exec", "/secret-deployment-id/missing");
    let cache = ExchangeRateCache::new(Some(missing), Duration::from_secs(180)).unwrap();

    let message = cache.get().await.unwrap_err().to_string();

    assert!(message.contains("404"), "{message}");
    assert!(!message.contains("secret-deployment-id"), "{message}");
}

#[tokio::test]
async fn missing_source_url_is_not_configured() {
    let cache = ExchangeRateCache::new(None, Duration::from_secs(180)).unwrap();

    assert!(matches!(
        cache.get().await.unwrap_err(),
        ExchangeRateError::NotConfigured
    ));
}
