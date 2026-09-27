use crate::{StationRefreshError, StationTokenStore, UpstreamClient};
use anyhow::Context;
use rusqlite::{params, Connection};
use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

pub enum Fact {
    StationOnline,
    /// M1: Station-cited realized PnL for Console consume (A8 Bearer share-up).
    CitedPnl { payload: serde_json::Value },
}

impl Fact {
    fn signal_type(&self) -> &'static str {
        match self {
            Fact::StationOnline => "station_online",
            Fact::CitedPnl { .. } => crate::share_cited_pnl::SIGNAL_TYPE,
        }
    }

    fn payload_json(&self, event_id: &str) -> anyhow::Result<String> {
        match self {
            Fact::StationOnline => Ok(serde_json::to_string(&json!({
                "v": 1,
                "event_id": event_id,
            }))?),
            Fact::CitedPnl { payload } => {
                let mut body = payload.clone();
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("event_id".into(), json!(event_id));
                }
                Ok(serde_json::to_string(&body)?)
            }
        }
    }
}

pub enum EnqueueOutcome {
    Enqueued { id: String },
    Coalesced,
    SkippedNoJwt,
}

pub struct FactRow {
    pub id: String,
    pub signal_type: String,
    pub done_at_ms: Option<i64>,
    pub next_attempt_at_ms: i64,
    pub attempts: i64,
}

pub struct FactOutbox {
    conn: Arc<Mutex<Connection>>,
    upstream: Arc<UpstreamClient>,
    token_store: Option<Arc<dyn StationTokenStore>>,
    clock_ms: Option<Arc<AtomicI64>>,
}

impl FactOutbox {
    pub fn open(db_path: impl AsRef<Path>, upstream: Arc<UpstreamClient>) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path.as_ref())
            .with_context(|| format!("open fact outbox at {}", db_path.as_ref().display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS fact_outbox (
               id TEXT PRIMARY KEY,
               signal_type TEXT NOT NULL,
               payload_json TEXT NOT NULL,
               created_at_ms INTEGER NOT NULL,
               attempts INTEGER NOT NULL DEFAULT 0,
               next_attempt_at_ms INTEGER NOT NULL,
               done_at_ms INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_fact_outbox_drain
               ON fact_outbox(done_at_ms, next_attempt_at_ms);",
        )?;
        // Heartbeats are idempotent; a backlog of unsent ones is noise, not data.
        conn.execute(
            "DELETE FROM fact_outbox
             WHERE signal_type = 'station_online' AND done_at_ms IS NULL
               AND id NOT IN (
                 SELECT id FROM fact_outbox
                 WHERE signal_type = 'station_online' AND done_at_ms IS NULL
                 ORDER BY created_at_ms DESC LIMIT 1
               )",
            [],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            upstream,
            token_store: None,
            clock_ms: None,
        })
    }

    pub fn with_token_store(mut self, store: Arc<dyn StationTokenStore>) -> Self {
        self.token_store = Some(store);
        self
    }

    pub fn with_clock(mut self, clock_ms: Arc<AtomicI64>) -> Self {
        self.clock_ms = Some(clock_ms);
        self
    }

    fn now_ms(&self) -> i64 {
        if let Some(clock) = &self.clock_ms {
            return clock.load(Ordering::SeqCst);
        }
        chrono::Utc::now().timestamp_millis()
    }

    fn has_station_jwt(&self) -> bool {
        if let Some(store) = &self.token_store {
            return store.load().ok().flatten().is_some();
        }
        self.upstream.brain_authorization_header().is_ok()
    }

    pub fn enqueue(&self, fact: Fact) -> anyhow::Result<EnqueueOutcome> {
        if !self.has_station_jwt() {
            return Ok(EnqueueOutcome::SkippedNoJwt);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = self.now_ms();
        let payload_json = fact.payload_json(&id)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(fact, Fact::StationOnline)
            && (last_online_send_is_fresh(&conn, now)? || online_send_pending(&conn)?)
        {
            return Ok(EnqueueOutcome::Coalesced);
        }
        if let Fact::CitedPnl { payload } = &fact {
            if let Some(existing_id) = pending_cited_pnl_id(&conn, payload)? {
                conn.execute(
                    "UPDATE fact_outbox
                     SET payload_json = ?, created_at_ms = ?, next_attempt_at_ms = ?
                     WHERE id = ?",
                    params![payload_json, now, now, existing_id],
                )?;
                return Ok(EnqueueOutcome::Coalesced);
            }
        }
        conn.execute(
            "INSERT INTO fact_outbox
             (id, signal_type, payload_json, created_at_ms, attempts, next_attempt_at_ms, done_at_ms)
             VALUES (?, ?, ?, ?, 0, ?, NULL)",
            params![id, fact.signal_type(), payload_json, now, now],
        )?;
        Ok(EnqueueOutcome::Enqueued { id })
    }

    /// M1 share-up: enqueue any v2 cited-PnL envelope when JWT is present.
    pub fn enqueue_cited_pnl(&self, payload: serde_json::Value) -> anyhow::Result<EnqueueOutcome> {
        if !crate::m1_envelope::envelope_has_cited_trips(&payload) {
            return Ok(EnqueueOutcome::Coalesced);
        }
        self.enqueue(Fact::CitedPnl { payload })
    }

    /// M1 share-up: cited INR cash WAC (per SHIPPING cash book).
    pub fn enqueue_cited_cash_pnl(
        &self,
        book_id: &str,
        trips: &[crate::inr_cash_wac::InrCashRoundTrip],
    ) -> anyhow::Result<EnqueueOutcome> {
        if trips.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload =
            crate::share_cited_pnl::cited_cash_pnl_payload_for_book(book_id, trips);
        self.enqueue_cited_pnl(payload)
    }

    /// M1 share-up: cited NFO realized INR (`*-nse-nfo`).
    pub fn enqueue_cited_nfo_pnl(
        &self,
        book_id: &str,
        trips: &[crate::nfo_realized_pnl::NfoRoundTrip],
    ) -> anyhow::Result<EnqueueOutcome> {
        if trips.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::m1_envelope::cited_nfo_payload(
            book_id,
            crate::nfo_realized_pnl::OWNER_PATH,
            trips,
        );
        self.enqueue_cited_pnl(payload)
    }

    /// M1 share-up: cited COM USD spot WAC.
    pub fn enqueue_cited_crypto_spot_pnl(
        &self,
        book_id: &str,
        trips: &[crate::round_trip_engine::RoundTrip],
    ) -> anyhow::Result<EnqueueOutcome> {
        if trips.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::m1_envelope::cited_crypto_spot_payload(book_id, trips);
        self.enqueue_cited_pnl(payload)
    }

    /// M1 share-up: Binance USDM REALIZED_PNL income rows.
    pub fn enqueue_cited_usdm_income(
        &self,
        book_id: &str,
        rows: &[crate::usdm_realized_pnl::UsdmIncomeRow],
    ) -> anyhow::Result<EnqueueOutcome> {
        if rows.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::m1_envelope::cited_usdm_income_payload(book_id, rows);
        self.enqueue_cited_pnl(payload)
    }

    /// M1 share-up: Kotak CDS realized INR.
    pub fn enqueue_cited_fx_cds_pnl(
        &self,
        book_id: &str,
        trips: &[crate::fx_cds_realized_pnl::FxCdsRoundTrip],
    ) -> anyhow::Result<EnqueueOutcome> {
        if trips.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::m1_envelope::cited_fx_cds_payload(
            book_id,
            crate::fx_cds_realized_pnl::OWNER_PATH,
            trips,
        );
        self.enqueue_cited_pnl(payload)
    }

    /// M1 share-up: Kotak MCX commodity future realized INR.
    pub fn enqueue_cited_mcx_pnl(
        &self,
        book_id: &str,
        trips: &[crate::mcx_realized_pnl::McxRoundTrip],
    ) -> anyhow::Result<EnqueueOutcome> {
        if trips.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::m1_envelope::cited_mcx_payload(
            book_id,
            crate::mcx_realized_pnl::OWNER_PATH,
            trips,
        );
        self.enqueue_cited_pnl(payload)
    }

    /// M1 share-up: Binance Coin-M REALIZED_PNL income rows.
    pub fn enqueue_cited_coinm_income(
        &self,
        book_id: &str,
        rows: &[crate::coinm_realized_pnl::CoinmIncomeRow],
    ) -> anyhow::Result<EnqueueOutcome> {
        if rows.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::m1_envelope::cited_coinm_income_payload(book_id, rows);
        self.enqueue_cited_pnl(payload)
    }

    /// Enqueue a heartbeat (if allowed) and flush any due outbox rows — e.g. right after sign-in.
    pub async fn flush_station_presence(self: &Arc<Self>) -> anyhow::Result<()> {
        let worker = Arc::clone(self);
        let _ = tokio::task::spawn_blocking(move || worker.enqueue(Fact::StationOnline)).await;
        self.drain().await
    }

    pub async fn drain(&self) -> anyhow::Result<()> {
        let due = self.due_ids()?;
        if due.is_empty() {
            return Ok(());
        }
        match self.upstream.ensure_fresh_station_access(None).await {
            Err(StationRefreshError::Revoked) => return Ok(()),
            // Transient refresh backoff must not wedge the outbox — POST with the current JWT.
            Ok(()) | Err(StationRefreshError::Transient(_)) => {}
        }
        let mut refreshed_after_401 = false;
        for id in due {
            let mut outcome = self.send_one(&id).await?;
            if let SendOutcome::Unauthorized { rejected_access } = &outcome {
                if !refreshed_after_401 {
                    refreshed_after_401 = true;
                    if self
                        .upstream
                        .ensure_fresh_station_access(Some(rejected_access))
                        .await
                        .is_ok()
                    {
                        outcome = self.send_one(&id).await?;
                    }
                }
            }
            // One dead token fails every row; stop instead of replaying the queue at Console.
            if matches!(outcome, SendOutcome::Unauthorized { .. }) {
                self.defer(&id, AUTH_BACKOFF_MS)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn defer(&self, id: &str, delay_ms: i64) -> anyhow::Result<()> {
        let now = self.now_ms();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE fact_outbox SET next_attempt_at_ms = ? WHERE id = ?",
            params![now.saturating_add(delay_ms), id],
        )?;
        Ok(())
    }

    pub fn row(&self, id: &str) -> anyhow::Result<Option<FactRow>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, signal_type, done_at_ms, next_attempt_at_ms, attempts
             FROM fact_outbox WHERE id = ?",
        )?;
        let mut rows = stmt.query([id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(FactRow {
            id: row.get(0)?,
            signal_type: row.get(1)?,
            done_at_ms: row.get(2)?,
            next_attempt_at_ms: row.get(3)?,
            attempts: row.get(4)?,
        }))
    }

    fn due_ids(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let now = self.now_ms();
        let mut stmt = conn.prepare(
            "SELECT id FROM fact_outbox
             WHERE done_at_ms IS NULL AND next_attempt_at_ms <= ?
             ORDER BY created_at_ms ASC",
        )?;
        let ids = stmt
            .query_map([now], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    async fn send_one(&self, id: &str) -> anyhow::Result<SendOutcome> {
        let (signal_type, payload_json) = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.query_row(
                "SELECT signal_type, payload_json FROM fact_outbox WHERE id = ?",
                [id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?
        };
        let payload: serde_json::Value = serde_json::from_str(&payload_json)?;
        let body = json!({
            "events": [{
                "signal_type": signal_type,
                "value": payload,
                "session_id": null
            }]
        });
        let url = format!("{}/api/daemon/events", self.upstream.config.base_url);
        let auth = self.upstream.brain_authorization_header()?;
        let resp = self
            .upstream
            .http
            .post(url)
            .header(reqwest::header::AUTHORIZATION, auth.as_str())
            .json(&body)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let retry_after_ms = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(|secs| secs.saturating_mul(1000));
        if status == 401 {
            let rejected_access = auth.strip_prefix("Bearer ").unwrap_or(&auth).to_string();
            return Ok(SendOutcome::Unauthorized { rejected_access });
        }

        let now = self.now_ms();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if status == 200 {
            conn.execute(
                "UPDATE fact_outbox SET done_at_ms = ? WHERE id = ?",
                params![now, id],
            )?;
            return Ok(SendOutcome::Done);
        }

        let attempts: i64 = conn.query_row(
            "SELECT attempts FROM fact_outbox WHERE id = ?",
            [id],
            |row| row.get(0),
        )?;
        let next_attempts = attempts + 1;
        let delay_ms = if status >= 500 {
            retry_delay_ms(next_attempts)
        } else {
            // 4xx will not fix itself on a tight loop; honor Retry-After when Console sends it.
            retry_after_ms
                .unwrap_or_else(|| retry_delay_ms(next_attempts))
                .max(CLIENT_ERROR_MIN_BACKOFF_MS)
                .min(AUTH_BACKOFF_MS)
        };
        conn.execute(
            "UPDATE fact_outbox SET attempts = ?, next_attempt_at_ms = ? WHERE id = ?",
            params![next_attempts, now.saturating_add(delay_ms), id],
        )?;
        Ok(SendOutcome::Retry)
    }
}

enum SendOutcome {
    Done,
    Retry,
    Unauthorized { rejected_access: String },
}

const BASE_BACKOFF_MS: i64 = 300;
const MAX_BACKOFF_MS: i64 = 30_000;
const CLIENT_ERROR_MIN_BACKOFF_MS: i64 = 5_000;
const AUTH_BACKOFF_MS: i64 = 5 * 60_000;
const COALESCE_MS: i64 = 15_000;

fn online_send_pending(conn: &Connection) -> anyhow::Result<bool> {
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM fact_outbox
         WHERE signal_type = 'station_online' AND done_at_ms IS NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(pending > 0)
}

fn last_online_send_is_fresh(conn: &Connection, now: i64) -> anyhow::Result<bool> {
    let last_done: Option<i64> = conn.query_row(
        "SELECT MAX(done_at_ms) FROM fact_outbox
         WHERE signal_type = 'station_online' AND done_at_ms IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(last_done.is_some_and(|ts| now.saturating_sub(ts) < COALESCE_MS))
}

/// One pending cited-PnL row per (book_id, cite_kind) — Today refresh replaces payload instead of flooding the outbox.
fn pending_cited_pnl_id(
    conn: &Connection,
    payload: &serde_json::Value,
) -> anyhow::Result<Option<String>> {
    let book_id = payload
        .get("book_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if book_id.is_empty() {
        return Ok(None);
    }
    let cite_kind = payload
        .get("cite_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("inr_cash_wac");
    let mut stmt = conn.prepare(
        "SELECT id FROM fact_outbox
         WHERE signal_type = 'station_cited_pnl'
           AND done_at_ms IS NULL
           AND json_extract(payload_json, '$.book_id') = ?1
           AND COALESCE(json_extract(payload_json, '$.cite_kind'), 'inr_cash_wac') = ?2
         ORDER BY created_at_ms DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query(params![book_id, cite_kind])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(row.get(0)?))
}

fn retry_delay_ms(attempts: i64) -> i64 {
    let exp = (attempts.saturating_sub(1) as u32).min(16);
    BASE_BACKOFF_MS
        .saturating_mul(1_i64 << exp)
        .min(MAX_BACKOFF_MS)
}

#[cfg(test)]
mod coalesce_tests {
    use super::*;
    use crate::share_cited_pnl::SIGNAL_TYPE;
    use crate::{MemoryStationTokenStore, StationTokenStore, StationTokens, UpstreamClient, UpstreamConfig};

    #[test]
    fn cited_pnl_coalesces_pending_same_book_and_kind() {
        let db = std::env::temp_dir().join(format!("fact-coalesce-{}.db", uuid::Uuid::new_v4()));
        let upstream = std::sync::Arc::new(
            UpstreamClient::new(UpstreamConfig {
                base_url: "http://127.0.0.1:9".into(),
                daemon_secret: "test".into(),
            })
            .expect("upstream"),
        );
        let tokens = std::sync::Arc::new(MemoryStationTokenStore::default());
        tokens
            .save(&StationTokens {
                access_token: "jwt".into(),
                refresh_token: "r".into(),
                expires_in: 3600,
                refresh_expires_in: None,
            })
            .expect("save jwt");
        let outbox = FactOutbox::open(&db, upstream)
            .expect("open")
            .with_token_store(tokens);
        let payload_a = serde_json::json!({
            "v": 2,
            "cite_kind": "nfo_realized_inr",
            "book_id": "dhan-nse-nfo",
            "currency": "INR",
            "trips": [{"trip_id": "a", "symbol": "X", "closed_at": "2026-01-01T00:00:00Z", "realized_pnl_inr": 1.0, "book_id": "dhan-nse-nfo", "currency": "INR"}]
        });
        let payload_b = serde_json::json!({
            "v": 2,
            "cite_kind": "nfo_realized_inr",
            "book_id": "dhan-nse-nfo",
            "currency": "INR",
            "trips": [{"trip_id": "b", "symbol": "Y", "closed_at": "2026-01-01T00:00:00Z", "realized_pnl_inr": 2.0, "book_id": "dhan-nse-nfo", "currency": "INR"}]
        });
        assert!(matches!(
            outbox.enqueue_cited_pnl(payload_a).expect("first"),
            EnqueueOutcome::Enqueued { .. }
        ));
        assert!(matches!(
            outbox.enqueue_cited_pnl(payload_b).expect("second"),
            EnqueueOutcome::Coalesced
        ));
        let conn = rusqlite::Connection::open(&db).expect("conn");
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fact_outbox WHERE signal_type = ? AND done_at_ms IS NULL",
                [SIGNAL_TYPE],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(pending, 1);
        let body: String = conn
            .query_row(
                "SELECT payload_json FROM fact_outbox WHERE signal_type = ? LIMIT 1",
                [SIGNAL_TYPE],
                |r| r.get(0),
            )
            .expect("body");
        assert!(body.contains("\"trip_id\":\"b\""));
    }
}
