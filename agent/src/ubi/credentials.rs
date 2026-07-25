//! Tagged Keychain credential blobs (R6, Phase 2).
//! Host-only — never copied into Wasm component memory.

use serde::{Deserialize, Serialize};

pub const BROKER_CREDENTIAL_KEYCHAIN_SERVICE: &str = "in.tradeautopsy.station.broker-credentials";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "authScheme", rename_all = "snake_case")]
pub enum CredentialBlob {
    #[serde(rename = "hmac_api_key_secret")]
    HmacApiKeySecret {
        #[serde(rename = "apiKey")]
        api_key: String,
        #[serde(rename = "apiSecret")]
        api_secret: String,
    },
    #[serde(rename = "kotak_neo_totp_session")]
    KotakNeoTotpSession {
        #[serde(rename = "consumerKey")]
        consumer_key: String,
        #[serde(rename = "tradeToken")]
        trade_token: String,
        sid: String,
        #[serde(rename = "baseUrl")]
        base_url: String,
        #[serde(default, rename = "expiresAt")]
        expires_at: Option<String>,
    },
}

impl CredentialBlob {
    pub fn hmac(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self::HmacApiKeySecret {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    /// Keychain account id — must match Swift `KeychainBrokerCredentialStore`.
    pub fn account_key(environment: &str, broker_slug: &str, connection_id: &str) -> String {
        format!("{environment}.{broker_slug}.{connection_id}")
    }

    pub fn api_key_for_tests(&self) -> Option<&str> {
        match self {
            Self::HmacApiKeySecret { api_key, .. } => Some(api_key),
            Self::KotakNeoTotpSession { .. } => None,
        }
    }
}

/// Decode Keychain JSON: tagged blob, or legacy flat `{apiKey,apiSecret}`.
pub fn decode_credential_blob(json: &str) -> anyhow::Result<CredentialBlob> {
    if let Ok(blob) = serde_json::from_str::<CredentialBlob>(json) {
        return Ok(blob);
    }
    #[derive(Deserialize)]
    struct Legacy {
        #[serde(rename = "apiKey")]
        api_key: String,
        #[serde(rename = "apiSecret")]
        api_secret: String,
    }
    let legacy: Legacy = serde_json::from_str(json)?;
    Ok(CredentialBlob::hmac(legacy.api_key, legacy.api_secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_hmac_tagged() {
        let blob = CredentialBlob::hmac("k", "s");
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("hmac_api_key_secret"));
        let decoded = decode_credential_blob(&json).unwrap();
        assert_eq!(decoded, blob);
    }

    #[test]
    fn legacy_flat_json_decodes_as_hmac() {
        let json = r#"{"apiKey":"k","apiSecret":"s"}"#;
        let decoded = decode_credential_blob(json).unwrap();
        assert_eq!(decoded, CredentialBlob::hmac("k", "s"));
    }

    #[test]
    fn kotak_session_blob_roundtrip() {
        let blob = CredentialBlob::KotakNeoTotpSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid1".into(),
            base_url: "https://cis.kotaksecurities.com".into(),
            expires_at: Some("2026-07-25T18:00:00Z".into()),
        };
        let json = serde_json::to_string(&blob).unwrap();
        let decoded = decode_credential_blob(&json).unwrap();
        assert_eq!(decoded, blob);
    }
}
