//! Zerodha Kite Connect session mint (host-side checksum exchange).
//!
//! `request_token` is single-use and never persisted (ADR 0005). See
//! `docs/reference/india/zerodha-kite/REST.md`.

use crate::ubi::credentials::CredentialBlob;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const KITE_API_HOST: &str = "api.kite.trade";
pub const KITE_SESSION_TOKEN_PATH: &str = "/session/token";
pub const KITE_API_VERSION_HEADER: &str = "3";

const CONNECT_STATE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone)]
pub struct PendingZerodhaConnect {
    pub environment: String,
    pub connection_id: String,
    pub api_key: String,
    pub api_secret: String,
    pub expires_at_unix_ms: i64,
}

static PENDING: OnceLock<Mutex<HashMap<String, PendingZerodhaConnect>>> = OnceLock::new();

fn pending_store() -> &'static Mutex<HashMap<String, PendingZerodhaConnect>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// SHA-256(`api_key` + `request_token` + `api_secret`) — no separators (Kite v3).
pub fn kite_login_checksum(api_key: &str, request_token: &str, api_secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{api_key}{request_token}{api_secret}").as_bytes());
    hex::encode(hasher.finalize())
}

/// `Authorization` value for authenticated Kite REST calls.
pub fn kite_authorization_header_value(api_key: &str, access_token: &str) -> String {
    format!("token {api_key}:{access_token}")
}

pub fn zerodha_callback_base_url() -> String {
    crate::oauth_loopback::https_oauth_callback_url("/api/daemon/broker/zerodha/callback")
}

pub fn kite_login_url(api_key: &str, state: &str) -> String {
    let redirect = zerodha_callback_base_url();
    format!(
        "https://kite.zerodha.com/connect/login?v=3&api_key={}&redirect_uri={}&state={}",
        url_encode_component(api_key),
        url_encode_component(&redirect),
        url_encode_component(state),
    )
}

fn url_encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Mint a single-use OAuth `state` nonce bound to the connection (ADR 0005).
pub fn begin_connect(
    environment: &str,
    connection_id: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<(String, String), String> {
    if environment.trim().is_empty() || connection_id.trim().is_empty() {
        return Err("environment and connection_id are required".into());
    }
    if api_key.trim().is_empty() || api_secret.trim().is_empty() {
        return Err("api_key and api_secret are required".into());
    }
    purge_expired_pending();
    let state = generate_state_nonce();
    let login_url = kite_login_url(api_key.trim(), &state);
    let pending = PendingZerodhaConnect {
        environment: environment.trim().to_string(),
        connection_id: connection_id.trim().to_string(),
        api_key: api_key.trim().to_string(),
        api_secret: api_secret.trim().to_string(),
        expires_at_unix_ms: now_unix_ms() + CONNECT_STATE_TTL.as_millis() as i64,
    };
    pending_store()
        .lock()
        .map_err(|_| "connect pending lock poisoned".to_string())?
        .insert(state.clone(), pending);
    Ok((state, login_url))
}

fn generate_state_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    hex::encode(bytes)
}

fn purge_expired_pending() {
    let now = now_unix_ms();
    if let Ok(mut map) = pending_store().lock() {
        map.retain(|_, v| v.expires_at_unix_ms > now);
    }
}

/// Consume a pending `state` (single-use). Returns None if missing, expired, or reused.
pub fn take_pending_connect(state: &str) -> Option<PendingZerodhaConnect> {
    purge_expired_pending();
    let state = state.trim();
    if state.is_empty() {
        return None;
    }
    let mut map = pending_store().lock().ok()?;
    let pending = map.remove(state)?;
    if pending.expires_at_unix_ms <= now_unix_ms() {
        return None;
    }
    Some(pending)
}

/// Kite often omits `state` on the loopback redirect — if exactly one connect is pending, use it.
pub fn take_single_active_pending_connect() -> Option<PendingZerodhaConnect> {
    purge_expired_pending();
    let mut map = pending_store().lock().ok()?;
    let now = now_unix_ms();
    let keys: Vec<String> = map
        .iter()
        .filter(|(_, v)| v.expires_at_unix_ms > now)
        .map(|(k, _)| k.clone())
        .collect();
    if keys.len() != 1 {
        return None;
    }
    let key = keys[0].clone();
    let pending = map.remove(&key)?;
    if pending.expires_at_unix_ms <= now {
        return None;
    }
    Some(pending)
}

#[derive(Debug, Clone)]
pub struct MintedKiteSession {
    pub api_key: String,
    pub api_secret: String,
    pub access_token: String,
    pub access_token_expiry_unix_ms: i64,
    pub user_id: String,
}

impl MintedKiteSession {
    pub fn into_credential_blob(self) -> CredentialBlob {
        CredentialBlob::KiteChecksumSession {
            api_key: self.api_key,
            api_secret: self.api_secret,
            access_token: self.access_token,
            access_token_expiry_unix_ms: self.access_token_expiry_unix_ms,
            user_id: self.user_id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum KiteExchangeErrorClass {
    InvalidRequest,
    Upstream,
}

impl KiteExchangeErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_credentials",
            Self::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KiteExchangeError {
    pub class: KiteExchangeErrorClass,
    pub message: String,
}

pub trait KiteSessionHttp: Send + Sync {
    fn post_form(
        &self,
        url: &str,
        headers: &[(String, String)],
        fields: &[(String, String)],
    ) -> Result<(u16, String), String>;
}

#[derive(Debug, Deserialize)]
struct KiteEnvelope {
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Option<KiteTokenData>,
}

#[derive(Debug, Deserialize)]
struct KiteTokenData {
    #[serde(rename = "user_id")]
    user_id: String,
    #[serde(rename = "access_token")]
    access_token: String,
    #[serde(default, rename = "login_time")]
    login_time: Option<String>,
}

pub fn exchange_request_token(
    http: &dyn KiteSessionHttp,
    api_key: &str,
    api_secret: &str,
    request_token: &str,
    now_unix_ms: i64,
) -> Result<MintedKiteSession, KiteExchangeError> {
    if api_key.trim().is_empty() || api_secret.trim().is_empty() || request_token.trim().is_empty()
    {
        return Err(KiteExchangeError {
            class: KiteExchangeErrorClass::InvalidRequest,
            message: "api_key, api_secret, and request_token are required".into(),
        });
    }
    let checksum = kite_login_checksum(api_key, request_token, api_secret);
    let url = format!("https://{KITE_API_HOST}{KITE_SESSION_TOKEN_PATH}");
    let headers = vec![
        ("X-Kite-Version".into(), KITE_API_VERSION_HEADER.into()),
        (
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into(),
        ),
    ];
    let fields = vec![
        ("api_key".into(), api_key.to_string()),
        ("request_token".into(), request_token.to_string()),
        ("checksum".into(), checksum),
    ];
    let (status, body) = http
        .post_form(&url, &headers, &fields)
        .map_err(|e| KiteExchangeError {
            class: KiteExchangeErrorClass::Upstream,
            message: e,
        })?;
    if status != 200 {
        return Err(KiteExchangeError {
            class: KiteExchangeErrorClass::Upstream,
            message: format!("kite session exchange HTTP {status}"),
        });
    }
    let envelope: KiteEnvelope = serde_json::from_str(&body).map_err(|_| KiteExchangeError {
        class: KiteExchangeErrorClass::Upstream,
        message: "kite session exchange: invalid JSON".into(),
    })?;
    if envelope.status != "success" {
        return Err(KiteExchangeError {
            class: KiteExchangeErrorClass::InvalidRequest,
            message: if envelope.message.is_empty() {
                "kite session exchange failed".into()
            } else {
                envelope.message
            },
        });
    }
    let data = envelope.data.ok_or_else(|| KiteExchangeError {
        class: KiteExchangeErrorClass::Upstream,
        message: "kite session exchange: missing data".into(),
    })?;
    let expiry = access_token_expiry_unix_ms(now_unix_ms, data.login_time.as_deref());
    Ok(MintedKiteSession {
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
        access_token: data.access_token,
        access_token_expiry_unix_ms: expiry,
        user_id: data.user_id,
    })
}

/// Next 6:00 IST boundary after `now_unix_ms` (regulatory daily token TTL; B6 / Kite docs).
pub fn access_token_expiry_unix_ms(now_unix_ms: i64, _login_time: Option<&str>) -> i64 {
    let now_secs = now_unix_ms / 1000;
    let ist_offset_secs = 5 * 3600 + 30 * 60;
    let ist_secs = now_secs + ist_offset_secs;
    let ist_day = ist_secs / 86_400;
    let ist_seconds_today = ist_secs % 86_400;
    let six_am = 6 * 3600;
    let target_day = if ist_seconds_today < six_am {
        ist_day
    } else {
        ist_day + 1
    };
    let target_ist_secs = target_day * 86_400 + six_am;
    (target_ist_secs - ist_offset_secs) * 1000
}

pub struct ReqwestKiteSessionHttp {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl ReqwestKiteSessionHttp {
    pub fn new() -> Result<Self, String> {
        let client = crate::egress::shared_client();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("kite http runtime: {e}"))?;
        Ok(Self { client, runtime })
    }
}

impl Default for ReqwestKiteSessionHttp {
    fn default() -> Self {
        Self::new().expect("kite session http")
    }
}

impl KiteSessionHttp for ReqwestKiteSessionHttp {
    fn post_form(
        &self,
        url: &str,
        headers: &[(String, String)],
        fields: &[(String, String)],
    ) -> Result<(u16, String), String> {
        let body = fields
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode_component(k), url_encode_component(v)))
            .collect::<Vec<_>>()
            .join("&");
        self.runtime.block_on(async {
            let mut builder = self.client.post(url).body(body);
            for (name, value) in headers {
                builder = builder.header(name.as_str(), value.as_str());
            }
            let response = builder
                .send()
                .await
                .map_err(|e| format!("kite http: {e}"))?;
            let status = response.status().as_u16();
            let text = response
                .text()
                .await
                .map_err(|e| format!("kite http body: {e}"))?;
            Ok((status, text))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHttp {
        last_form: std::sync::Mutex<Option<Vec<(String, String)>>>,
        response: (u16, String),
    }

    impl MockHttp {
        fn ok_json(body: &str) -> Self {
            Self {
                last_form: std::sync::Mutex::new(None),
                response: (200, body.into()),
            }
        }
    }

    impl KiteSessionHttp for MockHttp {
        fn post_form(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            fields: &[(String, String)],
        ) -> Result<(u16, String), String> {
            *self.last_form.lock().unwrap() = Some(fields.to_vec());
            Ok(self.response.clone())
        }
    }

    #[test]
    fn checksum_matches_kite_v3_concat_rule() {
        let api_key = "api_key";
        let request_token = "request_token";
        let api_secret = "api_secret";
        let checksum = kite_login_checksum(api_key, request_token, api_secret);
        let mut hasher = Sha256::new();
        hasher.update(b"api_keyrequest_tokenapi_secret");
        assert_eq!(checksum, hex::encode(hasher.finalize()));
    }

    #[test]
    fn authorization_header_uses_token_scheme() {
        assert_eq!(
            kite_authorization_header_value("mykey", "mytoken"),
            "token mykey:mytoken"
        );
    }

    #[test]
    fn begin_mints_state_and_login_url_without_persisting_request_token() {
        let (state, login) = begin_connect("prod", "conn-1", "abc123", "secret").unwrap();
        assert_eq!(state.len(), 32);
        assert!(login.contains("api_key=abc123"));
        assert!(login.contains("redirect_uri="));
        assert!(login.contains("state="));
        assert!(login.contains(&state));
        assert!(take_pending_connect(&state).is_some());
        assert!(take_pending_connect(&state).is_none(), "state is single-use");
    }

    #[test]
    fn single_active_pending_fallback_when_kite_omits_state() {
        let (_state, _) = begin_connect("prod", "conn-1", "abc123", "secret").unwrap();
        let pending = take_single_active_pending_connect().expect("one active");
        assert_eq!(pending.connection_id, "conn-1");
        assert!(take_single_active_pending_connect().is_none());
    }

    #[test]
    fn exchange_posts_checksum_and_builds_blob_fields() {
        let body = r#"{"status":"success","data":{"user_id":"AB1234","access_token":"atok","login_time":"2026-09-23 09:00:00"}}"#;
        let http = MockHttp::ok_json(body);
        let minted = exchange_request_token(
            &http,
            "key1",
            "sec1",
            "rtok",
            1_700_000_000_000,
        )
        .unwrap();
        let form = http.last_form.lock().unwrap().clone().unwrap();
        let checksum = form
            .iter()
            .find(|(k, _)| k == "checksum")
            .map(|(_, v)| v.as_str())
            .unwrap();
        assert_eq!(checksum, kite_login_checksum("key1", "rtok", "sec1"));
        assert_eq!(minted.user_id, "AB1234");
        assert_eq!(minted.access_token, "atok");
        let blob = minted.into_credential_blob();
        match blob {
            CredentialBlob::KiteChecksumSession {
                access_token,
                user_id,
                ..
            } => {
                assert_eq!(access_token, "atok");
                assert_eq!(user_id, "AB1234");
            }
            _ => panic!("expected kite blob"),
        }
    }
}
