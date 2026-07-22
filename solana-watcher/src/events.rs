//! Parsing of Anchor `DepositEvent` entries from transaction log messages.
//!
//! Anchor emits events as `Program data: <base64>` log lines. The decoded
//! payload is an 8-byte discriminator (`sha256("event:DepositEvent")[..8]`)
//! followed by the borsh-encoded event body:
//!   user_id: u64 LE | token: u8 (0=Sol, 1=Usdc) | amount: u64 LE | timestamp: i64 LE

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use sha2::{Digest, Sha256};

pub const PROGRAM_DATA_PREFIX: &str = "Program data: ";
pub const EVENT_NAME: &str = "DepositEvent";
/// 8-byte discriminator + 25-byte body.
pub const PAYLOAD_LEN: usize = 8 + 8 + 1 + 8 + 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepositToken {
    Sol,
    Usdc,
}

impl DepositToken {
    pub fn as_str(&self) -> &'static str {
        match self {
            DepositToken::Sol => "sol",
            DepositToken::Usdc => "usdc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepositEvent {
    pub user_id: u64,
    pub token: DepositToken,
    /// Lamports for SOL (9 decimals) or USDC base units (6 decimals).
    pub amount: u64,
    pub timestamp: i64,
}

/// Anchor event discriminator: first 8 bytes of sha256("event:<EventName>").
pub fn event_discriminator() -> [u8; 8] {
    let hash = Sha256::digest(format!("event:{EVENT_NAME}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

/// Extract all DepositEvents from a transaction's log messages.
pub fn parse_events_from_logs(logs: &[String]) -> Vec<DepositEvent> {
    logs.iter().filter_map(|line| parse_log_line(line)).collect()
}

/// Parse a single log line. Returns None for non-event lines or malformed payloads.
pub fn parse_log_line(line: &str) -> Option<DepositEvent> {
    let b64 = line.strip_prefix(PROGRAM_DATA_PREFIX)?;
    let payload = B64.decode(b64.trim()).ok()?;
    parse_payload(&payload)
}

/// Decode a raw payload: discriminator check + fixed-layout field parsing.
pub fn parse_payload(data: &[u8]) -> Option<DepositEvent> {
    if data.len() != PAYLOAD_LEN {
        return None;
    }
    if data[..8] != event_discriminator() {
        return None;
    }
    let user_id = u64::from_le_bytes(data[8..16].try_into().ok()?);
    let token = match data[16] {
        0 => DepositToken::Sol,
        1 => DepositToken::Usdc,
        _ => return None, // unknown enum variant
    };
    let amount = u64::from_le_bytes(data[17..25].try_into().ok()?);
    let timestamp = i64::from_le_bytes(data[25..33].try_into().ok()?);
    Some(DepositEvent { user_id, token, amount, timestamp })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_payload(user_id: u64, token_variant: u8, amount: u64, timestamp: i64) -> Vec<u8> {
        let mut data = Vec::with_capacity(PAYLOAD_LEN);
        data.extend_from_slice(&event_discriminator());
        data.extend_from_slice(&user_id.to_le_bytes());
        data.push(token_variant);
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&timestamp.to_le_bytes());
        data
    }

    #[test]
    fn parses_valid_event() {
        let payload = build_payload(42, 1, 5_500_000, 1_700_000_000);
        let line = format!("{PROGRAM_DATA_PREFIX}{}", B64.encode(&payload));
        let event = parse_log_line(&line).expect("should parse");
        assert_eq!(event.user_id, 42);
        assert_eq!(event.token, DepositToken::Usdc);
        assert_eq!(event.amount, 5_500_000);
        assert_eq!(event.timestamp, 1_700_000_000);
    }

    #[test]
    fn rejects_wrong_discriminator() {
        let mut payload = build_payload(1, 0, 1_000_000_000, 0);
        payload[0] ^= 0xff; // corrupt discriminator
        let line = format!("{PROGRAM_DATA_PREFIX}{}", B64.encode(&payload));
        assert_eq!(parse_log_line(&line), None);
    }

    #[test]
    fn rejects_unknown_token_variant() {
        let payload = build_payload(1, 7, 1, 0);
        assert_eq!(parse_payload(&payload), None);
    }

    #[test]
    fn rejects_wrong_length_and_non_event_lines() {
        assert_eq!(parse_payload(&[0u8; 10]), None);
        assert_eq!(parse_log_line("Program log: hello"), None);
        assert_eq!(parse_log_line("Program data: !!!not-base64!!!"), None);
    }

    #[test]
    fn parses_multiple_events_from_logs() {
        let p1 = build_payload(1, 0, 2_000_000_000, 10);
        let p2 = build_payload(2, 1, 3_000_000, 20);
        let logs = vec![
            "Program log: something".to_string(),
            format!("{PROGRAM_DATA_PREFIX}{}", B64.encode(&p1)),
            format!("{PROGRAM_DATA_PREFIX}{}", B64.encode(&p2)),
        ];
        let events = parse_events_from_logs(&logs);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].token, DepositToken::Sol);
        assert_eq!(events[1].token, DepositToken::Usdc);
    }
}
