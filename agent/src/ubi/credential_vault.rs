//! Host credential vault — Enforcer loads secrets; Wasm never sees them (R6).

use crate::ubi::credentials::{
    decode_credential_blob, CredentialBlob, BROKER_CREDENTIAL_KEYCHAIN_SERVICE,
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
}

/// Reads the same Keychain service/account as Station Swift store.
pub struct KeyringBrokerCredentialVault;

impl BrokerCredentialVault for KeyringBrokerCredentialVault {
    fn load(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
    ) -> Result<Option<CredentialBlob>> {
        let account = CredentialBlob::account_key(environment, broker_slug, connection_id);
        let entry = keyring::Entry::new(BROKER_CREDENTIAL_KEYCHAIN_SERVICE, &account)
            .context("broker credential keyring entry")?;
        match entry.get_password() {
            Ok(s) if !s.trim().is_empty() => Ok(Some(decode_credential_blob(&s)?)),
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => bail!("keyring get broker credentials: {e}"),
        }
    }

    fn save(
        &self,
        environment: &str,
        broker_slug: &str,
        connection_id: &str,
        blob: &CredentialBlob,
    ) -> Result<()> {
        let account = CredentialBlob::account_key(environment, broker_slug, connection_id);
        let entry = keyring::Entry::new(BROKER_CREDENTIAL_KEYCHAIN_SERVICE, &account)
            .context("broker credential keyring entry")?;
        let json = serde_json::to_string(blob).context("serialize credential blob")?;
        entry
            .set_password(&json)
            .context("keyring set broker credentials")?;
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
    }
}
