//! Real-time exchange rates from the Apps Script Web App (apps-script/finance),
//! served from the last usable snapshot while one background request refreshes it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::time::Instant;

use crate::models::LiveExchangeRate;

pub const CACHE_TTL: Duration = Duration::from_secs(180);
const MAX_AGE: Duration = Duration::from_secs(600);
const CLIENT_WAIT: Duration = Duration::from_secs(5);
/// Apps Script retries GOOGLEFINANCE for a few seconds, so allow a generous budget.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// GOOGLEFINANCE provides about 10 significant digits; the rest is float noise.
const SIGNIFICANT_DIGITS: usize = 10;

#[derive(Clone, Debug, thiserror::Error)]
pub enum ExchangeRateError {
    #[error("exchange rate source is not configured")]
    NotConfigured,
    /// Built via `From`, which strips the secret source URL from the message.
    #[error("exchange rate source request failed: {0}")]
    Request(Arc<reqwest::Error>),
    #[error("exchange rate source returned an error: {0}")]
    Source(String),
    #[error("exchange rate source response is invalid: {0}")]
    Invalid(String),
    #[error("no usable exchange rates; source refresh still pending")]
    RefreshPending,
}

impl From<reqwest::Error> for ExchangeRateError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(Arc::new(error.without_url()))
    }
}

pub type LiveRates = Arc<Vec<LiveExchangeRate>>;

struct CacheEntry {
    loaded_at: Instant,
    usable_until: Instant,
    rates: LiveRates,
}

type RefreshResult = Result<LiveRates, ExchangeRateError>;

#[derive(Default)]
struct CacheState {
    entry: Option<CacheEntry>,
    refreshing: Option<broadcast::Sender<RefreshResult>>,
}

pub struct ExchangeRateCache {
    client: reqwest::Client,
    url: Option<String>,
    ttl: Duration,
    state: Mutex<CacheState>,
}

impl ExchangeRateCache {
    pub fn new(url: Option<String>, ttl: Duration) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self {
            client,
            url,
            ttl,
            state: Mutex::default(),
        })
    }

    pub async fn get(self: &Arc<Self>) -> Result<LiveRates, ExchangeRateError> {
        let mut pending = {
            let mut state = self.state.lock().unwrap();
            let now = Instant::now();
            let usable = state.entry.as_ref().filter(|e| now < e.usable_until);
            if let Some(entry) = usable
                && now - entry.loaded_at < self.ttl
            {
                return Ok(entry.rates.clone());
            }
            let stale = usable.map(|entry| entry.rates.clone());
            let refresh = state.refreshing.get_or_insert_with(|| {
                let cache = self.clone();
                tokio::spawn(async move { cache.refresh().await });
                broadcast::channel(1).0
            });
            if let Some(rates) = stale {
                return Ok(rates);
            }
            refresh.subscribe()
        };
        tokio::time::timeout(CLIENT_WAIT, pending.recv())
            .await
            .map_err(|_| ExchangeRateError::RefreshPending)?
            .expect("refresh publishes one result before closing")
    }

    async fn refresh(&self) {
        let result = self.fetch().await;
        let mut state = self.state.lock().unwrap();
        let outcome = result.map(|entry| {
            let rates = entry.rates.clone();
            state.entry = Some(entry);
            rates
        });
        if let Err(error) = &outcome {
            tracing::warn!(%error, "exchange rate refresh failed");
        }
        let _ = state.refreshing.take().unwrap().send(outcome);
    }

    async fn fetch(&self) -> Result<CacheEntry, ExchangeRateError> {
        let url = self.url.as_deref().ok_or(ExchangeRateError::NotConfigured)?;

        let payload: SourcePayload = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let rates = Arc::new(parse_payload(payload)?);
        // parse_payload guarantees a nonempty table with one source timestamp.
        let age = (Utc::now() - rates[0].observed_at)
            .to_std()
            .unwrap_or_default();
        if age >= MAX_AGE {
            return Err(ExchangeRateError::Invalid(
                "source snapshot is too old".into(),
            ));
        }
        let loaded_at = Instant::now();
        Ok(CacheEntry {
            loaded_at,
            usable_until: loaded_at + (MAX_AGE - age),
            rates,
        })
    }
}

#[derive(Debug, Deserialize)]
struct SourcePayload {
    error: Option<String>,
    fetched_at: Option<DateTime<Utc>>,
    #[serde(default)]
    rates: Vec<SourceRate>,
}

#[derive(Debug, Deserialize)]
struct SourceRate {
    base: String,
    quote: String,
    rate: Option<f64>,
}

fn parse_payload(payload: SourcePayload) -> Result<Vec<LiveExchangeRate>, ExchangeRateError> {
    if let Some(error) = payload.error {
        return Err(ExchangeRateError::Source(error));
    }
    let observed_at = payload
        .fetched_at
        .ok_or_else(|| ExchangeRateError::Invalid("missing fetched_at".to_string()))?;

    let mut rates = Vec::with_capacity(payload.rates.len());
    for item in payload.rates {
        if !is_currency(&item.base) || !is_currency(&item.quote) {
            return Err(ExchangeRateError::Invalid(format!(
                "invalid currency pair {}/{}",
                item.base, item.quote
            )));
        }
        // Apps Script returns null when one currency is temporarily unavailable.
        let Some(rate) = item.rate else {
            tracing::warn!(base = %item.base, quote = %item.quote, "exchange rate unavailable");
            continue;
        };
        if !rate.is_finite() || rate <= 0.0 {
            return Err(ExchangeRateError::Invalid(format!(
                "invalid rate {}{}: {rate}",
                item.base, item.quote
            )));
        }
        rates.push(LiveExchangeRate {
            base: item.base,
            quote: item.quote,
            rate: round_significant(rate),
            observed_at,
        });
    }

    if rates.is_empty() {
        return Err(ExchangeRateError::Invalid("no available rate".to_string()));
    }
    Ok(rates)
}

fn is_currency(code: &str) -> bool {
    code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase())
}

fn round_significant(value: f64) -> f64 {
    format!("{value:.prec$e}", prec = SIGNIFICANT_DIGITS - 1)
        .parse()
        .unwrap_or(value)
}
