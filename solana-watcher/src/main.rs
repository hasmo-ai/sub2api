//! solana-watcher: watches an Anchor program for DepositEvent logs and
//! credits sub2api user balances via the Admin API.

mod config;
mod credit;
mod events;
mod listener;
mod pricing;
mod store;

use anyhow::Result;
use config::Config;
use listener::Listener;
use store::Store;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let store = Store::open(&cfg.db_path)?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    Listener::new(cfg, store, http).run().await;
}
