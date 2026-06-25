//! Local recent-fills snapshot for toolbar picker + diff (design §8.2, Phase 6).

use crate::broker::BrokerFill;
use anyhow::Context;
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct RecentTradesStore {
    conn: Arc<Mutex<Connection>>,
}

impl RecentTradesStore {
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("open recent fills db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS recent_fills (
  fill_id TEXT PRIMARY KEY,
  trade_id TEXT NOT NULL,
  symbol TEXT NOT NULL,
  side TEXT NOT NULL,
  qty REAL NOT NULL,
  price REAL NOT NULL,
  filled_at_rfc3339 TEXT NOT NULL,
  broker TEXT NOT NULL,
  inserted_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_recent_fills_inserted ON recent_fills(inserted_at_ms DESC);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn upsert_fill(&self, fill: &BrokerFill) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let inserted_ms = Utc::now().timestamp_millis();
        guard.execute(
            r#"INSERT INTO recent_fills (
                fill_id, trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, inserted_at_ms
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
            ON CONFLICT(fill_id) DO UPDATE SET
              trade_id=excluded.trade_id,
              symbol=excluded.symbol,
              side=excluded.side,
              qty=excluded.qty,
              price=excluded.price,
              filled_at_rfc3339=excluded.filled_at_rfc3339,
              broker=excluded.broker"#,
            params![
                fill.fill_id,
                fill.trade_id,
                fill.symbol,
                fill.side,
                fill.qty,
                fill.price,
                fill.filled_at
                    .to_rfc3339_opts(SecondsFormat::Millis, true),
                fill.broker,
                inserted_ms
            ],
        )?;
        Ok(())
    }

    pub fn fetch_recent_json(&self, limit: usize) -> anyhow::Result<Vec<serde_json::Value>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, fill_id
             FROM recent_fills ORDER BY inserted_at_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            let trade_id: String = r.get(0)?;
            let symbol: String = r.get(1)?;
            let side: String = r.get(2)?;
            let qty: f64 = r.get(3)?;
            let price: f64 = r.get(4)?;
            let filled_at: String = r.get(5)?;
            let broker: String = r.get(6)?;
            let fill_id: String = r.get(7)?;
            let filled_at_ms: i64 = DateTime::parse_from_rfc3339(&filled_at)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            Ok(json!({
                "tradeId": &trade_id,
                "trade_id": trade_id,
                "symbol": symbol,
                "side": side,
                "qty": qty,
                "price": price,
                "filledAt": filled_at,
                "filled_at_ms": filled_at_ms,
                "broker": broker,
                "fillId": fill_id,
            }))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Returns newly persisted fills (excluding known `fill_id`s).
    pub fn merge_poll(&self, fills: &[BrokerFill]) -> anyhow::Result<Vec<BrokerFill>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut new_fills = Vec::new();
        let inserted_ms_base = Utc::now().timestamp_millis();
        let mut chk =
            guard.prepare_cached("SELECT 1 FROM recent_fills WHERE fill_id = ?1 LIMIT 1")?;

        for (i, fill) in fills.iter().enumerate() {
            let exists = chk.exists(params![&fill.fill_id])?;
            if exists {
                continue;
            }

            guard.execute(
                r#"INSERT INTO recent_fills (
                    fill_id, trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, inserted_at_ms
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"#,
                params![
                    fill.fill_id,
                    fill.trade_id,
                    fill.symbol,
                    fill.side,
                    fill.qty,
                    fill.price,
                    fill.filled_at
                        .to_rfc3339_opts(SecondsFormat::Millis, true),
                    fill.broker,
                    inserted_ms_base + i as i64
                ],
            )?;
            new_fills.push(fill.clone());
        }
        Ok(new_fills)
    }

    pub fn newest_filled_at(&self) -> anyhow::Result<Option<DateTime<Utc>>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let v: Option<String> = guard
            .query_row(
                "SELECT filled_at_rfc3339 FROM recent_fills ORDER BY inserted_at_ms DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;

        Ok(match v {
            Some(ref s) => Some(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc)),
            None => None,
        })
    }
}
