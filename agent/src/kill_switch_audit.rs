//! Station Slice B Phase 4 (#191) — append-only Ed25519-signed kill switch audit log.

use anyhow::Context;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const CANONICAL_PREFIX: &str = "kill_switch_audit_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KillSwitchAuditRecord {
    pub id: i64,
    pub event_type: String,
    pub fired_at_ms: i64,
    pub level: i32,
    pub broker: String,
    pub trigger: Option<String>,
    pub hosts_json: String,
    pub signature_b64: String,
    pub public_key_id: String,
}

#[derive(Debug, Clone)]
pub struct KillSwitchAuditAppend {
    pub event_type: String,
    pub level: i32,
    pub broker: String,
    pub trigger: Option<String>,
    pub hosts: Vec<String>,
}

#[derive(Clone)]
pub struct KillSwitchAuditSigner {
    key: Arc<SigningKey>,
    public_key_id: String,
}

impl KillSwitchAuditSigner {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let key = SigningKey::from_bytes(&seed);
        let public_key_id = public_key_id_from_verifying_key(&key.verifying_key());
        Self {
            key: Arc::new(key),
            public_key_id,
        }
    }

    pub fn from_env_or_generate() -> Self {
        let Ok(hex_seed) = std::env::var("AGENT_KILL_SWITCH_AUDIT_SEED_HEX") else {
            return Self::generate();
        };
        let bytes = match hex::decode(hex_seed.trim()) {
            Ok(b) if b.len() == 32 => b,
            _ => return Self::generate(),
        };
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        Self::from_seed(seed)
    }

    fn generate() -> Self {
        use rand::rngs::OsRng;
        let key = SigningKey::generate(&mut OsRng);
        let public_key_id = public_key_id_from_verifying_key(&key.verifying_key());
        Self {
            key: Arc::new(key),
            public_key_id,
        }
    }

    pub fn public_key_id(&self) -> &str {
        &self.public_key_id
    }

    pub fn sign_record(
        &self,
        event_type: &str,
        fired_at_ms: i64,
        level: i32,
        broker: &str,
        trigger: Option<&str>,
        hosts_json: &str,
    ) -> String {
        let msg = canonical_audit_message(
            event_type,
            fired_at_ms,
            level,
            broker,
            trigger,
            hosts_json,
        );
        let sig: Signature = self.key.sign(msg.as_bytes());
        B64.encode(sig.to_bytes())
    }

    pub fn verify_record(&self, record: &KillSwitchAuditRecord) -> bool {
        verify_audit_signature(
            self.key.verifying_key(),
            &record.event_type,
            record.fired_at_ms,
            record.level,
            &record.broker,
            record.trigger.as_deref(),
            &record.hosts_json,
            &record.signature_b64,
        )
    }
}

fn public_key_id_from_verifying_key(key: &VerifyingKey) -> String {
    let digest = Sha256::digest(key.as_bytes());
    format!("v1:{}", hex::encode(&digest[..8]))
}

pub fn canonical_audit_message(
    event_type: &str,
    fired_at_ms: i64,
    level: i32,
    broker: &str,
    trigger: Option<&str>,
    hosts_json: &str,
) -> String {
    let trigger_part = trigger.unwrap_or("");
    format!(
        "{CANONICAL_PREFIX}\n{event_type}\n{fired_at_ms}\n{level}\n{broker}\n{trigger_part}\n{hosts_json}"
    )
}

pub fn verify_audit_signature(
    verifying_key: VerifyingKey,
    event_type: &str,
    fired_at_ms: i64,
    level: i32,
    broker: &str,
    trigger: Option<&str>,
    hosts_json: &str,
    signature_b64: &str,
) -> bool {
    let Ok(raw) = B64.decode(signature_b64.trim().as_bytes()) else {
        return false;
    };
    if raw.len() != 64 {
        return false;
    }
    let Ok(sig_arr): Result<[u8; 64], _> = raw.try_into() else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_arr);
    let msg = canonical_audit_message(event_type, fired_at_ms, level, broker, trigger, hosts_json);
    verifying_key.verify(msg.as_bytes(), &sig).is_ok()
}

#[derive(Clone)]
pub struct KillSwitchAuditStore {
    conn: Arc<Mutex<Connection>>,
}

impl KillSwitchAuditStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create audit db parent {}", parent.display()))?;
            }
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open kill switch audit db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS kill_switch_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  event_type TEXT NOT NULL,
  fired_at_ms INTEGER NOT NULL,
  level INTEGER NOT NULL,
  broker TEXT NOT NULL,
  trigger TEXT,
  hosts_json TEXT NOT NULL,
  signature BLOB NOT NULL,
  public_key_id TEXT NOT NULL
);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn db_path_from_env_or_default() -> PathBuf {
        std::env::var("AGENT_KILL_SWITCH_AUDIT_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("tradeautopsy-agent-kill-switch-audit.db");
                p
            })
    }

    pub fn append_signed(
        &self,
        signer: &KillSwitchAuditSigner,
        input: KillSwitchAuditAppend,
    ) -> anyhow::Result<KillSwitchAuditRecord> {
        let fired_at_ms = chrono::Utc::now().timestamp_millis();
        let hosts_json =
            serde_json::to_string(&input.hosts).context("serialize hosts for audit")?;
        let signature_b64 = signer.sign_record(
            &input.event_type,
            fired_at_ms,
            input.level,
            &input.broker,
            input.trigger.as_deref(),
            &hosts_json,
        );
        let signature_bytes = B64
            .decode(signature_b64.as_bytes())
            .context("decode signature for sqlite blob")?;
        let public_key_id = signer.public_key_id().to_string();

        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO kill_switch_audit (
                event_type, fired_at_ms, level, broker, trigger, hosts_json, signature, public_key_id
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"#,
            params![
                input.event_type,
                fired_at_ms,
                input.level,
                input.broker,
                input.trigger,
                hosts_json,
                signature_bytes,
                public_key_id,
            ],
        )?;
        let id = guard.last_insert_rowid();
        Ok(KillSwitchAuditRecord {
            id,
            event_type: input.event_type,
            fired_at_ms,
            level: input.level,
            broker: input.broker,
            trigger: input.trigger,
            hosts_json,
            signature_b64,
            public_key_id,
        })
    }

    pub fn tail(&self, limit: u32) -> anyhow::Result<Vec<KillSwitchAuditRecord>> {
        let guard = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            r#"SELECT id, event_type, fired_at_ms, level, broker, trigger, hosts_json, signature, public_key_id
               FROM kill_switch_audit
               ORDER BY id DESC
               LIMIT ?1"#,
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let signature_blob: Vec<u8> = row.get(7)?;
            Ok(KillSwitchAuditRecord {
                id: row.get(0)?,
                event_type: row.get(1)?,
                fired_at_ms: row.get(2)?,
                level: row.get(3)?,
                broker: row.get(4)?,
                trigger: row.get(5)?,
                hosts_json: row.get(6)?,
                signature_b64: B64.encode(signature_blob),
                public_key_id: row.get(8)?,
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

    #[test]
    fn audit_signature_roundtrip_verifies() {
        let signer = KillSwitchAuditSigner::from_seed([7u8; 32]);
        let hosts_json = r#"["kite.zerodha.com"]"#;
        let sig = signer.sign_record("fire", 1_700_000_000_000, 3, "zerodha", Some("manual"), hosts_json);
        assert!(verify_audit_signature(
            signer.key.verifying_key(),
            "fire",
            1_700_000_000_000,
            3,
            "zerodha",
            Some("manual"),
            hosts_json,
            &sig,
        ));
    }

    #[test]
    fn tampered_audit_payload_fails_verification() {
        let signer = KillSwitchAuditSigner::from_seed([8u8; 32]);
        let hosts_json = r#"["kite.zerodha.com"]"#;
        let sig = signer.sign_record("fire", 1_700_000_000_000, 3, "zerodha", None, hosts_json);
        assert!(!verify_audit_signature(
            signer.key.verifying_key(),
            "fire",
            1_700_000_000_000,
            3,
            "zerodha",
            Some("tampered"),
            hosts_json,
            &sig,
        ));
    }

    #[test]
    fn append_only_store_persists_verifiable_row() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "rta-audit-store-{}.db",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);

        let store = KillSwitchAuditStore::open(&path).expect("open store");
        let signer = KillSwitchAuditSigner::from_seed([9u8; 32]);
        let record = store
            .append_signed(
                &signer,
                KillSwitchAuditAppend {
                    event_type: "fire".to_string(),
                    level: 3,
                    broker: "zerodha".to_string(),
                    trigger: Some("test".to_string()),
                    hosts: vec!["kite.zerodha.com".to_string()],
                },
            )
            .expect("append");

        assert!(signer.verify_record(&record));
        let tail = store.tail(1).expect("tail");
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].event_type, "fire");
        assert!(signer.verify_record(&tail[0]));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tampered_stored_row_fails_verify_record() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "rta-audit-tamper-{}.db",
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);

        let store = KillSwitchAuditStore::open(&path).expect("open store");
        let signer = KillSwitchAuditSigner::from_seed([10u8; 32]);
        store
            .append_signed(
                &signer,
                KillSwitchAuditAppend {
                    event_type: "fire".to_string(),
                    level: 3,
                    broker: "zerodha".to_string(),
                    trigger: None,
                    hosts: vec!["kite.zerodha.com".to_string()],
                },
            )
            .expect("append");

        let mut row = store.tail(1).expect("tail").pop().expect("one row");
        assert!(signer.verify_record(&row));
        row.hosts_json = r#"["tampered.example.com"]"#.to_string();
        assert!(!signer.verify_record(&row));

        let _ = std::fs::remove_file(path);
    }
}
