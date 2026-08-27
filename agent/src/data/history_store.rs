//! SQLite coverage for licensed Binance `historical_series`. Not TickBook. Not Yahoo.
//!
//! Coverage is what **we** stored (first/last open we wrote). Binance retention is
//! NOT SPECIFIED IN SOURCE — never emit `insufficient_retention` from a gap here.

use super::binance_klines::{HistoryCandle, HistorySeries};
use super::binance_public::normalize_quote_instrument;
use super::tick::Transport;
use anyhow::Context;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HistoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl HistoryStore {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create history db parent {}", parent.display()))?;
            }
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open history db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS history_candles (
  adapter_id TEXT NOT NULL,
  instrument_id TEXT NOT NULL,
  interval TEXT NOT NULL,
  open_time_ms INTEGER NOT NULL,
  close_time_ms INTEGER NOT NULL,
  open TEXT NOT NULL,
  high TEXT NOT NULL,
  low TEXT NOT NULL,
  close TEXT NOT NULL,
  volume TEXT NOT NULL,
  PRIMARY KEY (adapter_id, instrument_id, interval, open_time_ms)
);
CREATE TABLE IF NOT EXISTS history_coverage (
  adapter_id TEXT NOT NULL,
  instrument_id TEXT NOT NULL,
  interval TEXT NOT NULL,
  first_open_ms INTEGER NOT NULL,
  last_open_ms INTEGER NOT NULL,
  last_fetch_ms INTEGER NOT NULL,
  PRIMARY KEY (adapter_id, instrument_id, interval)
);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn upsert_series(&self, series: &HistorySeries) -> anyhow::Result<()> {
        if series.candles.is_empty() {
            return Ok(());
        }
        let adapter_id = series.adapter_id.trim();
        let instrument_id = normalize_quote_instrument(&series.instrument_id);
        let interval = series.interval.trim();
        let first_open_ms = series
            .candles
            .iter()
            .map(|c| c.open_time_ms)
            .min()
            .expect("non-empty candles");
        let last_open_ms = series
            .candles
            .iter()
            .map(|c| c.open_time_ms)
            .max()
            .expect("non-empty candles");
        let last_fetch_ms = chrono::Utc::now().timestamp_millis();

        let mut guard = self.conn.lock().expect("sqlite mutex poisoned");
        let tx = guard.transaction()?;
        tx.execute(
            r#"DELETE FROM history_candles
               WHERE adapter_id = ?1 AND instrument_id = ?2 AND interval = ?3"#,
            params![adapter_id, instrument_id, interval],
        )?;
        for candle in &series.candles {
            tx.execute(
                r#"INSERT INTO history_candles (
                    adapter_id, instrument_id, interval, open_time_ms,
                    close_time_ms, open, high, low, close, volume
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#,
                params![
                    adapter_id,
                    instrument_id,
                    interval,
                    candle.open_time_ms,
                    candle.close_time_ms,
                    candle.open.as_str(),
                    candle.high.as_str(),
                    candle.low.as_str(),
                    candle.close.as_str(),
                    candle.volume.as_str(),
                ],
            )?;
        }
        tx.execute(
            r#"INSERT INTO history_coverage (
                adapter_id, instrument_id, interval,
                first_open_ms, last_open_ms, last_fetch_ms
            ) VALUES (?1,?2,?3,?4,?5,?6)
            ON CONFLICT(adapter_id, instrument_id, interval) DO UPDATE SET
              first_open_ms = excluded.first_open_ms,
              last_open_ms = excluded.last_open_ms,
              last_fetch_ms = excluded.last_fetch_ms"#,
            params![
                adapter_id,
                instrument_id,
                interval,
                first_open_ms,
                last_open_ms,
                last_fetch_ms,
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_all_series(&self) -> anyhow::Result<Vec<HistorySeries>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            r#"SELECT adapter_id, instrument_id, interval, open_time_ms,
                      close_time_ms, open, high, low, close, volume
               FROM history_candles
               ORDER BY adapter_id, instrument_id, interval, open_time_ms"#,
        )?;
        let mut grouped: HashMap<(String, String, String), Vec<HistoryCandle>> = HashMap::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let adapter_id: String = row.get(0)?;
            let instrument_id: String = row.get(1)?;
            let interval: String = row.get(2)?;
            let candle = HistoryCandle {
                open_time_ms: row.get(3)?,
                close_time_ms: row.get(4)?,
                open: row.get(5)?,
                high: row.get(6)?,
                low: row.get(7)?,
                close: row.get(8)?,
                volume: row.get(9)?,
            };
            grouped
                .entry((adapter_id, instrument_id, interval))
                .or_default()
                .push(candle);
        }
        Ok(grouped
            .into_iter()
            .filter(|(_, candles)| !candles.is_empty())
            .map(
                |((adapter_id, instrument_id, interval), candles)| HistorySeries {
                    instrument_id,
                    adapter_id,
                    interval,
                    candles,
                    // Coverage schema does not persist transport. Licensed klines are REST.
                    transport: Transport::Rest,
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_klines::series_from_klines_json;

    const FIXTURE: &str = include_str!("../../fixtures/binance/klines.json");

    fn temp_db() -> (std::path::PathBuf, HistoryStore) {
        let path = std::env::temp_dir().join(format!(
            "rta-history-store-unit-{}.db",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        let store = HistoryStore::open(&path).expect("open");
        (path, store)
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[test]
    fn history_store_upsert_then_load_keeps_text_decimals_and_last_close() {
        let (path, store) = temp_db();
        let series =
            series_from_klines_json(FIXTURE, "BTCUSDT", "1m", Transport::Fixture).expect("fixture");
        store.upsert_series(&series).expect("upsert");

        let loaded = store.load_all_series().expect("load");
        assert_eq!(loaded.len(), 1);
        let row = &loaded[0];
        assert_eq!(row.adapter_id, series.adapter_id);
        assert_eq!(row.instrument_id, "btcusdt");
        assert_eq!(row.interval, "1m");
        assert_eq!(row.candles.len(), 2);
        assert_eq!(row.candles[0].open, "0.01634790");
        assert_eq!(
            row.candles.last().map(|c| c.close.as_str()),
            Some("0.01590000")
        );

        let conn = Connection::open(&path).expect("reopen");
        let (first, last): (i64, i64) = conn
            .query_row(
                "SELECT first_open_ms, last_open_ms FROM history_coverage",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("coverage");
        assert_eq!(first, 1_499_040_000_000);
        assert_eq!(last, 1_499_644_800_000);
        cleanup(&path);
    }

    #[test]
    fn history_store_empty_series_is_not_persisted() {
        let (path, store) = temp_db();
        let empty = HistorySeries {
            instrument_id: "btcusdt".into(),
            adapter_id: "binance_com".into(),
            interval: "1m".into(),
            candles: Vec::new(),
            transport: Transport::Fixture,
        };
        store.upsert_series(&empty).expect("skip empty");
        assert!(store.load_all_series().expect("load").is_empty());
        let conn = Connection::open(&path).expect("reopen");
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM history_candles", [], |r| r.get(0))
            .expect("count");
        assert_eq!(n, 0);
        cleanup(&path);
    }
}
