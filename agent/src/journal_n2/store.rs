//! Local N2 journal overlay — Plan + Working + Debrief spine per `declaration_id` (Wave 7).
//!
//! Console remains the declaration archive; this store holds Working fires, debrief moments,
//! and working compare snapshots for Station Journal lander.

use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct JournalN2Store {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct ConditionFireRow {
    pub rule_id: String,
    pub fired_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct DeclarationN2Row {
    pub declaration_id: String,
    pub local_date: String,
    pub moment_a_note: String,
    pub moment_c_note: String,
    pub moment_c_context: String,
    pub adherence: Option<Value>,
    pub emotion_out: Option<i32>,
    pub completed_at_ms: Option<i64>,
    pub working: Option<Value>,
    pub fires: Vec<ConditionFireRow>,
}

impl JournalN2Store {
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let conn =
            Connection::open(path).with_context(|| format!("open journal n2 db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS journal_n2_declaration (
  declaration_id TEXT PRIMARY KEY,
  local_date TEXT NOT NULL DEFAULT '',
  moment_a_note TEXT NOT NULL DEFAULT '',
  moment_c_note TEXT NOT NULL DEFAULT '',
  moment_c_context TEXT NOT NULL DEFAULT '',
  adherence_json TEXT,
  emotion_out INTEGER,
  completed_at_ms INTEGER,
  working_json TEXT,
  updated_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS journal_n2_condition_fire (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  declaration_id TEXT NOT NULL,
  rule_id TEXT NOT NULL,
  fired_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_journal_n2_fire_decl ON journal_n2_condition_fire(declaration_id);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn record_condition_fire(
        &self,
        declaration_id: &str,
        local_date: &str,
        rule_id: &str,
        fired_at_ms: i64,
        working: Option<Value>,
    ) -> anyhow::Result<()> {
        let decl = declaration_id.trim();
        let rule = rule_id.trim();
        if decl.is_empty() || rule.is_empty() {
            return Ok(());
        }
        let guard = self.conn.lock().expect("journal n2 sqlite mutex poisoned");
        let now = fired_at_ms;
        guard.execute(
            r#"INSERT INTO journal_n2_declaration (declaration_id, local_date, updated_at_ms)
               VALUES (?1, ?2, ?3)
               ON CONFLICT(declaration_id) DO UPDATE SET
                 local_date = CASE WHEN excluded.local_date != '' THEN excluded.local_date ELSE journal_n2_declaration.local_date END,
                 updated_at_ms = excluded.updated_at_ms"#,
            params![decl, local_date, now],
        )?;
        if let Some(w) = working {
            let wtxt = serde_json::to_string(&w)?;
            guard.execute(
                "UPDATE journal_n2_declaration SET working_json = ?1, updated_at_ms = ?2 WHERE declaration_id = ?3",
                params![wtxt, now, decl],
            )?;
        }
        guard.execute(
            "INSERT INTO journal_n2_condition_fire (declaration_id, rule_id, fired_at_ms) VALUES (?1, ?2, ?3)",
            params![decl, rule, fired_at_ms],
        )?;
        Ok(())
    }

    pub fn upsert_debrief_patch(&self, body: &Value) -> anyhow::Result<()> {
        let decl = body
            .get("declaration_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let decl = match decl {
            Some(d) => d,
            None => return Ok(()),
        };
        let moment_a = body
            .get("moment_a_note")
            .and_then(Value::as_str)
            .unwrap_or("");
        let moment_c = body
            .get("moment_c_note")
            .and_then(Value::as_str)
            .unwrap_or("");
        let moment_c_ctx = body
            .get("moment_c_context")
            .and_then(Value::as_str)
            .unwrap_or("");
        let adherence = body.get("adherence").cloned();
        let emotion_out = body.get("emotion_out").and_then(Value::as_i64).map(|n| n as i32);
        let completed_at_ms = body.get("completed_at_ms").and_then(Value::as_i64);
        let local_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let adherence_txt = adherence
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let guard = self.conn.lock().expect("journal n2 sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO journal_n2_declaration (
                declaration_id, local_date, moment_a_note, moment_c_note, moment_c_context,
                adherence_json, emotion_out, completed_at_ms, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(declaration_id) DO UPDATE SET
              moment_a_note = excluded.moment_a_note,
              moment_c_note = excluded.moment_c_note,
              moment_c_context = excluded.moment_c_context,
              adherence_json = excluded.adherence_json,
              emotion_out = excluded.emotion_out,
              completed_at_ms = excluded.completed_at_ms,
              updated_at_ms = excluded.updated_at_ms"#,
            params![
                decl,
                local_date,
                moment_a,
                moment_c,
                moment_c_ctx,
                adherence_txt,
                emotion_out,
                completed_at_ms,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn rows_for_ids(&self, ids: &[String]) -> anyhow::Result<HashMap<String, DeclarationN2Row>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let guard = self.conn.lock().expect("journal n2 sqlite mutex poisoned");
        let mut out = HashMap::new();
        for id in ids {
            let decl = id.trim();
            if decl.is_empty() {
                continue;
            }
            let row = guard
                .query_row(
                    r#"SELECT local_date, moment_a_note, moment_c_note, moment_c_context,
                              adherence_json, emotion_out, completed_at_ms, working_json
                       FROM journal_n2_declaration WHERE declaration_id = ?1"#,
                    params![decl],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                            r.get::<_, Option<String>>(4)?,
                            r.get::<_, Option<i32>>(5)?,
                            r.get::<_, Option<i64>>(6)?,
                            r.get::<_, Option<String>>(7)?,
                        ))
                    },
                )
                .optional()?;
            let (
                local_date,
                moment_a_note,
                moment_c_note,
                moment_c_context,
                adherence_json,
                emotion_out,
                completed_at_ms,
                working_json,
            ) = match row {
                Some(r) => r,
                None => continue,
            };
            let adherence = adherence_json
                .as_deref()
                .and_then(|t| serde_json::from_str::<Value>(t).ok());
            let working = working_json
                .as_deref()
                .and_then(|t| serde_json::from_str::<Value>(t).ok());
            let mut fires = Vec::new();
            let mut stmt = guard.prepare(
                "SELECT rule_id, fired_at_ms FROM journal_n2_condition_fire WHERE declaration_id = ?1 ORDER BY fired_at_ms ASC",
            )?;
            let fire_iter = stmt.query_map(params![decl], |r| {
                Ok(ConditionFireRow {
                    rule_id: r.get(0)?,
                    fired_at_ms: r.get(1)?,
                })
            })?;
            for f in fire_iter {
                fires.push(f?);
            }
            out.insert(
                decl.to_string(),
                DeclarationN2Row {
                    declaration_id: decl.to_string(),
                    local_date,
                    moment_a_note,
                    moment_c_note,
                    moment_c_context,
                    adherence,
                    emotion_out,
                    completed_at_ms,
                    working,
                    fires,
                },
            );
        }
        Ok(out)
    }

    pub fn n2_day_sheet_json(row: &DeclarationN2Row, plan_from_console: Option<&Value>) -> Value {
        let fires: Vec<Value> = row
            .fires
            .iter()
            .map(|f| {
                json!({
                    "rule_id": f.rule_id,
                    "fired_at_ms": f.fired_at_ms,
                })
            })
            .collect();
        let debrief = json!({
            "moment_a_note": row.moment_a_note,
            "moment_c_note": row.moment_c_note,
            "moment_c_context": row.moment_c_context,
            "adherence": row.adherence.clone().unwrap_or(Value::Null),
            "emotion_out": row.emotion_out,
            "completed_at_ms": row.completed_at_ms,
        });
        let working = row.working.clone().unwrap_or(Value::Null);
        let plan = plan_from_console.cloned().unwrap_or(Value::Null);
        json!({
            "local_date": row.local_date,
            "plan": plan,
            "working": working,
            "debrief": debrief,
            "condition_fires": fires,
        })
    }
}
