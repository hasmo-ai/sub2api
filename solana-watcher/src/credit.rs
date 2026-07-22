//! Crediting user balances through the sub2api Admin API
//! (POST /api/v1/admin/redeem-codes/create-and-redeem).

use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::{debug, warn};

/// How the API call turned out, from the listener's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditOutcome {
    /// 200: credited (or idempotent replay of an already-credited code+user).
    Credited,
    /// 409: same code bound to a different user. Treated as handled (warn).
    Conflict,
    /// Network/5xx/other failure after retries: leave for the next poll round.
    Retry,
}

const MAX_ATTEMPTS: u32 = 4;

/// Derive the redeem code from a key (usually "sig" or "sig:index").
/// `"sol_" + hex(sha256(key))[..28]` = 32 chars, matching the backend MaxLen(32)
/// and making the code naturally idempotent per deposit.
pub fn derive_code(key: &str) -> String {
    let hash = hex::encode(Sha256::digest(key.as_bytes()));
    format!("sol_{}", &hash[..28])
}

#[derive(Debug, Serialize)]
struct CreateAndRedeemRequest<'a> {
    code: &'a str,
    #[serde(rename = "type")]
    kind: &'a str,
    value: f64,
    user_id: u64,
    notes: &'a str,
}

pub struct CreditClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl CreditClient {
    pub fn new(http: reqwest::Client, base_url: &str, api_key: &str) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// Credit `amount_usd` to `user_id` for deposit `key` (tx signature,
    /// optionally with an event index suffix). Retries transient failures
    /// with exponential backoff before giving up until the next poll round.
    pub async fn credit(&self, key: &str, user_id: u64, amount_usd: f64) -> Result<CreditOutcome> {
        let code = derive_code(key);
        let url = format!("{}/api/v1/admin/redeem-codes/create-and-redeem", self.base_url);
        let idempotency_key = format!("sol-{key}");
        let body = CreateAndRedeemRequest {
            code: &code,
            kind: "balance",
            value: amount_usd,
            user_id,
            notes: &format!("solana deposit tx: {key}"),
        };

        let mut delay = Duration::from_secs(1);
        for attempt in 1..=MAX_ATTEMPTS {
            let result = self
                .http
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("Idempotency-Key", &idempotency_key)
                .json(&body)
                .send()
                .await;

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    if status == reqwest::StatusCode::OK {
                        debug!(%code, user_id, amount_usd, "credited");
                        return Ok(CreditOutcome::Credited);
                    }
                    if status == reqwest::StatusCode::CONFLICT {
                        warn!(%code, user_id, "credit conflict (code bound to another user), treating as handled");
                        return Ok(CreditOutcome::Conflict);
                    }
                    let text = resp.text().await.unwrap_or_default();
                    if status.is_client_error() {
                        // 4xx other than 409 will never succeed on retry; surface it.
                        anyhow::bail!("credit API returned {status}: {text}");
                    }
                    warn!(%status, attempt, "credit API server error, backing off: {text}");
                }
                Err(err) => {
                    warn!(attempt, error = %err, "credit API request failed, backing off");
                }
            }

            if attempt < MAX_ATTEMPTS {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
        Ok(CreditOutcome::Retry)
    }

    /// Non-retryable API errors should be logged but must not crash the loop.
    pub async fn credit_logged(&self, key: &str, user_id: u64, amount_usd: f64) -> CreditOutcome {
        match self.credit(key, user_id, amount_usd).await {
            Ok(outcome) => outcome,
            Err(err) => {
                warn!(error = format!("{err:#}"), %key, user_id, "credit failed permanently this round");
                CreditOutcome::Retry
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_derivation_is_32_chars_and_deterministic() {
        let sig = "5j7s8K3mN0pQ...fake-signature";
        let code = derive_code(sig);
        assert_eq!(code.len(), 32);
        assert!(code.starts_with("sol_"));
        assert!(code[4..].chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(code, derive_code(sig)); // deterministic
        assert_ne!(code, derive_code("another-signature"));
        // Event-indexed variant for multi-event transactions.
        assert_eq!(derive_code(&format!("{sig}:1")).len(), 32);
    }

    #[test]
    fn code_matches_sha256_spec() {
        let sig = "abc";
        let expected = format!("sol_{}", &hex::encode(Sha256::digest(b"abc"))[..28]);
        assert_eq!(derive_code(sig), expected);
    }
}
