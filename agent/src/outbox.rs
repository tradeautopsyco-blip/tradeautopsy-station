use crate::UpstreamClient;
use anyhow::Context;
use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

const STATE_ENQUEUED: &str = "ENQUEUED";
const STATE_INFLIGHT: &str = "INFLIGHT";
const STATE_ACKED: &str = "ACKED";
const STATE_DEAD_LETTER: &str = "DEAD_LETTER";

#[derive(Clone)]
pub struct OutboxConfig {
    pub db_path: PathBuf,
    pub worker_count: usize,
    pub max_attempts: u32,
    pub base_backoff_ms: i64,
    pub max_backoff_ms: i64,
}

impl OutboxConfig {
    pub fn from_env() -> Self {
        let db_path = std::env::var("AGENT_OUTBOX_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("tradeautopsy-agent-outbox.db");
                p
            });
        let worker_count = std::env::var("AGENT_OUTBOX_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        let max_attempts = std::env::var("AGENT_OUTBOX_MAX_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(5)
            .max(1);
        let base_backoff_ms = std::env::var("AGENT_OUTBOX_BASE_BACKOFF_MS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(300);
        let max_backoff_ms = std::env::var("AGENT_OUTBOX_MAX_BACKOFF_MS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(10_000);
        Self {
            db_path,
            worker_count,
            max_attempts,
            base_backoff_ms,
            max_backoff_ms,
        }
    }
}

#[derive(Clone)]
pub struct CaptureOutbox {
    conn: Arc<Mutex<Connection>>,
    upstream: Arc<UpstreamClient>,
    config: OutboxConfig,
    notify: Arc<Notify>,
}

struct OutboxRow {
    id: i64,
    request_id: String,
    user_id: String,
    payload_json: String,
    attempts: u32,
}

#[derive(Debug)]
pub enum ProcessNowResult {
    Acked { response_body: String },
    Queued,
    DeadLetter { reason: String },
}

enum AttemptOutcome {
    Acked { response_body: String },
    Retry { reason: String },
    DeadLetter { reason: String },
}

#[derive(Debug, Serialize)]
pub struct OutboxCounts {
    pub enqueued: u64,
    pub inflight: u64,
    pub acked: u64,
    pub dead_letter: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadLetterStatusItem {
    pub outbox_id: i64,
    pub request_id: String,
    pub attempts: u64,
    pub reason: String,
    pub idempotency_key: Option<String>,
    pub draft_text: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct OutboxStatusSnapshot {
    pub counts: OutboxCounts,
    pub dead_letters: Vec<DeadLetterStatusItem>,
}

impl CaptureOutbox {
    pub fn new(config: OutboxConfig, upstream: Arc<UpstreamClient>) -> anyhow::Result<Self> {
        let conn = Connection::open(&config.db_path)
            .with_context(|| format!("open sqlite outbox at {}", config.db_path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS capture_outbox (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               request_id TEXT NOT NULL,
               user_id TEXT NOT NULL,
               payload_json TEXT NOT NULL,
               state TEXT NOT NULL,
               attempts INTEGER NOT NULL DEFAULT 0,
               next_attempt_at_ms INTEGER NOT NULL,
               last_error TEXT,
               response_json TEXT,
               created_at_ms INTEGER NOT NULL,
               updated_at_ms INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_capture_outbox_drain
               ON capture_outbox(state, next_attempt_at_ms, id);",
        )?;
        let now = now_ms();
        conn.execute(
            "UPDATE capture_outbox
             SET state = ?, next_attempt_at_ms = ?, updated_at_ms = ?
             WHERE state = ?",
            params![STATE_ENQUEUED, now, now, STATE_INFLIGHT],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            upstream,
            config,
            notify: Arc::new(Notify::new()),
        })
    }

    pub fn worker_count(&self) -> usize {
        self.config.worker_count
    }

    pub async fn enqueue_capture(
        &self,
        user_id: &str,
        request_id: &str,
        payload: Value,
    ) -> anyhow::Result<i64> {
        let payload_json = serde_json::to_string(&payload)?;
        let now = now_ms();
        let id = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "INSERT INTO capture_outbox
                 (request_id, user_id, payload_json, state, attempts, next_attempt_at_ms, last_error, response_json, created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, ?, 0, ?, NULL, NULL, ?, ?)",
                params![request_id, user_id, payload_json, STATE_ENQUEUED, now, now, now],
            )?;
            conn.last_insert_rowid()
        };
        self.notify.notify_waiters();
        Ok(id)
    }

    pub async fn process_now(&self, id: i64) -> anyhow::Result<ProcessNowResult> {
        let Some(mut row) = self.claim_specific(id)? else {
            return Ok(ProcessNowResult::Queued);
        };
        let outcome = self.send_attempt(&mut row).await;
        self.apply_outcome(row, outcome)?;
        self.current_result(id)
    }

    pub async fn run_worker_loop(self: Arc<Self>) {
        loop {
            match self.next_due_id() {
                Ok(Some(id)) => {
                    let _ = self.process_now(id).await;
                }
                Ok(None) => {
                    tokio::select! {
                        _ = self.notify.notified() => {}
                        _ = tokio::time::sleep(Duration::from_millis(250)) => {}
                    }
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }
    }

    fn next_due_id(&self) -> anyhow::Result<Option<i64>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        let mut stmt = conn.prepare(
            "SELECT id, next_attempt_at_ms
             FROM capture_outbox
             WHERE state = ?
             ORDER BY id ASC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![STATE_ENQUEUED])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let id: i64 = row.get(0)?;
        let next_attempt_at: i64 = row.get(1)?;
        if next_attempt_at <= now {
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    fn current_result(&self, id: i64) -> anyhow::Result<ProcessNowResult> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let (state, response_json, last_error): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT state, response_json, last_error FROM capture_outbox WHERE id = ?",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        if state == STATE_ACKED {
            Ok(ProcessNowResult::Acked {
                response_body: response_json.unwrap_or_else(|| "{}".to_string()),
            })
        } else if state == STATE_DEAD_LETTER {
            Ok(ProcessNowResult::DeadLetter {
                reason: last_error.unwrap_or_else(|| "dead-letter".to_string()),
            })
        } else {
            Ok(ProcessNowResult::Queued)
        }
    }

    fn claim_specific(&self, id: i64) -> anyhow::Result<Option<OutboxRow>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let candidate = {
            let mut stmt = conn.prepare(
                "SELECT id, request_id, user_id, payload_json, attempts
                 FROM capture_outbox
                 WHERE id = ? AND state = ?",
            )?;
            let mut rows = stmt.query(params![id, STATE_ENQUEUED])?;
            if let Some(row) = rows.next()? {
                Some(OutboxRow {
                    id: row.get(0)?,
                    request_id: row.get(1)?,
                    user_id: row.get(2)?,
                    payload_json: row.get(3)?,
                    attempts: row.get::<_, i64>(4)? as u32,
                })
            } else {
                None
            }
        };
        let Some(outbox_row) = candidate else {
            return Ok(None);
        };
        let now = now_ms();
        conn.execute(
            "UPDATE capture_outbox SET state = ?, updated_at_ms = ? WHERE id = ?",
            params![STATE_INFLIGHT, now, id],
        )?;
        Ok(Some(outbox_row))
    }

    async fn send_attempt(&self, row: &mut OutboxRow) -> AttemptOutcome {
        let payload: Value = match serde_json::from_str(&row.payload_json) {
            Ok(v) => v,
            Err(err) => {
                return AttemptOutcome::DeadLetter {
                    reason: format!("invalid payload_json: {err}"),
                }
            }
        };
        let request_id = if row.attempts == 0 {
            row.request_id.clone()
        } else {
            ulid::Ulid::new().to_string()
        };
        let url = format!(
            "{}/api/daemon/journal/toolbar-capture/accept",
            self.upstream.config.base_url
        );
        // A8 IV — Console identity is Station Bearer only; wire user_id stays local.
        let response = match self.upstream.authorize_brain(
            self.upstream
                .http
                .post(url)
                .header("x-request-id", request_id)
                .json(&payload),
        ) {
            Ok(req) => req.send().await,
            Err(err) => {
                return AttemptOutcome::Retry {
                    reason: format!("station bearer missing: {err}"),
                }
            }
        };
        let resp = match response {
            Ok(v) => v,
            Err(err) => {
                return AttemptOutcome::Retry {
                    reason: format!("network: {err}"),
                }
            }
        };

        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_else(|_| "{}".to_string());
        if status == 200 {
            if let Ok(json_body) = serde_json::from_str::<Value>(&text) {
                if json_body["success"] == Value::Bool(true) {
                    let out_status = json_body["data"]["status"].as_str();
                    if matches!(out_status, Some("accepted" | "duplicate")) {
                        return AttemptOutcome::Acked {
                            response_body: text,
                        };
                    }
                }
            }
            return AttemptOutcome::Retry {
                reason: "unexpected 200 body".to_string(),
            };
        }

        if status == 429 || status >= 500 {
            return AttemptOutcome::Retry {
                reason: format!("http {status}"),
            };
        }

        if status == 400 {
            if let Ok(json_body) = serde_json::from_str::<Value>(&text) {
                let code = json_body["error"]["code"].as_str().unwrap_or("");
                if code == "VALIDATION_ERROR" {
                    return AttemptOutcome::DeadLetter {
                        reason: "validation".to_string(),
                    };
                }
            }
            return AttemptOutcome::DeadLetter {
                reason: "validation".to_string(),
            };
        }

        AttemptOutcome::DeadLetter {
            reason: format!("http {status}"),
        }
    }

    fn apply_outcome(&self, row: OutboxRow, outcome: AttemptOutcome) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        match outcome {
            AttemptOutcome::Acked { response_body } => {
                conn.execute(
                    "UPDATE capture_outbox
                     SET state = ?, response_json = ?, last_error = NULL, updated_at_ms = ?
                     WHERE id = ?",
                    params![STATE_ACKED, response_body, now, row.id],
                )?;
            }
            AttemptOutcome::DeadLetter { reason } => {
                conn.execute(
                    "UPDATE capture_outbox
                     SET state = ?, attempts = attempts + 1, last_error = ?, updated_at_ms = ?
                     WHERE id = ?",
                    params![STATE_DEAD_LETTER, reason, now, row.id],
                )?;
            }
            AttemptOutcome::Retry { reason } => {
                let next_attempts = row.attempts + 1;
                if next_attempts >= self.config.max_attempts {
                    conn.execute(
                        "UPDATE capture_outbox
                         SET state = ?, attempts = attempts + 1, last_error = ?, updated_at_ms = ?
                         WHERE id = ?",
                        params![STATE_DEAD_LETTER, reason, now, row.id],
                    )?;
                } else {
                    let delay = self.retry_delay_ms(row.id, next_attempts);
                    let next_at = now.saturating_add(delay);
                    conn.execute(
                        "UPDATE capture_outbox
                         SET state = ?, attempts = attempts + 1, next_attempt_at_ms = ?, last_error = ?, updated_at_ms = ?
                         WHERE id = ?",
                        params![STATE_ENQUEUED, next_at, reason, now, row.id],
                    )?;
                    self.notify.notify_waiters();
                }
            }
        }
        Ok(())
    }

    fn retry_delay_ms(&self, id: i64, attempts: u32) -> i64 {
        let exp = (attempts.saturating_sub(1)).min(10);
        let mut delay = self.config.base_backoff_ms.saturating_mul(1_i64 << exp);
        if delay > self.config.max_backoff_ms {
            delay = self.config.max_backoff_ms;
        }
        let jitter = ((id.unsigned_abs() + attempts as u64 * 31) % 97) as i64;
        delay.saturating_add(jitter)
    }

    pub fn notify_drain(&self) {
        self.notify.notify_waiters();
    }

    pub fn status_snapshot(&self, dead_letter_limit: u32) -> anyhow::Result<OutboxStatusSnapshot> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let mut count_stmt = conn.prepare(
            "SELECT
               SUM(CASE WHEN state = 'ENQUEUED' THEN 1 ELSE 0 END) AS enqueued,
               SUM(CASE WHEN state = 'INFLIGHT' THEN 1 ELSE 0 END) AS inflight,
               SUM(CASE WHEN state = 'ACKED' THEN 1 ELSE 0 END) AS acked,
               SUM(CASE WHEN state = 'DEAD_LETTER' THEN 1 ELSE 0 END) AS dead_letter
             FROM capture_outbox",
        )?;
        let counts = count_stmt.query_row([], |row| {
            Ok(OutboxCounts {
                enqueued: row.get::<_, Option<i64>>(0)?.unwrap_or(0) as u64,
                inflight: row.get::<_, Option<i64>>(1)?.unwrap_or(0) as u64,
                acked: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
                dead_letter: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
            })
        })?;

        let mut dl_stmt = conn.prepare(
            "SELECT id, request_id, attempts, last_error, payload_json, updated_at_ms
             FROM capture_outbox
             WHERE state = ?
             ORDER BY updated_at_ms DESC, id DESC
             LIMIT ?",
        )?;
        let mut dead_letters = Vec::new();
        let mut rows = dl_stmt.query(params![STATE_DEAD_LETTER, dead_letter_limit])?;
        while let Some(row) = rows.next()? {
            let outbox_id: i64 = row.get(0)?;
            let request_id: String = row.get(1)?;
            let attempts: i64 = row.get(2)?;
            let reason: Option<String> = row.get(3)?;
            let payload_json: String = row.get(4)?;
            let updated_at_ms: i64 = row.get(5)?;

            let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
            let idempotency_key = payload
                .get("idempotencyKey")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            let draft_text = payload
                .get("draftText")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);

            dead_letters.push(DeadLetterStatusItem {
                outbox_id,
                request_id,
                attempts: attempts as u64,
                reason: reason.unwrap_or_else(|| "dead-letter".to_string()),
                idempotency_key,
                draft_text,
                updated_at_ms,
            });
        }

        Ok(OutboxStatusSnapshot {
            counts,
            dead_letters,
        })
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn queued_response_json() -> Value {
    json!({
        "success": true,
        "data": {
            "status": "queued"
        }
    })
}
