//! Environment-based configuration with devnet-friendly defaults.

use anyhow::{Context, Result};
use std::env;
use std::time::Duration;

pub const DEFAULT_RPC_HTTP_URL: &str = "https://api.devnet.solana.com";
pub const DEFAULT_PROGRAM_ID: &str = "ErRaZ4rnCLC3nZdwwHuUTtgqnDD3UFGCsBDvsxii1X3i";
pub const DEFAULT_SUB2API_BASE_URL: &str = "http://localhost:8080";
pub const DEFAULT_MIN_DEPOSIT_USD: f64 = 1.0;
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 15;
pub const DEFAULT_DB_PATH: &str = "./solana-watcher.db";
pub const DEFAULT_PRICE_CACHE_TTL_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_http_url: String,
    /// Anchor program to watch (base58, kept as string: we only pass it to JSON-RPC).
    pub program_id: String,
    pub sub2api_base_url: String,
    /// Required, no default.
    pub admin_api_key: String,
    pub min_deposit_usd: f64,
    pub poll_interval: Duration,
    pub db_path: String,
    pub price_cache_ttl: Duration,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let admin_api_key = env::var("ADMIN_API_KEY")
            .context("ADMIN_API_KEY is required (no default)")?;

        Ok(Self {
            rpc_http_url: env_string("RPC_HTTP_URL", DEFAULT_RPC_HTTP_URL),
            program_id: env_string("PROGRAM_ID", DEFAULT_PROGRAM_ID),
            sub2api_base_url: env_string("SUB2API_BASE_URL", DEFAULT_SUB2API_BASE_URL),
            admin_api_key,
            min_deposit_usd: env_parse("MIN_DEPOSIT_USD", DEFAULT_MIN_DEPOSIT_USD)?,
            poll_interval: Duration::from_secs(env_parse(
                "POLL_INTERVAL_SECS",
                DEFAULT_POLL_INTERVAL_SECS,
            )?),
            db_path: env_string("DB_PATH", DEFAULT_DB_PATH),
            price_cache_ttl: Duration::from_secs(env_parse(
                "PRICE_CACHE_TTL_SECS",
                DEFAULT_PRICE_CACHE_TTL_SECS,
            )?),
        })
    }
}

fn env_string(key: &str, default: &str) -> String {
    env::var(key).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| default.to_string())
}

fn env_parse<T>(key: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match env::var(key) {
        Ok(v) if !v.is_empty() => v
            .parse::<T>()
            .with_context(|| format!("failed to parse env var {key}={v:?}")),
        _ => Ok(default),
    }
}
