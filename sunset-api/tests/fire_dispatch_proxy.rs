use std::sync::{Arc, Mutex};

use axum::{Router, extract::Request, http::header::AUTHORIZATION};
use chrono::{FixedOffset, Utc};
use sunset_api::config::FireDispatchConfig;
use sunset_api::fire_dispatch::FireDispatchCache;

type Seen = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A URL-prefix proxy that records each request and answers with a one-case DTS table.
async fn serve_proxy() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let recorder = seen.clone();
    let app = Router::new().fallback(move |request: Request| {
        let recorder = recorder.clone();
        async move {
            let target = request
                .uri()
                .path_and_query()
                .map(ToString::to_string)
                .unwrap_or_default();
            let auth = request
                .headers()
                .get(AUTHORIZATION)
                .map(|value| value.to_str().unwrap().to_string());
            recorder.lock().unwrap().push((target, auth));
            let received = Utc::now()
                .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
                .format("%Y/%m/%d %H:%M:%S");
            format!(
                "<table><tr><th>序號</th><th>受理時間</th><th>案類</th><th>發生地點</th>\
                 <th>派遣分隊</th><th>執行狀況</th></tr><tr><td>1</td><td>{received}</td>\
                 <td>緊急救護</td><td>高雄市林園區</td><td>林園分隊</td><td>已出動</td></tr></table>"
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/relay/", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, seen)
}

fn config(proxy_url: String, counties: &[&str]) -> FireDispatchConfig {
    FireDispatchConfig {
        proxy_url,
        proxy_basic_auth: Some("user:pass".to_string()),
        proxy_counties: counties.iter().map(ToString::to_string).collect(),
    }
}

#[tokio::test]
async fn configured_county_goes_through_proxy_with_basic_auth() {
    let (url, seen) = serve_proxy().await;
    let cache = FireDispatchCache::new(Some(config(url, &["kaohsiung"]))).unwrap();

    let county = cache.county("kaohsiung").await.unwrap();

    assert_eq!(
        county.source_url,
        "https://119dts.fdkc.gov.tw/DTS/caselist/html"
    );
    assert_eq!(county.cases.len(), 1);
    assert_eq!(county.cases[0].unit.as_deref(), Some("林園分隊"));
    assert_eq!(
        *seen.lock().unwrap(),
        vec![(
            "/relay/https://119dts.fdkc.gov.tw/DTS/caselist/html".to_string(),
            Some("Basic dXNlcjpwYXNz".to_string())
        )]
    );
}

#[tokio::test]
async fn proxy_failures_do_not_expose_proxy_url() {
    // Bind then drop a listener so the port refuses connections.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/secret-token/", listener.local_addr().unwrap());
    drop(listener);
    let cache = FireDispatchCache::new(Some(config(url, &["kaohsiung"]))).unwrap();

    let error = cache.county("kaohsiung").await.unwrap_err().to_string();

    assert!(!error.contains("secret-token"), "{error}");
}

#[test]
fn unknown_proxy_county_and_malformed_auth_are_rejected() {
    let unknown = FireDispatchCache::new(Some(config("http://proxy".into(), &["atlantis"])));
    let error = unknown.err().expect("unknown county must be rejected");
    assert!(error.to_string().contains("atlantis"));

    let malformed = FireDispatchCache::new(Some(FireDispatchConfig {
        proxy_basic_auth: Some("no-colon".to_string()),
        ..config("http://proxy".into(), &["kaohsiung"])
    }));
    assert!(malformed.is_err());
}
