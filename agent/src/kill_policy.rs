//! T5 K4 (#351) — one-row KillPolicy next to the kill-switch audit sqlite file.

use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const L3_COUNTDOWN_SECS: u32 = 90;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillPolicy {
    pub default_level: u8, // 1|2|3
    pub countdown_secs: u32,
    pub website_block: bool,
}

impl Default for KillPolicy {
    fn default() -> Self {
        Self {
            default_level: 3,
            countdown_secs: L3_COUNTDOWN_SECS,
            website_block: true,
        }
    }
}

#[derive(Clone)]
pub struct KillPolicyStore {
    conn: Arc<Mutex<Connection>>,
}

impl KillPolicyStore {
    pub fn open(db_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = db_path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create policy db parent {}", parent.display()))?;
            }
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open kill policy db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS kill_policy (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  default_level INTEGER NOT NULL,
  countdown_secs INTEGER NOT NULL,
  website_block INTEGER NOT NULL
);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn load_or_insert_defaults(&self) -> anyhow::Result<KillPolicy> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let existing = guard
            .query_row(
                "SELECT default_level, countdown_secs, website_block FROM kill_policy WHERE id = 1",
                [],
                |row| {
                    let website_block: i64 = row.get(2)?;
                    Ok(KillPolicy {
                        default_level: row.get(0)?,
                        countdown_secs: row.get(1)?,
                        website_block: website_block != 0,
                    })
                },
            )
            .optional()?;
        if let Some(policy) = existing {
            return Ok(policy);
        }
        let defaults = KillPolicy::default();
        guard.execute(
            r#"INSERT INTO kill_policy (id, default_level, countdown_secs, website_block)
               VALUES (1, ?1, ?2, ?3)"#,
            params![
                defaults.default_level,
                defaults.countdown_secs,
                defaults.website_block as i32,
            ],
        )?;
        Ok(defaults)
    }

    /// T7 Settings will UPDATE this row. Tests seed countdown / website_block.
    pub fn save(&self, policy: &KillPolicy) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO kill_policy (id, default_level, countdown_secs, website_block)
               VALUES (1, ?1, ?2, ?3)
               ON CONFLICT(id) DO UPDATE SET
                 default_level = excluded.default_level,
                 countdown_secs = excluded.countdown_secs,
                 website_block = excluded.website_block"#,
            params![
                policy.default_level,
                policy.countdown_secs,
                policy.website_block as i32,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (std::path::PathBuf, KillPolicyStore) {
        let path =
            std::env::temp_dir().join(format!("rta-kill-policy-unit-{}.db", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&path);
        let store = KillPolicyStore::open(&path).expect("open");
        (path, store)
    }

    #[test]
    fn load_or_insert_defaults_inserts_singleton_row() {
        let (path, store) = temp_db();
        let policy = store.load_or_insert_defaults().expect("insert defaults");
        assert_eq!(policy, KillPolicy::default());
        assert_eq!(policy.default_level, 3);
        assert_eq!(policy.countdown_secs, 90);
        assert!(policy.website_block);

        let conn = Connection::open(&path).expect("reopen");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kill_policy", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_or_insert_defaults_is_idempotent() {
        let (path, store) = temp_db();
        let first = store.load_or_insert_defaults().expect("first");
        let second = store.load_or_insert_defaults().expect("second");
        assert_eq!(first, second);
        assert_eq!(first, KillPolicy::default());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_then_load_roundtrip_keeps_non_default_countdown() {
        let (path, store) = temp_db();
        store.load_or_insert_defaults().expect("defaults");
        let custom = KillPolicy {
            default_level: 2,
            countdown_secs: 45,
            website_block: false,
        };
        store.save(&custom).expect("save");
        let loaded = store.load_or_insert_defaults().expect("load");
        assert_eq!(loaded, custom);
        let _ = std::fs::remove_file(path);
    }
}
