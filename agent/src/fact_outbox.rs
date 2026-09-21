use crate::{StationTokenStore, UpstreamClient};
use anyhow::Context;
use rusqlite::{params, Connection};
use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

pub enum Fact {
    StationOnline,
    /// M1: Station-cited cash realized PnL for Console consume (A8 Bearer share-up).
    CitedCashPnl { payload: serde_json::Value },
}

impl Fact {
    fn signal_type(&self) -> &'static str {
        match self {
            Fact::StationOnline => "station_online",
            Fact::CitedCashPnl { .. } => crate::share_cited_pnl::SIGNAL_TYPE,
        }
    }

    fn payload_json(&self, event_id: &str) -> anyhow::Result<String> {
        match self {
            Fact::StationOnline => Ok(serde_json::to_string(&json!({
                "v": 1,
                "event_id": event_id,
            }))?),
            Fact::CitedCashPnl { payload } => {
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
        if matches!(fact, Fact::StationOnline) && last_online_send_is_fresh(&conn, now)? {
            return Ok(EnqueueOutcome::Coalesced);
        }
        conn.execute(
            "INSERT INTO fact_outbox
             (id, signal_type, payload_json, created_at_ms, attempts, next_attempt_at_ms, done_at_ms)
             VALUES (?, ?, ?, ?, 0, ?, NULL)",
            params![id, fact.signal_type(), payload_json, now, now],
        )?;
        Ok(EnqueueOutcome::Enqueued { id })
    }

    /// M1 share-up: enqueue cited INR cash trips when JWT is present.
    pub fn enqueue_cited_cash_pnl(
        &self,
        trips: &[crate::inr_cash_wac::InrCashRoundTrip],
    ) -> anyhow::Result<EnqueueOutcome> {
        if trips.is_empty() {
            return Ok(EnqueueOutcome::Coalesced);
        }
        let payload = crate::share_cited_pnl::cited_cash_pnl_payload(trips);
        let trips_arr = payload
            .get("trips")
            .and_then(|t| t.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if trips_arr == 0 {
            return Ok(EnqueueOutcome::Coalesced);
        }
        self.enqueue(Fact::CitedCashPnl { payload })
    }

    pub async fn drain(&self) -> anyhow::Result<()> {
        let due = self.due_ids()?;
        for id in due {
            self.send_one(&id).await?;
        }
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

    async fn send_one(&self, id: &str) -> anyhow::Result<()> {
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
        let req = self
            .upstream
            .authorize_brain(self.upstream.http.post(url).json(&body))?;
        let resp = req.send().await?;
        let status = resp.status().as_u16();
        let now = self.now_ms();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if status == 200 {
            conn.execute(
                "UPDATE fact_outbox SET done_at_ms = ? WHERE id = ?",
                params![now, id],
            )?;
        } else if status >= 500 {
            let attempts: i64 = conn.query_row(
                "SELECT attempts FROM fact_outbox WHERE id = ?",
                [id],
                |row| row.get(0),
            )?;
            let next_attempts = attempts + 1;
            let delay_ms = retry_delay_ms(next_attempts);
            conn.execute(
                "UPDATE fact_outbox SET attempts = ?, next_attempt_at_ms = ? WHERE id = ?",
                params![next_attempts, now.saturating_add(delay_ms), id],
            )?;
        }
        Ok(())
    }
}

const BASE_BACKOFF_MS: i64 = 300;
const MAX_BACKOFF_MS: i64 = 30_000;
const COALESCE_MS: i64 = 15_000;

fn last_online_send_is_fresh(conn: &Connection, now: i64) -> anyhow::Result<bool> {
    let last_done: Option<i64> = conn.query_row(
        "SELECT MAX(done_at_ms) FROM fact_outbox
         WHERE signal_type = 'station_online' AND done_at_ms IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(last_done.is_some_and(|ts| now.saturating_sub(ts) < COALESCE_MS))
}

fn retry_delay_ms(attempts: i64) -> i64 {
    let exp = (attempts.saturating_sub(1) as u32).min(16);
    BASE_BACKOFF_MS
        .saturating_mul(1_i64 << exp)
        .min(MAX_BACKOFF_MS)
}
