//! SQLite persistence: processed-transaction ledger (first idempotency line)
//! and the polling cursor.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (or create) the database and ensure the schema exists.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("open db {path}"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS processed_txs (
                signature     TEXT PRIMARY KEY,
                user_id       INTEGER NOT NULL,
                token         TEXT NOT NULL,
                amount_native INTEGER NOT NULL,
                amount_usd    REAL NOT NULL,
                credited_at   INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cursor (
                id             INTEGER PRIMARY KEY CHECK (id = 1),
                last_signature TEXT
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn is_processed(&self, signature: &str) -> Result<bool> {
        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM processed_txs WHERE signature = ?1",
                params![signature],
                |row| row.get(0),
            )
            .optional()?;
        Ok(exists.is_some())
    }

    /// Record a fully handled transaction (credited or intentionally skipped
    /// for being below the minimum). INSERT OR IGNORE keeps it idempotent.
    pub fn record_processed(
        &self,
        signature: &str,
        user_id: u64,
        token: &str,
        amount_native: u64,
        amount_usd: f64,
    ) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        self.conn.execute(
            "INSERT OR IGNORE INTO processed_txs
             (signature, user_id, token, amount_native, amount_usd, credited_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![signature, user_id as i64, token, amount_native as i64, amount_usd, now],
        )?;
        Ok(())
    }

    pub fn cursor(&self) -> Result<Option<String>> {
        let sig: Option<String> = self
            .conn
            .query_row("SELECT last_signature FROM cursor WHERE id = 1", [], |row| row.get(0))
            .optional()?;
        Ok(sig)
    }

    pub fn set_cursor(&self, signature: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cursor (id, last_signature) VALUES (1, ?1)
             ON CONFLICT (id) DO UPDATE SET last_signature = excluded.last_signature",
            params![signature],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_processed_and_cursor() {
        let store = Store::open(":memory:").unwrap();
        assert!(!store.is_processed("sig1").unwrap());
        store.record_processed("sig1", 7, "sol", 1_000_000_000, 12.5).unwrap();
        assert!(store.is_processed("sig1").unwrap());
        // Idempotent re-insert must not fail.
        store.record_processed("sig1", 7, "sol", 1_000_000_000, 12.5).unwrap();

        assert_eq!(store.cursor().unwrap(), None);
        store.set_cursor("sig1").unwrap();
        assert_eq!(store.cursor().unwrap().as_deref(), Some("sig1"));
        store.set_cursor("sig2").unwrap();
        assert_eq!(store.cursor().unwrap().as_deref(), Some("sig2"));
    }
}
