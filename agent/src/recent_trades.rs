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

fn ensure_optional_column(conn: &Connection, column: &str, ddl: &str) -> anyhow::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(recent_fills)")?;
    let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    conn.execute(ddl, [])?;
    Ok(())
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
  fee_amount REAL,
  fee_asset TEXT,
  currency TEXT,
  product TEXT,
  exchange_segment TEXT,
  inserted_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_recent_fills_inserted ON recent_fills(inserted_at_ms DESC);
"#,
        )?;
        ensure_optional_column(
            &conn,
            "fee_amount",
            "ALTER TABLE recent_fills ADD COLUMN fee_amount REAL",
        )?;
        ensure_optional_column(
            &conn,
            "fee_asset",
            "ALTER TABLE recent_fills ADD COLUMN fee_asset TEXT",
        )?;
        // I-N3: persist venue cash fields that FillEvent already carries.
        ensure_optional_column(
            &conn,
            "currency",
            "ALTER TABLE recent_fills ADD COLUMN currency TEXT",
        )?;
        ensure_optional_column(
            &conn,
            "product",
            "ALTER TABLE recent_fills ADD COLUMN product TEXT",
        )?;
        ensure_optional_column(
            &conn,
            "exchange_segment",
            "ALTER TABLE recent_fills ADD COLUMN exchange_segment TEXT",
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
                fill_id, trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, fee_amount, fee_asset, currency, product, exchange_segment, inserted_at_ms
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
            ON CONFLICT(fill_id) DO UPDATE SET
              trade_id=excluded.trade_id,
              symbol=excluded.symbol,
              side=excluded.side,
              qty=excluded.qty,
              price=excluded.price,
              filled_at_rfc3339=excluded.filled_at_rfc3339,
              broker=excluded.broker,
              fee_amount=excluded.fee_amount,
              fee_asset=excluded.fee_asset,
              currency=excluded.currency,
              product=excluded.product,
              exchange_segment=excluded.exchange_segment"#,
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
                fill.fee_amount,
                fill.fee_asset.as_deref(),
                fill.currency.as_deref(),
                fill.product.as_deref(),
                fill.exchange_segment.as_deref(),
                inserted_ms
            ],
        )?;
        Ok(())
    }

    pub fn fetch_recent_json(&self, limit: usize) -> anyhow::Result<Vec<serde_json::Value>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, fill_id, fee_amount, fee_asset, currency, product, exchange_segment
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
            let fee_amount: Option<f64> = r.get(8)?;
            let fee_asset: Option<String> = r.get(9)?;
            let currency: Option<String> = r.get(10)?;
            let product: Option<String> = r.get(11)?;
            let exchange_segment: Option<String> = r.get(12)?;
            let filled_at_ms: i64 = DateTime::parse_from_rfc3339(&filled_at)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            let mut row = json!({
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
            });
            if let Some(amount) = fee_amount {
                row["feeAmount"] = json!(amount);
            }
            if let Some(asset) = fee_asset {
                row["feeAsset"] = json!(asset);
            }
            if let Some(ccy) = currency {
                row["currency"] = json!(ccy);
            }
            if let Some(product) = product {
                row["product"] = json!(product);
            }
            if let Some(segment) = exchange_segment {
                row["exchangeSegment"] = json!(segment);
            }
            Ok(row)
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
                    fill_id, trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, fee_amount, fee_asset, currency, product, exchange_segment, inserted_at_ms
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)"#,
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
                    fill.fee_amount,
                    fill.fee_asset.as_deref(),
                    fill.currency.as_deref(),
                    fill.product.as_deref(),
                    fill.exchange_segment.as_deref(),
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
                "SELECT filled_at_rfc3339 FROM recent_fills ORDER BY filled_at_rfc3339 DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;

        Ok(match v {
            Some(ref s) => Some(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc)),
            None => None,
        })
    }

    /// All fills in chronological order (round-trip engine input).
    pub fn fetch_all_fills(&self) -> anyhow::Result<Vec<BrokerFill>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT fill_id, trade_id, symbol, side, qty, price, filled_at_rfc3339, broker, fee_amount, fee_asset, currency, product, exchange_segment
             FROM recent_fills ORDER BY filled_at_rfc3339 ASC, fill_id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            let fill_id: String = r.get(0)?;
            let trade_id: String = r.get(1)?;
            let symbol: String = r.get(2)?;
            let side: String = r.get(3)?;
            let qty: f64 = r.get(4)?;
            let price: f64 = r.get(5)?;
            let filled_at: String = r.get(6)?;
            let broker: String = r.get(7)?;
            let fee_amount: Option<f64> = r.get(8)?;
            let fee_asset: Option<String> = r.get(9)?;
            let currency: Option<String> = r.get(10)?;
            let product: Option<String> = r.get(11)?;
            let exchange_segment: Option<String> = r.get(12)?;
            let filled_at = DateTime::parse_from_rfc3339(&filled_at)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(BrokerFill {
                fill_id,
                trade_id,
                symbol,
                side,
                qty,
                price,
                filled_at,
                broker,
                fee_amount,
                fee_asset,
                currency,
                product,
                exchange_segment,
                instrument_type: None,
                lot: None,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn recent_fills_round_trip_keeps_inr_cash_fields() {
        // I-N3
        let dir = std::env::temp_dir().join(format!("ta-recent-fills-inr-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("recent.db");
        let _ = std::fs::remove_file(&path);
        let store = RecentTradesStore::open(&path).expect("open");
        let fill = BrokerFill {
            fill_id: "FILL-1".into(),
            trade_id: "NSE998877".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 2500.0,
            filled_at: Utc.with_ymd_and_hms(2026, 7, 25, 8, 35, 0).unwrap(),
            broker: "kotak_neo".into(),
            fee_amount: None,
            fee_asset: None,
            currency: Some("INR".into()),
            product: Some("CNC".into()),
            exchange_segment: Some("nse_cm".into()),
            instrument_type: None,
            lot: None,
        };
        store.upsert_fill(&fill).expect("upsert");
        let loaded = store.fetch_all_fills().expect("fetch");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].currency.as_deref(), Some("INR"));
        assert_eq!(loaded[0].product.as_deref(), Some("CNC"));
        assert_eq!(loaded[0].exchange_segment.as_deref(), Some("nse_cm"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
