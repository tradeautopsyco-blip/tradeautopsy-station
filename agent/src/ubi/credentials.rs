//! Tagged Keychain credential blobs (R6, Phase 2).
//! Host-only — never copied into Wasm component memory.

use serde::{Deserialize, Serialize};

/// Station HMAC (and legacy Kotak) Keychain service — ACL-trusted for Station + agent.
pub const BROKER_CREDENTIAL_KEYCHAIN_SERVICE: &str = "in.tradeautopsy.station.broker-credentials";

/// Kotak Neo session vault only — avoids sharing SecAccess with Station HMAC items.
pub const KOTAK_SESSION_KEYCHAIN_SERVICE: &str = "in.tradeautopsy.station.kotak-session-vault";

/// Keychain `kSecAttrService` for a catalog slug.
pub fn keychain_service_for(broker_slug: &str) -> &'static str {
    match broker_slug {
        "kotak_neo" | "kotak" => KOTAK_SESSION_KEYCHAIN_SERVICE,
        _ => BROKER_CREDENTIAL_KEYCHAIN_SERVICE,
    }
}

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
    #[serde(rename = "kite_checksum_session")]
    KiteChecksumSession {
        #[serde(rename = "apiKey")]
        api_key: String,
        #[serde(rename = "apiSecret")]
        api_secret: String,
        #[serde(rename = "accessToken")]
        access_token: String,
        #[serde(rename = "accessTokenExpiryUnixMs")]
        access_token_expiry_unix_ms: i64,
        #[serde(rename = "userId")]
        user_id: String,
    },
    #[serde(rename = "upstox_oauth_bearer_session")]
    UpstoxOAuthBearerSession {
        #[serde(rename = "clientId")]
        client_id: String,
        #[serde(rename = "clientSecret")]
        client_secret: String,
        #[serde(rename = "accessToken")]
        access_token: String,
        #[serde(rename = "accessTokenExpiryUnixMs")]
        access_token_expiry_unix_ms: i64,
        #[serde(rename = "userId")]
        user_id: String,
        #[serde(rename = "redirectUri")]
        redirect_uri: String,
    },
    #[serde(rename = "fyers_oauth_json_session")]
    FyersOAuthJsonAppIdHashSession {
        #[serde(rename = "appId")]
        app_id: String,
        #[serde(rename = "secretId")]
        secret_id: String,
        #[serde(rename = "accessToken")]
        access_token: String,
        #[serde(default, rename = "refreshToken")]
        refresh_token: Option<String>,
        #[serde(rename = "accessTokenExpiryUnixMs")]
        access_token_expiry_unix_ms: i64,
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
        /// OMS server id — required as trade-book query `sId` (SDK TradeReportAPI).
        #[serde(default, rename = "hsServerId")]
        hs_server_id: String,
        #[serde(default, rename = "expiresAt")]
        expires_at: Option<String>,
    },
    #[serde(rename = "dhan_consent_session")]
    DhanConsentSession {
        #[serde(rename = "dhanClientId")]
        dhan_client_id: String,
        #[serde(rename = "appId")]
        app_id: String,
        #[serde(rename = "appSecret")]
        app_secret: String,
        #[serde(rename = "accessToken")]
        access_token: String,
        #[serde(rename = "expiryTime")]
        expiry_time: String,
    },
    #[serde(rename = "okx_passphrase_session")]
    OkxPassphraseSession {
        #[serde(rename = "apiKey")]
        api_key: String,
        #[serde(rename = "apiSecret")]
        api_secret: String,
        passphrase: String,
    },
    #[serde(rename = "coinbase_jwt_es256_session")]
    CoinbaseJwtEs256Session {
        /// CDP API key name (`kid` / JWT `sub`).
        #[serde(rename = "apiKey")]
        api_key: String,
        /// PEM-encoded EC private key (vault-only; never in Wasm).
        #[serde(rename = "pemPrivateKey")]
        pem_private_key: String,
    },
    #[serde(rename = "groww_checksum_session")]
    GrowwChecksumSession {
        #[serde(rename = "apiKey")]
        api_key: String,
        /// Vault-only: host presents it at mint (checksum preimage); never leaves the agent.
        #[serde(rename = "apiSecret")]
        api_secret: String,
        /// Minted session token (`token`); Bearer-attached by the host on private paths.
        #[serde(rename = "token")]
        token: String,
        /// ISO-8601 as returned by the mint (`expiry`); read per mint, never derived.
        #[serde(rename = "expiry")]
        expiry: String,
        #[serde(rename = "tokenRefId")]
        token_ref_id: String,
        /// Mint time (ISO-8601); `checksum` and `timestamp` are NOT fields — recomputed per mint.
        #[serde(rename = "mintedAt")]
        minted_at: String,
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
            Self::KiteChecksumSession { api_key, .. } => Some(api_key),
            Self::UpstoxOAuthBearerSession { client_id, .. } => Some(client_id),
            Self::FyersOAuthJsonAppIdHashSession { app_id, .. } => Some(app_id),
            Self::OkxPassphraseSession { api_key, .. } => Some(api_key),
            Self::CoinbaseJwtEs256Session { api_key, .. } => Some(api_key),
            Self::GrowwChecksumSession { api_key, .. } => Some(api_key),
            Self::DhanConsentSession { app_id, .. } => Some(app_id),
            Self::KotakNeoTotpSession { .. } => None,
        }
    }
}

/// Decode Keychain JSON: tagged blob, or legacy flat `{apiKey,apiSecret}`.
pub fn decode_credential_blob(json: &str) -> anyhow::Result<CredentialBlob> {
    if let Ok(blob) = serde_json::from_str::<CredentialBlob>(json) {
        return Ok(blob);
    }
    // Station app-only OAuth JSON shares `apiKey`/`apiSecret` but lacks session fields — serde
    // rejects it above; legacy decode would wrongly treat it as Binance-style HMAC.
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(scheme) = value.get("authScheme").and_then(|v| v.as_str()) {
            if scheme != "hmac_api_key_secret" {
                anyhow::bail!(
                    "incomplete session credential blob (authScheme={scheme}); finish Connect in Station"
                );
            }
        }
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
    fn station_kite_app_only_json_does_not_decode_as_hmac() {
        let json = r#"{"authScheme":"kite_checksum_session","apiKey":"k","apiSecret":"s"}"#;
        let err = decode_credential_blob(json).unwrap_err();
        assert!(
            err.to_string().contains("kite_checksum_session"),
            "{err}"
        );
    }

    #[test]
    fn station_oauth_app_only_json_never_decodes_as_hmac() {
        for json in [
            r#"{"authScheme":"upstox_oauth_bearer_session","apiKey":"c","apiSecret":"s"}"#,
            r#"{"authScheme":"fyers_oauth_json_app_id_hash_session","apiKey":"a","apiSecret":"s"}"#,
            r#"{"authScheme":"groww_checksum_session","apiKey":"k","apiSecret":"s"}"#,
            r#"{"authScheme":"dhan_consent_session","apiKey":"a","apiSecret":"s"}"#,
        ] {
            assert!(
                decode_credential_blob(json).is_err(),
                "expected error for {json}"
            );
        }
    }

    #[test]
    fn kite_checksum_session_blob_roundtrip() {
        let blob = CredentialBlob::KiteChecksumSession {
            api_key: "key".into(),
            api_secret: "sec".into(),
            access_token: "at".into(),
            access_token_expiry_unix_ms: 1_700_000_000_000,
            user_id: "AB1234".into(),
        };
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("kite_checksum_session"));
        assert!(json.contains("accessTokenExpiryUnixMs"));
        let decoded = decode_credential_blob(&json).unwrap();
        assert_eq!(decoded, blob);
    }

    #[test]
    fn groww_checksum_session_blob_roundtrip() {
        let blob = CredentialBlob::GrowwChecksumSession {
            api_key: "groww-key".into(),
            api_secret: "vault-secret".into(),
            token: "minted-token".into(),
            expiry: "2026-09-25T06:00:00+05:30".into(),
            token_ref_id: "ref-1".into(),
            minted_at: "2026-09-24T18:00:00+05:30".into(),
        };
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("groww_checksum_session"));
        assert!(json.contains("apiKey"));
        assert!(json.contains("tokenRefId"));
        assert!(json.contains("mintedAt"));
        // Per-mint inputs are NEVER blob fields — recomputed at each mint (ADR 0014).
        assert!(!json.contains(r#""checksum""#));
        assert!(!json.contains(r#""timestamp""#));
        let decoded = decode_credential_blob(&json).unwrap();
        assert_eq!(decoded, blob);
        assert_eq!(blob.api_key_for_tests(), Some("groww-key"));
    }

    #[test]
    fn kotak_session_blob_roundtrip() {
        let blob = CredentialBlob::KotakNeoTotpSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid1".into(),
            base_url: "https://cis.kotaksecurities.com".into(),
            hs_server_id: "server4".into(),
            expires_at: Some("2026-07-25T18:00:00Z".into()),
        };
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("hsServerId"));
        let decoded = decode_credential_blob(&json).unwrap();
        assert_eq!(decoded, blob);
    }

    #[test]
    fn kotak_session_blob_defaults_missing_hs_server_id() {
        let json = r#"{
            "authScheme":"kotak_neo_totp_session",
            "consumerKey":"ck",
            "tradeToken":"tt",
            "sid":"sid1",
            "baseUrl":"https://cis.kotaksecurities.com"
        }"#;
        let decoded = decode_credential_blob(json).unwrap();
        match decoded {
            CredentialBlob::KotakNeoTotpSession { hs_server_id, .. } => {
                assert!(hs_server_id.is_empty());
            }
            _ => panic!("expected kotak"),
        }
    }

    #[test]
    fn okx_passphrase_session_blob_roundtrip() {
        let blob = CredentialBlob::OkxPassphraseSession {
            api_key: "key".into(),
            api_secret: "sec".into(),
            passphrase: "phrase".into(),
        };
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("okx_passphrase_session"));
        assert!(json.contains("passphrase"));
        let decoded = decode_credential_blob(&json).unwrap();
        assert_eq!(decoded, blob);
    }

    #[test]
    fn keychain_service_routes_kotak_to_dedicated_vault() {
        assert_eq!(
            keychain_service_for("kotak_neo"),
            KOTAK_SESSION_KEYCHAIN_SERVICE
        );
        assert_eq!(
            keychain_service_for("kotak"),
            KOTAK_SESSION_KEYCHAIN_SERVICE
        );
        assert_eq!(
            keychain_service_for("binance_com"),
            BROKER_CREDENTIAL_KEYCHAIN_SERVICE
        );
        assert_ne!(
            KOTAK_SESSION_KEYCHAIN_SERVICE,
            BROKER_CREDENTIAL_KEYCHAIN_SERVICE
        );
    }
}
