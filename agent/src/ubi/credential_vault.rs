//! Host credential vault — Enforcer loads secrets; Wasm never sees them (R6).

use crate::ubi::credentials::{
    decode_credential_blob, keychain_service_for, CredentialBlob,
    BROKER_CREDENTIAL_KEYCHAIN_SERVICE, KOTAK_SESSION_KEYCHAIN_SERVICE,
};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::sync::Mutex;

pub trait BrokerCredentialVault: Send + Sync {
    fn load(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
    ) -> Result<Option<CredentialBlob>>;

    fn save(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
        blob: &CredentialBlob,
    ) -> Result<()>;

    /// Remove a vault entry (Connect rollback / Delete). Missing entry is Ok.
    fn delete(&self, environment: &str, broker_slug: &str, connection_id: &str) -> Result<()>;
}

#[derive(Default)]
pub struct MemoryBrokerCredentialVault {
    inner: Mutex<HashMap<String, CredentialBlob>>,
}

impl MemoryBrokerCredentialVault {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BrokerCredentialVault for MemoryBrokerCredentialVault {
    fn load(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
    ) -> Result<Option<CredentialBlob>> {
        let key = CredentialBlob::account_key(environment, broker_slug, connection_id);
        Ok(self.inner.lock().expect("vault").get(&key).cloned())
    }

    fn save(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
        blob: &CredentialBlob,
    ) -> Result<()> {
        let key = CredentialBlob::account_key(environment, broker_slug, connection_id);
        self.inner.lock().expect("vault").insert(key, blob.clone());
        Ok(())
    }

    fn delete(&self, environment: &str, broker_slug: &str, connection_id: &str) -> Result<()> {
        let key = CredentialBlob::account_key(environment, broker_slug, connection_id);
        self.inner.lock().expect("vault").remove(&key);
        Ok(())
    }
}

fn uses_kotak_session_service(broker_slug: &str) -> bool {
    keychain_service_for(broker_slug) == KOTAK_SESSION_KEYCHAIN_SERVICE
}

fn read_keyring_blob(service: &str, account: &str) -> Result<Option<CredentialBlob>> {
    let entry = keyring::Entry::new(service, account).context("broker credential keyring entry")?;
    match entry.get_password() {
        Ok(s) if !s.trim().is_empty() => match decode_credential_blob(&s) {
            Ok(blob) => Ok(Some(blob)),
            Err(e) => {
                tracing::warn!(
                    account = %account,
                    "broker credential blob unreadable (treating as absent): {e}"
                );
                Ok(None)
            }
        },
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => bail!("keyring get broker credentials: {e}"),
    }
}

fn write_keyring_blob(service: &str, account: &str, blob: &CredentialBlob) -> Result<()> {
    let entry = keyring::Entry::new(service, account).context("broker credential keyring entry")?;
    let json = serde_json::to_string(blob).context("serialize credential blob")?;
    entry
        .set_password(&json)
        .context("keyring set broker credentials")?;
    Ok(())
}

fn delete_keyring_blob(service: &str, account: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, account).context("broker credential keyring entry")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => bail!("keyring delete broker credentials: {e}"),
    }
}

/// Reads/writes Keychain services matching Station:
/// - HMAC → `BROKER_CREDENTIAL_KEYCHAIN_SERVICE`
/// - Kotak session → `KOTAK_SESSION_KEYCHAIN_SERVICE` (migrates once from legacy shared service)
///
/// Requires `keyring` crate feature `apple-native`. Without it, keyring 3 falls back to an
/// in-process mock store — mint appears to succeed but Station SecItem reads miss → false
/// "Network unavailable".
///
/// Process-local cache: after mint `save` (or first `load`), Start / present reuse memory and
/// do not re-hit Keychain — avoids repeated macOS ACL password prompts in one agent lifetime.
pub struct KeyringBrokerCredentialVault {
    cache: Mutex<HashMap<String, CredentialBlob>>,
}

impl KeyringBrokerCredentialVault {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Load from Keychain with Kotak legacy migration (no cache). Used by tests / migration path.
    fn load_from_keychain(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
    ) -> Result<Option<CredentialBlob>> {
        let account = CredentialBlob::account_key(environment, broker_slug, connection_id);
        let primary = keychain_service_for(broker_slug);
        if let Some(blob) = read_keyring_blob(primary, &account)? {
            return Ok(Some(blob));
        }

        // One-shot migrate Kotak sessions stored under the old shared HMAC service.
        if uses_kotak_session_service(broker_slug) {
            if let Some(blob) = read_keyring_blob(BROKER_CREDENTIAL_KEYCHAIN_SERVICE, &account)? {
                let _ = write_keyring_blob(KOTAK_SESSION_KEYCHAIN_SERVICE, &account, &blob);
                let _ = delete_keyring_blob(BROKER_CREDENTIAL_KEYCHAIN_SERVICE, &account);
                return Ok(Some(blob));
            }
        }

        Ok(None)
    }
}

impl Default for KeyringBrokerCredentialVault {
    fn default() -> Self {
        Self::new()
    }
}

impl BrokerCredentialVault for KeyringBrokerCredentialVault {
    fn load(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
    ) -> Result<Option<CredentialBlob>> {
        let account = CredentialBlob::account_key(environment, broker_slug, connection_id);
        if let Some(cached) = self
            .cache
            .lock()
            .expect("vault cache")
            .get(&account)
            .cloned()
        {
            return Ok(Some(cached));
        }

        let Some(blob) = self.load_from_keychain(environment, broker_slug, connection_id)? else {
            return Ok(None);
        };
        self.cache
            .lock()
            .expect("vault cache")
            .insert(account, blob.clone());
        Ok(Some(blob))
    }

    fn save(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
        blob: &CredentialBlob,
    ) -> Result<()> {
        let account = CredentialBlob::account_key(environment, broker_slug, connection_id);
        let service = keychain_service_for(broker_slug);
        write_keyring_blob(service, &account, blob)?;
        // Best-effort: avoid leaving a stale Kotak session in the HMAC service after remint.
        if uses_kotak_session_service(broker_slug) {
            let _ = delete_keyring_blob(BROKER_CREDENTIAL_KEYCHAIN_SERVICE, &account);
        }
        self.cache
            .lock()
            .expect("vault cache")
            .insert(account, blob.clone());
        Ok(())
    }

    fn delete(&self, environment: &str, broker_slug: &str, connection_id: &str) -> Result<()> {
        let account = CredentialBlob::account_key(environment, broker_slug, connection_id);
        self.cache.lock().expect("vault cache").remove(&account);
        let service = keychain_service_for(broker_slug);
        delete_keyring_blob(service, &account)?;
        if uses_kotak_session_service(broker_slug) {
            let _ = delete_keyring_blob(BROKER_CREDENTIAL_KEYCHAIN_SERVICE, &account);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_vault_roundtrip() {
        let vault = MemoryBrokerCredentialVault::new();
        let blob = CredentialBlob::hmac("k", "s");
        vault.save("prod", "binance_com", "conn-1", &blob).unwrap();
        let loaded = vault.load("prod", "binance_com", "conn-1").unwrap();
        assert_eq!(loaded, Some(blob));
        assert!(vault
            .load("prod", "binance_com", "missing")
            .unwrap()
            .is_none());
        vault.delete("prod", "binance_com", "conn-1").unwrap();
        assert!(vault
            .load("prod", "binance_com", "conn-1")
            .unwrap()
            .is_none());
    }

    #[test]
    fn keyring_vault_cache_serves_after_save_without_second_keyring_roundtrip() {
        // Memory-shaped behavior of the cache layer: save populates cache; load hits it.
        let vault = KeyringBrokerCredentialVault::new();
        let account = CredentialBlob::account_key("prod", "kotak_neo", "conn-1");
        let blob = CredentialBlob::hmac("k", "s");
        vault
            .cache
            .lock()
            .expect("cache")
            .insert(account.clone(), blob.clone());
        let loaded = vault.load("prod", "kotak_neo", "conn-1").unwrap();
        assert_eq!(loaded, Some(blob));
        // Delete clears cache even if keyring has no entry.
        vault.delete("prod", "kotak_neo", "conn-1").unwrap();
        assert!(vault.cache.lock().expect("cache").get(&account).is_none());
    }

    #[test]
    fn kotak_slug_uses_dedicated_keychain_service() {
        assert_eq!(
            keychain_service_for("kotak_neo"),
            KOTAK_SESSION_KEYCHAIN_SERVICE
        );
        assert_eq!(
            keychain_service_for("binance_com"),
            BROKER_CREDENTIAL_KEYCHAIN_SERVICE
        );
    }

    #[test]
    fn cache_hit_skips_migration_path_for_kotak() {
        let vault = KeyringBrokerCredentialVault::new();
        let account = CredentialBlob::account_key("prod", "kotak_neo", "conn-migrate");
        let blob = CredentialBlob::KotakNeoTotpSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://cis.kotaksecurities.com".into(),
            hs_server_id: "server4".into(),
            expires_at: None,
        };
        vault
            .cache
            .lock()
            .expect("cache")
            .insert(account.clone(), blob.clone());
        let loaded = vault
            .load("prod", "kotak_neo", "conn-migrate")
            .expect("load");
        assert_eq!(loaded, Some(blob));
        assert!(vault.cache.lock().expect("cache").contains_key(&account));
    }
}
