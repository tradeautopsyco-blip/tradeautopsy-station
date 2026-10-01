//! Durable kill-switch latch — current armed state (distinct from append-only audit).

use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const ROW_ID: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillLatchSnapshot {
    pub active: bool,
    pub level: i32,
    pub broker: String,
    pub armed_at_ms: i64,
    pub expires_at_ms: Option<i64>,
    pub requires_ack: bool,
}

#[derive(Clone)]
pub struct KillLatchStore {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillLatchLoad {
    Inactive,
    Active(KillLatchSnapshot),
}

impl KillLatchStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create latch db parent {}", parent.display()))?;
            }
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open kill latch db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS kill_latch (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  active        INTEGER NOT NULL,
  level         INTEGER NOT NULL,
  broker        TEXT    NOT NULL,
  armed_at_ms   INTEGER NOT NULL,
  expires_at_ms INTEGER,
  requires_ack  INTEGER NOT NULL DEFAULT 0
);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn db_path_from_env_or_default() -> PathBuf {
        std::env::var("AGENT_KILL_LATCH_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = station_app_support_dir();
                p.push("kill-latch.db");
                p
            })
    }

    pub fn load(&self) -> anyhow::Result<KillLatchLoad> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let row = guard
            .query_row(
                "SELECT active, level, broker, armed_at_ms, expires_at_ms, requires_ack FROM kill_latch WHERE id = 1",
                [],
                |row| {
                    let active: i64 = row.get(0)?;
                    let level: i32 = row.get(1)?;
                    let broker: String = row.get(2)?;
                    let armed_at_ms: i64 = row.get(3)?;
                    let expires_at_ms: Option<i64> = row.get(4)?;
                    let requires_ack: i64 = row.get(5)?;
                    Ok(KillLatchSnapshot {
                        active: active != 0,
                        level,
                        broker,
                        armed_at_ms,
                        expires_at_ms,
                        requires_ack: requires_ack != 0,
                    })
                },
            )
            .optional()?;
        match row {
            None => Ok(KillLatchLoad::Inactive),
            Some(s) if !s.active => Ok(KillLatchLoad::Inactive),
            Some(s) => Ok(KillLatchLoad::Active(s)),
        }
    }

    pub fn arm(&self, snapshot: &KillLatchSnapshot) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO kill_latch (id, active, level, broker, armed_at_ms, expires_at_ms, requires_ack)
               VALUES (1, 1, ?1, ?2, ?3, ?4, ?5)
               ON CONFLICT(id) DO UPDATE SET
                 active = 1,
                 level = excluded.level,
                 broker = excluded.broker,
                 armed_at_ms = excluded.armed_at_ms,
                 expires_at_ms = excluded.expires_at_ms,
                 requires_ack = excluded.requires_ack"#,
            params![
                snapshot.level,
                snapshot.broker,
                snapshot.armed_at_ms,
                snapshot.expires_at_ms,
                if snapshot.requires_ack { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        guard.execute(
            "UPDATE kill_latch SET active = 0 WHERE id = 1",
            [],
        )?;
        Ok(())
    }
}

/// `~/Library/Application Support/in.tradeautopsy.station/` (or env override).
pub fn station_app_support_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("AGENT_STATION_APP_SUPPORT_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = PathBuf::from(home);
        p.push("Library/Application Support/in.tradeautopsy.station");
        return p;
    }
    PathBuf::from("/tmp/tradeautopsy-station-app-support")
}

pub fn default_kill_switch_audit_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("AGENT_KILL_SWITCH_AUDIT_DB_PATH") {
        return PathBuf::from(p);
    }
    let mut p = station_app_support_dir();
    p.push("kill-switch-audit.db");
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> PathBuf {
        std::env::temp_dir().join(format!("rta-latch-{}.db", uuid::Uuid::new_v4()))
    }

    #[test]
    fn latch_round_trips_arm_and_clear() {
        let path = temp_db();
        let _ = std::fs::remove_file(&path);
        let store = KillLatchStore::open(&path).expect("open");
        assert_eq!(store.load().expect("load"), KillLatchLoad::Inactive);

        let snap = KillLatchSnapshot {
            active: true,
            level: 3,
            broker: "binance_us".to_string(),
            armed_at_ms: 1_700_000_000_000,
            expires_at_ms: Some(1_700_000_090_000),
            requires_ack: true,
        };
        store.arm(&snap).expect("arm");
        let loaded = store.load().expect("load armed");
        assert_eq!(
            loaded,
            KillLatchLoad::Active(snap.clone()),
            "latch must round-trip"
        );

        store.clear().expect("clear");
        assert_eq!(store.load().expect("load cleared"), KillLatchLoad::Inactive);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn latch_read_failure_fails_closed() {
        let path = temp_db();
        let _ = std::fs::remove_file(&path);
        let store = KillLatchStore::open(&path).expect("open");
        store.arm(&KillLatchSnapshot {
            active: true,
            level: 3,
            broker: "zerodha".to_string(),
            armed_at_ms: 1,
            expires_at_ms: None,
            requires_ack: true,
        })
        .expect("arm");

        // Break the row so sqlite returns a type error on read (fail closed at runtime).
        {
            let conn = Connection::open(&path).expect("raw open");
            conn.execute("UPDATE kill_latch SET level = 'corrupt'", [])
                .expect("corrupt row");
        }

        let err = store.load().expect_err("corrupt db must not load as inactive");
        assert!(
            !err.to_string().is_empty(),
            "corrupt latch must error, never imply disarmed"
        );

        let _ = std::fs::remove_file(path);
    }
}
