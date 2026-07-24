//! Station Caller tokens in OS keychain (A8 II.6).
//! Access + refresh for `aud=station` JWTs — never daemon secret + x-user-id.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "TradeAutopsy";
const USER: &str = "station_caller_tokens";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_expires_in: Option<u64>,
}

pub trait StationTokenStore: Send + Sync {
    fn save(&self, tokens: &StationTokens) -> Result<()>;
    fn load(&self) -> Result<Option<StationTokens>>;
    fn clear(&self) -> Result<()>;
}

/// In-memory store for tests and ephemeral sessions.
#[derive(Default)]
pub struct MemoryStationTokenStore {
    inner: std::sync::Mutex<Option<StationTokens>>,
}

impl StationTokenStore for MemoryStationTokenStore {
    fn save(&self, tokens: &StationTokens) -> Result<()> {
        *self.inner.lock().expect("station token mutex") = Some(tokens.clone());
        Ok(())
    }

    fn load(&self) -> Result<Option<StationTokens>> {
        Ok(self.inner.lock().expect("station token mutex").clone())
    }

    fn clear(&self) -> Result<()> {
        *self.inner.lock().expect("station token mutex") = None;
        Ok(())
    }
}

/// macOS Keychain / Windows Credential Manager / Linux Secret Service via `keyring`.
pub struct KeyringStationTokenStore;

impl StationTokenStore for KeyringStationTokenStore {
    fn save(&self, tokens: &StationTokens) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, USER).context("keyring entry")?;
        let json = serde_json::to_string(tokens).context("serialize station tokens")?;
        entry.set_password(&json).context("keyring set station tokens")?;
        Ok(())
    }

    fn load(&self) -> Result<Option<StationTokens>> {
        let entry = keyring::Entry::new(SERVICE, USER).context("keyring entry")?;
        match entry.get_password() {
            Ok(s) if !s.trim().is_empty() => {
                let tokens: StationTokens =
                    serde_json::from_str(&s).context("parse station tokens from keyring")?;
                Ok(Some(tokens))
            }
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("keyring get station tokens: {e}")),
        }
    }

    fn clear(&self) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, USER).context("keyring entry")?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("keyring clear station tokens: {e}")),
        }
    }
}

/// Authorization header value for Console APIs (`Bearer <access>`).
pub fn bearer_authorization(tokens: &StationTokens) -> String {
    format!("Bearer {}", tokens.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips_station_tokens() {
        let store = MemoryStationTokenStore::default();
        let tokens = StationTokens {
            access_token: "access.jwt.here".into(),
            refresh_token: "opaque-refresh".into(),
            expires_in: 900,
            refresh_expires_in: Some(5_184_000),
        };

        store.save(&tokens).expect("save");
        let loaded = store.load().expect("load").expect("present");
        assert_eq!(loaded, tokens);
        assert_eq!(bearer_authorization(&loaded), "Bearer access.jwt.here");

        store.clear().expect("clear");
        assert!(store.load().expect("load after clear").is_none());
    }
}
