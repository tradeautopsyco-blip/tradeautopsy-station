//! OKX REST signing + Connect validation (host-side).
//!
//! ADR 0016 · B6 `okx_com` rows 10/16. Three-field API key: key, secret, passphrase.
//! Keys are created on **www.okx.com** (global), never US/EEA hosts.

use crate::ubi::credentials::CredentialBlob;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const OKX_API_HOST: &str = "www.okx.com";

/// Sibling venues refused on the global book (B6 row 0 / row 12).
pub const OKX_REFUSED_HOSTS: &[&str] = &["us.okx.com", "eea.okx.com"];

/// ISO-8601 timestamp with millisecond precision (`OK-ACCESS-TIMESTAMP`).
pub fn okx_access_timestamp_iso(now_unix_ms: i64) -> String {
    let dt = chrono::DateTime::from_timestamp_millis(now_unix_ms).unwrap_or_else(chrono::Utc::now);
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Preimage: `{timestamp}{METHOD}{requestPath}{body}` — GET body is empty (official docs).
pub fn okx_sign_preimage(
    timestamp: &str,
    method: &str,
    request_path: &str,
    body: &str,
) -> String {
    format!(
        "{timestamp}{}{request_path}{body}",
        method.to_ascii_uppercase()
    )
}

/// HMAC-SHA256 secret → Base64 (`OK-ACCESS-SIGN`).
pub fn okx_hmac_sign_base64(api_secret: &str, preimage: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(preimage.as_bytes());
    BASE64_STANDARD.encode(mac.finalize().into_bytes())
}

pub fn okx_sign_request(
    api_secret: &str,
    timestamp: &str,
    method: &str,
    request_path: &str,
    body: Option<&str>,
) -> String {
    let body = body.unwrap_or("");
    let preimage = okx_sign_preimage(timestamp, method, request_path, body);
    okx_hmac_sign_base64(api_secret, &preimage)
}

/// Build the tagged Keychain blob after Connect key entry (no mint — per-request signing).
pub fn connect_store(
    api_key: impl Into<String>,
    api_secret: impl Into<String>,
    passphrase: impl Into<String>,
) -> CredentialBlob {
    CredentialBlob::OkxPassphraseSession {
        api_key: api_key.into(),
        api_secret: api_secret.into(),
        passphrase: passphrase.into(),
    }
}

pub trait OkxSessionHttp: Send + Sync {
    fn get_signed(
        &self,
        request_path: &str,
        headers: &[(String, String)],
    ) -> Result<(u16, String), String>;
}

/// Lightweight auth probe: `GET /api/v5/account/balance` on `www.okx.com`.
pub fn validate_credentials(
    http: &dyn OkxSessionHttp,
    api_key: &str,
    api_secret: &str,
    passphrase: &str,
) -> Result<(), String> {
    if api_key.trim().is_empty() || api_secret.trim().is_empty() || passphrase.trim().is_empty() {
        return Err("api key, secret, and passphrase are required".into());
    }
    let path = "/api/v5/account/balance";
    let timestamp = okx_access_timestamp_iso(chrono::Utc::now().timestamp_millis());
    let sign = okx_sign_request(api_secret, &timestamp, "GET", path, None);
    let headers = vec![
        ("OK-ACCESS-KEY".into(), api_key.trim().to_string()),
        ("OK-ACCESS-SIGN".into(), sign),
        ("OK-ACCESS-TIMESTAMP".into(), timestamp),
        ("OK-ACCESS-PASSPHRASE".into(), passphrase.trim().to_string()),
        ("Accept".into(), "application/json".into()),
    ];
    let (status, body) = http.get_signed(path, &headers)?;
    if status == 401 || status == 403 {
        return Err("invalid okx credentials".into());
    }
    if status != 200 {
        return Err(format!("okx balance probe http {status}"));
    }
    let root: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("balance json: {e}"))?;
    if root.get("code").and_then(|v| v.as_str()) != Some("0") {
        return Err(format!(
            "okx balance probe rejected: {}",
            root.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_doc_get_balance_vector() {
        let secret = "chNOOS4KvNXR_Xq4k4c9qsfoKWvnDecLATCRlcBwyKDYnWgO";
        let timestamp = "2020-12-08T09:08:57.715Z";
        let sign = okx_sign_request(secret, timestamp, "GET", "/api/v5/account/balance", None);
        assert_eq!(sign, "PJ61e1nb2F2Qd7D8SPiaIcx2gjdELc+o0ygzre9z33k=");
    }

    #[test]
    fn get_with_query_in_sign_path() {
        let secret = "chNOOS4KvNXR_Xq4k4c9qsfoKWvnDecLATCRlcBwyKDYnWgO";
        let timestamp = "2020-12-08T09:08:57.715Z";
        let sign = okx_sign_request(
            secret,
            timestamp,
            "GET",
            "/api/v5/account/balance?ccy=BTC",
            None,
        );
        assert!(!sign.is_empty());
        BASE64_STANDARD.decode(sign).unwrap();
    }

    #[test]
    fn refused_hosts_are_not_prod() {
        assert_eq!(OKX_API_HOST, "www.okx.com");
        assert!(OKX_REFUSED_HOSTS.contains(&"us.okx.com"));
        assert!(OKX_REFUSED_HOSTS.contains(&"eea.okx.com"));
    }
}
