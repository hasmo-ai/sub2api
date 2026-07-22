//! SOL/USD price from CoinGecko with a TTL cache, plus USD conversion helpers.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const COINGECKO_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";

const LAMPORTS_PER_SOL: f64 = 1e9;
const USDC_BASE_UNITS: f64 = 1e6;

#[derive(Debug, Deserialize)]
struct PriceResponse {
    solana: SolanaPrice,
}

#[derive(Debug, Deserialize)]
struct SolanaPrice {
    usd: f64,
}

pub struct PriceClient {
    http: reqwest::Client,
    ttl: Duration,
    cache: Mutex<Option<(Instant, f64)>>,
}

impl PriceClient {
    pub fn new(http: reqwest::Client, ttl: Duration) -> Self {
        Self { http, ttl, cache: Mutex::new(None) }
    }

    /// Current SOL/USD price. Cached for `ttl`; returns Err on fetch/parse
    /// failure so callers can skip SOL crediting (USDC is unaffected).
    pub async fn sol_usd(&self) -> Result<f64> {
        {
            let guard = self.cache.lock().await;
            if let Some((at, price)) = *guard {
                if at.elapsed() < self.ttl {
                    return Ok(price);
                }
            }
        }
        let resp: PriceResponse = self
            .http
            .get(COINGECKO_URL)
            .send()
            .await
            .context("coingecko request failed")?
            .error_for_status()
            .context("coingecko returned error status")?
            .json()
            .await
            .context("coingecko response parse failed")?;
        let price = resp.solana.usd;
        *self.cache.lock().await = Some((Instant::now(), price));
        Ok(price)
    }
}

/// Convert lamports to USD at the given SOL/USD price.
pub fn sol_lamports_to_usd(lamports: u64, sol_usd_price: f64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL * sol_usd_price
}

/// Convert USDC base units (6 decimals) to USD; USDC is treated 1:1.
pub fn usdc_units_to_usd(units: u64) -> f64 {
    units as f64 / USDC_BASE_UNITS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sol_conversion() {
        assert_eq!(sol_lamports_to_usd(1_000_000_000, 150.0), 150.0);
        assert_eq!(sol_lamports_to_usd(500_000_000, 200.0), 100.0);
        assert_eq!(sol_lamports_to_usd(0, 200.0), 0.0);
    }

    #[test]
    fn usdc_conversion() {
        assert_eq!(usdc_units_to_usd(1_000_000), 1.0);
        assert_eq!(usdc_units_to_usd(5_500_000), 5.5);
    }
}
