use anyhow::Context;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use std::io::Cursor;
use std::sync::Mutex;

use super::ticker::normalize_broker_ticker;

const ZERODHA_INSTRUMENTS_URL: &str = "https://api.kite.trade/instruments";
const REFRESH_STALE_SECS: i64 = 43_200;
const INSERT_BATCH_SIZE: usize = 500;

const SCHEMA_SQL: &str = include_str!("schema.sql");

#[derive(serde::Serialize)]
pub struct InstrumentResult {
    pub trading_symbol: String,
    pub name: String,
    pub exchange: String,
    pub segment: String,
    pub instrument_token: i64,
    pub last_price: f64,
}

pub struct InstrumentStore {
    conn: Mutex<Connection>,
}

impl InstrumentStore {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("open instruments db at {db_path}"))?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn needs_refresh(&self) -> anyhow::Result<bool> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let count: i64 =
            guard.query_row("SELECT COUNT(*) FROM instruments", [], |r| r.get(0))?;
        if count == 0 {
            return Ok(true);
        }
        let max_updated: Option<i64> = guard
            .query_row("SELECT MAX(updated_at) FROM instruments", [], |r| r.get(0))
            .ok();
        let now = Utc::now().timestamp();
        Ok(max_updated.unwrap_or(0) < now - REFRESH_STALE_SECS)
    }

    pub async fn refresh_from_zerodha(&self) -> anyhow::Result<usize> {
        let client = reqwest::Client::builder()
            .gzip(true)
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .context("build zerodha instruments HTTP client")?;
        let response = client
            .get(ZERODHA_INSTRUMENTS_URL)
            .send()
            .await
            .context("fetch zerodha instruments CSV")?;
        if !response.status().is_success() {
            anyhow::bail!(
                "zerodha instruments HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }
        let bytes = response
            .bytes()
            .await
            .context("read zerodha instruments body")?;
        self.load_csv(&bytes)
    }

    fn load_csv(&self, bytes: &[u8]) -> anyhow::Result<usize> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let updated_at = Utc::now().timestamp();
        let tx = guard
            .unchecked_transaction()
            .context("begin instruments refresh transaction")?;

        tx.execute("DELETE FROM instruments", [])?;
        tx.execute("INSERT INTO instruments_fts(instruments_fts) VALUES('rebuild')", [])?;

        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(Cursor::new(bytes));
        let mut insert = tx.prepare(
            "INSERT INTO instruments (
                instrument_token, trading_symbol, name, exchange, segment, instrument_type, expiry, last_price, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut batch: Vec<ZerodhaRow> = Vec::with_capacity(INSERT_BATCH_SIZE);
        let mut total = 0usize;

        for row in reader.deserialize::<ZerodhaRow>() {
            batch.push(row.context("parse zerodha CSV row")?);
            if batch.len() >= INSERT_BATCH_SIZE {
                total += flush_batch(&mut insert, &batch, updated_at)?;
                batch.clear();
            }
        }
        if !batch.is_empty() {
            total += flush_batch(&mut insert, &batch, updated_at)?;
        }

        drop(insert);
        tx.commit().context("commit instruments refresh transaction")?;
        Ok(total)
    }

    pub fn search(&self, q: &str, limit: usize) -> anyhow::Result<Vec<InstrumentResult>> {
        if q.len() < 2 {
            return Ok(Vec::new());
        }
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            "WITH matched AS (
               SELECT i.trading_symbol, i.name, i.exchange, i.segment, i.instrument_token, i.last_price,
                 CASE i.exchange
                   WHEN 'NSE' THEN 0
                   WHEN 'NFO' THEN 1
                   WHEN 'BFO' THEN 2
                   WHEN 'MCX' THEN 3
                   WHEN 'BSE' THEN 4
                   ELSE 5
                 END AS exchange_rank
               FROM instruments_fts f
               JOIN instruments i ON i.rowid = f.rowid
               WHERE f.instruments_fts MATCH ?1 || '*'
             ),
             best AS (
               SELECT trading_symbol, MIN(exchange_rank) AS min_rank
               FROM matched
               GROUP BY trading_symbol
             )
             SELECT m.trading_symbol, m.name, m.exchange, m.segment, m.instrument_token, m.last_price
             FROM matched m
             JOIN best b
               ON b.trading_symbol = m.trading_symbol AND b.min_rank = m.exchange_rank
             ORDER BY length(m.trading_symbol), m.trading_symbol
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![q, limit as i64], |r| {
            Ok(InstrumentResult {
                trading_symbol: r.get(0)?,
                name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                exchange: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                segment: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                instrument_token: r.get(4)?,
                last_price: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            let mut item = row?;
            if let Some(normalized) = normalize_broker_ticker(&item.trading_symbol) {
                item.trading_symbol = normalized;
            } else {
                continue;
            }
            if out.iter().any(|existing: &InstrumentResult| {
                existing.trading_symbol == item.trading_symbol
            }) {
                continue;
            }
            out.push(item);
        }
        Ok(out)
    }

    pub fn get_last_price(&self, trading_symbol: &str) -> anyhow::Result<Option<f64>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        guard
            .query_row(
                "SELECT last_price FROM instruments
                 WHERE trading_symbol = ?1 AND last_price > 0
                 ORDER BY CASE exchange WHEN 'NSE' THEN 0 WHEN 'NFO' THEN 1 ELSE 2 END
                 LIMIT 1",
                params![trading_symbol],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn upsert_last_price(&self, trading_symbol: &str, price: f64) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let updated_at = Utc::now().timestamp();
        guard.execute(
            "UPDATE instruments SET last_price = ?1, updated_at = ?2 WHERE trading_symbol = ?3",
            params![price, updated_at, trading_symbol],
        )?;
        Ok(())
    }
}

fn flush_batch(
    insert: &mut rusqlite::Statement<'_>,
    batch: &[ZerodhaRow],
    updated_at: i64,
) -> anyhow::Result<usize> {
    for row in batch {
        insert.execute(params![
            row.instrument_token,
            row.tradingsymbol,
            row.name,
            row.exchange,
            row.segment,
            row.instrument_type,
            row.expiry,
            row.last_price,
            updated_at,
        ])?;
    }
    Ok(batch.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_store() -> InstrumentStore {
        let dir = std::env::temp_dir().join(format!(
            "ta-instruments-test-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("instruments.db");
        let _ = std::fs::remove_file(&path);
        let store = InstrumentStore::new(path.to_str().unwrap()).expect("open test db");
        {
            let guard = store.conn.lock().expect("sqlite mutex poisoned");
            let updated_at = Utc::now().timestamp();
            guard
                .execute(
                    "INSERT INTO instruments (
                        instrument_token, trading_symbol, name, exchange, segment, instrument_type, expiry, last_price, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        1_i64,
                        "RELIANCE-EQ",
                        "Reliance Industries Ltd",
                        "NSE",
                        "NSE",
                        "EQ",
                        "",
                        2500.0_f64,
                        updated_at,
                    ],
                )
                .expect("seed reliance");
            guard
                .execute(
                    "INSERT INTO instruments_fts(rowid, trading_symbol, name)
                     SELECT rowid, trading_symbol, name FROM instruments",
                    [],
                )
                .expect("rebuild fts");
        }
        store
    }

    #[test]
    fn search_returns_normalized_broker_ticker() {
        let store = seed_store();
        let results = store.search("REL", 5).expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trading_symbol, "RELIANCE");
    }
}

#[derive(Debug, Deserialize)]
struct ZerodhaRow {
    instrument_token: i64,
    tradingsymbol: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    exchange: String,
    #[serde(default)]
    segment: String,
    #[serde(default)]
    instrument_type: String,
    #[serde(default)]
    expiry: String,
    #[serde(default)]
    last_price: f64,
}
