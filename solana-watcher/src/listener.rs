//! Main polling loop: fetch finalized signatures for the program, walk them
//! oldest-first, parse DepositEvents, convert to USD, and credit balances.
//!
//! Error handling policy: any RPC or retryable credit failure aborts the
//! current round (`break`) without advancing the cursor past the failed
//! signature, so the next poll re-attempts it. The cursor only ever moves
//! forward over fully handled signatures — nothing is silently dropped.

use crate::config::Config;
use crate::credit::{CreditClient, CreditOutcome};
use crate::events::{parse_events_from_logs, DepositToken};
use crate::pricing::{sol_lamports_to_usd, usdc_units_to_usd, PriceClient};
use crate::store::Store;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

pub struct Listener {
    cfg: Config,
    rpc: RpcClient,
    store: Store,
    pricing: PriceClient,
    credit: CreditClient,
}

impl Listener {
    pub fn new(cfg: Config, store: Store, http: reqwest::Client) -> Self {
        let rpc = RpcClient::new(http.clone(), &cfg.rpc_http_url);
        let pricing = PriceClient::new(http.clone(), cfg.price_cache_ttl);
        let credit = CreditClient::new(http, &cfg.sub2api_base_url, &cfg.admin_api_key);
        Self { cfg, rpc, store, pricing, credit }
    }

    pub async fn run(&self) -> ! {
        info!(
            rpc = %self.cfg.rpc_http_url,
            program_id = %self.cfg.program_id,
            poll_secs = self.cfg.poll_interval.as_secs(),
            min_deposit_usd = self.cfg.min_deposit_usd,
            "solana-watcher started"
        );
        loop {
            if let Err(err) = self.poll_once().await {
                error!(error = format!("{err:#}"), "poll round failed, will retry next round");
            }
            tokio::time::sleep(self.cfg.poll_interval).await;
        }
    }

    async fn poll_once(&self) -> Result<()> {
        let until = self.store.cursor()?;
        let mut sigs = self.rpc.get_signatures_for_address(&self.cfg.program_id, until.as_deref()).await?;
        if sigs.is_empty() {
            debug!("no new signatures");
            return Ok(());
        }
        // RPC returns newest-first; process oldest-first for chronological crediting.
        sigs.reverse();
        info!(count = sigs.len(), "fetched new signatures");

        for sig in sigs {
            match self.process_signature(&sig).await {
                Ok(true) => self.store.set_cursor(&sig)?,
                Ok(false) => break, // retryable failure: stop here, retry next round
                Err(err) => {
                    error!(error = format!("{err:#}"), %sig, "failed to process signature");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Returns Ok(true) if the signature is fully handled and the cursor may
    /// advance past it, Ok(false) to retry it next round.
    async fn process_signature(&self, sig: &str) -> Result<bool> {
        if self.store.is_processed(sig)? {
            debug!(%sig, "already processed, skipping");
            return Ok(true);
        }

        let Some(tx) = self.rpc.get_transaction(sig).await? else {
            warn!(%sig, "transaction not available yet, will retry");
            return Ok(false);
        };

        if tx.err.is_some() {
            debug!(%sig, "on-chain transaction failed, skipping");
            return Ok(true); // no events in a failed tx; do not store, just pass
        }

        let events = parse_events_from_logs(&tx.log_messages);
        if events.is_empty() {
            return Ok(true);
        }

        let mut total_usd = 0.0f64;
        let mut total_native = 0u64;
        let mut first_user_id = 0u64;
        let mut tokens: Vec<&'static str> = Vec::new();

        for (idx, event) in events.iter().enumerate() {
            // First event uses the plain signature as idempotency key (per the
            // API contract); later events in the same tx get an index suffix.
            let key = if idx == 0 { sig.to_string() } else { format!("{sig}:{idx}") };

            let amount_usd = match event.token {
                DepositToken::Sol => {
                    let price = match self.pricing.sol_usd().await {
                        Ok(p) => p,
                        Err(err) => {
                            warn!(error = format!("{err:#}"), %sig, "SOL price unavailable, skipping SOL crediting this round");
                            return Ok(false);
                        }
                    };
                    sol_lamports_to_usd(event.amount, price)
                }
                DepositToken::Usdc => usdc_units_to_usd(event.amount),
            };

            if idx == 0 {
                first_user_id = event.user_id;
            }
            total_native = total_native.saturating_add(event.amount);
            tokens.push(event.token.as_str());

            if amount_usd < self.cfg.min_deposit_usd {
                info!(%sig, user_id = event.user_id, amount_usd, "deposit below minimum, recording without crediting");
                total_usd += amount_usd;
                continue;
            }

            match self.credit.credit_logged(&key, event.user_id, amount_usd).await {
                CreditOutcome::Credited | CreditOutcome::Conflict => {
                    total_usd += amount_usd;
                    info!(%sig, user_id = event.user_id, amount_usd, "deposit credited");
                }
                CreditOutcome::Retry => return Ok(false),
            }
        }

        let token_label = if tokens.iter().all(|t| *t == tokens[0]) { tokens[0] } else { "multi" };
        self.store.record_processed(sig, first_user_id, token_label, total_native, total_usd)?;
        Ok(true)
    }
}

/// Minimal JSON-RPC client (avoids pulling in the full solana-client stack).
struct RpcClient {
    http: reqwest::Client,
    url: String,
}

#[derive(Debug, Deserialize)]
struct SignatureInfo {
    signature: String,
}

#[derive(Debug)]
struct TransactionLogs {
    err: Option<Value>,
    log_messages: Vec<String>,
}

impl RpcClient {
    fn new(http: reqwest::Client, url: &str) -> Self {
        Self { http, url: url.to_string() }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        #[derive(Deserialize)]
        struct RpcResponse<T> {
            result: Option<T>,
            error: Option<Value>,
        }
        let resp: RpcResponse<T> = self
            .http
            .post(&self.url)
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .send()
            .await
            .with_context(|| format!("rpc {method} request failed"))?
            .json()
            .await
            .with_context(|| format!("rpc {method} response parse failed"))?;
        if let Some(err) = resp.error {
            anyhow::bail!("rpc {method} error: {err}");
        }
        resp.result.with_context(|| format!("rpc {method} missing result"))
    }

    /// Newest-first signatures for the program, strictly newer than `until`.
    async fn get_signatures_for_address(
        &self,
        program_id: &str,
        until: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut cfg = json!({"commitment": "finalized", "limit": 100});
        if let Some(sig) = until {
            cfg["until"] = json!(sig);
        }
        let infos: Vec<SignatureInfo> =
            self.call("getSignaturesForAddress", json!([program_id, cfg])).await?;
        Ok(infos.into_iter().map(|i| i.signature).collect())
    }

    /// Transaction metadata: error flag and log messages. None if not found.
    async fn get_transaction(&self, signature: &str) -> Result<Option<TransactionLogs>> {
        #[derive(Deserialize)]
        struct TxResult {
            meta: Option<TxMeta>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TxMeta {
            err: Option<Value>,
            log_messages: Option<Vec<String>>,
        }
        let result: Option<TxResult> = self
            .call(
                "getTransaction",
                json!([
                    signature,
                    {"encoding": "json", "commitment": "finalized", "maxSupportedTransactionVersion": 0}
                ]),
            )
            .await?;
        Ok(result.map(|tx| {
            let meta = tx.meta.unwrap_or(TxMeta { err: None, log_messages: None });
            TransactionLogs { err: meta.err, log_messages: meta.log_messages.unwrap_or_default() }
        }))
    }
}
