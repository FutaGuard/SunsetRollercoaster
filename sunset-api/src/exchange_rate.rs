//! Real-time exchange rates from the Apps Script Web App (apps-script/finance),
//! cached in memory so clients never hit Google more than once per TTL.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::models::LiveExchangeRate;

pub const CACHE_TTL: Duration = Duration::from_secs(180);
/// Apps Script retries GOOGLEFINANCE for a few seconds, so allow a generous budget.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// GOOGLEFINANCE provides about 10 significant digits; the rest is float noise.
const SIGNIFICANT_DIGITS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum ExchangeRateError {
    #[error("exchange rate source is not configured")]
    NotConfigured,
    /// Built via `From`, which strips the secret source URL from the message.
    #[error("exchange rate source request failed: {0}")]
    Request(reqwest::Error),
    #[error("exchange rate source returned an error: {0}")]
    Source(String),
    #[error("exchange rate source response is invalid: {0}")]
    Invalid(String),
}

impl From<reqwest::Error> for ExchangeRateError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error.without_url())
    }
}

pub type LiveRates = Arc<Vec<LiveExchangeRate>>;

struct CacheEntry {
    loaded_at: Instant,
    rates: LiveRates,
}

pub struct ExchangeRateCache {
    client: reqwest::Client,
    url: Option<String>,
    ttl: Duration,
    entry: Mutex<Option<CacheEntry>>,
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
            entry: Mutex::new(None),
        })
    }

    /// Returns cached rates younger than the TTL; otherwise fetches fresh ones.
    /// The lock is held while fetching so concurrent misses share one request.
    /// Failures are not cached.
    pub async fn get(&self) -> Result<LiveRates, ExchangeRateError> {
        let url = self
            .url
            .as_deref()
            .ok_or(ExchangeRateError::NotConfigured)?;
        let mut entry = self.entry.lock().await;
        if let Some(cached) = entry.as_ref() {
            if cached.loaded_at.elapsed() < self.ttl {
                return Ok(cached.rates.clone());
            }
        }

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
        *entry = Some(CacheEntry {
            loaded_at: Instant::now(),
            rates: rates.clone(),
        });
        Ok(rates)
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
