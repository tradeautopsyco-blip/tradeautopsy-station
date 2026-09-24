//! Fyers OAuth JSON appIdHash session mint (host-side validate-authcode exchange).
//!
//! ADR 0007 · B6 `fyers` rows 10/14–16. `auth_code` is single-use and never persisted.

use crate::ubi::credentials::CredentialBlob;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const FYERS_API_HOST: &str = "api-t1.fyers.in";
pub const FYERS_GENERATE_AUTHCODE_PATH: &str = "/api/v3/generate-authcode";
pub const FYERS_VALIDATE_AUTHCODE_PATH: &str = "/api/v3/validate-authcode";
pub const FYERS_USER_AGENT: &str = "TradeAutopsy-Station/1.0";

const CONNECT_STATE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone)]
pub struct PendingFyersConnect {
    pub environment: String,
    pub connection_id: String,
    pub app_id: String,
    pub secret_id: String,
    pub redirect_uri: String,
    pub expires_at_unix_ms: i64,
}

static PENDING: OnceLock<Mutex<HashMap<String, PendingFyersConnect>>> = OnceLock::new();

fn pending_store() -> &'static Mutex<HashMap<String, PendingFyersConnect>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// SHA-256 hex digest of `"<app_id>:<secret_id>"` (official appIdHash).
pub fn fyers_app_id_hash(app_id: &str, secret_id: &str) -> String {
    let input = format!("{}:{}", app_id.trim(), secret_id.trim());
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn fyers_authorization_header_value(app_id: &str, access_token: &str) -> String {
    format!("{}:{}", app_id.trim(), access_token.trim())
}

pub fn fyers_callback_base_url() -> String {
    crate::oauth_loopback::https_oauth_callback_url("/api/daemon/broker/fyers/callback")
}

pub fn fyers_authorize_url(app_id: &str, state: &str) -> String {
    let redirect = fyers_callback_base_url();
    format!(
        "https://{FYERS_API_HOST}{FYERS_GENERATE_AUTHCODE_PATH}?client_id={}&redirect_uri={}&response_type=code&state={}",
        url_encode_component(app_id),
        url_encode_component(&redirect),
        url_encode_component(state)
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

/// Mint OAuth `state` + generate-authcode URL (ADR 0007).
pub fn begin_connect(
    environment: &str,
    connection_id: &str,
    app_id: &str,
    secret_id: &str,
) -> Result<(String, String), String> {
    if environment.trim().is_empty() || connection_id.trim().is_empty() {
        return Err("environment and connection_id are required".into());
    }
    if app_id.trim().is_empty() || secret_id.trim().is_empty() {
        return Err("app_id and secret_id are required".into());
    }
    purge_expired_pending();
    let state = generate_state_nonce();
    let redirect_uri = fyers_callback_base_url();
    let login_url = fyers_authorize_url(app_id.trim(), &state);
    let pending = PendingFyersConnect {
        environment: environment.trim().to_string(),
        connection_id: connection_id.trim().to_string(),
        app_id: app_id.trim().to_string(),
        secret_id: secret_id.trim().to_string(),
        redirect_uri,
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

pub fn take_pending_connect(state: &str) -> Option<PendingFyersConnect> {
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

#[derive(Debug, Clone)]
pub struct MintedFyersSession {
    pub app_id: String,
    pub secret_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub access_token_expiry_unix_ms: i64,
}

impl MintedFyersSession {
    pub fn into_credential_blob(self) -> CredentialBlob {
        CredentialBlob::FyersOAuthJsonAppIdHashSession {
            app_id: self.app_id,
            secret_id: self.secret_id,
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            access_token_expiry_unix_ms: self.access_token_expiry_unix_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FyersExchangeErrorClass {
    InvalidRequest,
    SessionExpired,
    Upstream,
}

impl FyersExchangeErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_credentials",
            Self::SessionExpired => "session_expired",
            Self::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FyersExchangeError {
    pub class: FyersExchangeErrorClass,
    pub message: String,
}

pub trait FyersSessionHttp: Send + Sync {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &str,
    ) -> Result<(u16, String), String>;
}

#[derive(Debug, Deserialize)]
struct FyersValidateEnvelope {
    #[serde(default)]
    s: String,
    #[serde(default)]
    code: i32,
    #[serde(default)]
    message: String,
    #[serde(default, rename = "access_token")]
    access_token: Option<String>,
    #[serde(default, rename = "refresh_token")]
    refresh_token: Option<String>,
}

pub fn exchange_auth_code(
    http: &dyn FyersSessionHttp,
    app_id: &str,
    secret_id: &str,
    auth_code: &str,
    now_unix_ms: i64,
) -> Result<MintedFyersSession, FyersExchangeError> {
    if app_id.trim().is_empty() || secret_id.trim().is_empty() || auth_code.trim().is_empty() {
        return Err(FyersExchangeError {
            class: FyersExchangeErrorClass::InvalidRequest,
            message: "app_id, secret_id, and auth_code are required".into(),
        });
    }
    let app_id_hash = fyers_app_id_hash(app_id, secret_id);
    let payload = serde_json::json!({
        "grant_type": "authorization_code",
        "appIdHash": app_id_hash,
        "code": auth_code.trim(),
    });
    let body = payload.to_string();
    let url = format!("https://{FYERS_API_HOST}{FYERS_VALIDATE_AUTHCODE_PATH}");
    let headers = vec![
        ("Content-Type".into(), "application/json".into()),
        ("User-Agent".into(), FYERS_USER_AGENT.into()),
    ];
    let (status, response_body) = http.post_json(&url, &headers, &body).map_err(|e| {
        FyersExchangeError {
            class: FyersExchangeErrorClass::Upstream,
            message: e,
        }
    })?;
    if status == 401 || status == 403 || body_signals_invalid_token(&response_body) {
        return Err(FyersExchangeError {
            class: FyersExchangeErrorClass::SessionExpired,
            message: "fyers validate-authcode: invalid or expired authorization".into(),
        });
    }
    if status != 200 {
        return Err(FyersExchangeError {
            class: FyersExchangeErrorClass::Upstream,
            message: format!("fyers validate-authcode HTTP {status}"),
        });
    }
    let envelope: FyersValidateEnvelope =
        serde_json::from_str(&response_body).map_err(|_| FyersExchangeError {
            class: FyersExchangeErrorClass::Upstream,
            message: "fyers validate-authcode: invalid JSON".into(),
        })?;
    if envelope.s.eq_ignore_ascii_case("error")
        || matches!(envelope.code, -8 | -15 | -16 | -17)
        || body_signals_invalid_token(&envelope.message)
    {
        let class = if body_signals_invalid_token(&envelope.message)
            || matches!(envelope.code, -8 | -15 | -16 | -17)
        {
            FyersExchangeErrorClass::SessionExpired
        } else {
            FyersExchangeErrorClass::InvalidRequest
        };
        return Err(FyersExchangeError {
            class,
            message: if envelope.message.is_empty() {
                "fyers validate-authcode failed".into()
            } else {
                envelope.message
            },
        });
    }
    let access_token = envelope.access_token.filter(|t| !t.trim().is_empty()).ok_or(
        FyersExchangeError {
            class: FyersExchangeErrorClass::Upstream,
            message: "fyers validate-authcode: missing access_token".into(),
        },
    )?;
    let refresh_token = envelope
        .refresh_token
        .filter(|t| !t.trim().is_empty());
    let expiry = access_token_expiry_unix_ms(now_unix_ms);
    Ok(MintedFyersSession {
        app_id: app_id.to_string(),
        secret_id: secret_id.to_string(),
        access_token,
        refresh_token,
        access_token_expiry_unix_ms: expiry,
    })
}

fn body_signals_invalid_token(body: &str) -> bool {
    let head = &body[..body.len().min(2048)];
    head.contains("\"code\":-8")
        || head.contains("\"code\": -8")
        || head.contains("\"code\":-15")
        || head.contains("\"code\":-16")
        || head.contains("\"code\":-17")
        || head.contains("Invalid token")
        || head.contains("invalid token")
        || head.contains("Token expired")
}

/// Next 3:30 AM IST boundary after `now_unix_ms` (Fyers daily token TTL; B6 row 16).
pub fn access_token_expiry_unix_ms(now_unix_ms: i64) -> i64 {
    let now_secs = now_unix_ms / 1000;
    let ist_offset_secs = 5 * 3600 + 30 * 60;
    let ist_secs = now_secs + ist_offset_secs;
    let ist_day = ist_secs / 86_400;
    let ist_seconds_today = ist_secs % 86_400;
    let boundary = 3 * 3600 + 30 * 60;
    let target_day = if ist_seconds_today < boundary {
        ist_day
    } else {
        ist_day + 1
    };
    let target_ist_secs = target_day * 86_400 + boundary;
    (target_ist_secs - ist_offset_secs) * 1000
}

pub struct ReqwestFyersSessionHttp {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl ReqwestFyersSessionHttp {
    pub fn new() -> Result<Self, String> {
        let client = crate::egress::shared_client();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("fyers http runtime: {e}"))?;
        Ok(Self { client, runtime })
    }
}

impl Default for ReqwestFyersSessionHttp {
    fn default() -> Self {
        Self::new().expect("fyers session http")
    }
}

impl FyersSessionHttp for ReqwestFyersSessionHttp {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &str,
    ) -> Result<(u16, String), String> {
        self.runtime.block_on(async {
            let mut builder = self.client.post(url).body(body.to_string());
            for (name, value) in headers {
                builder = builder.header(name.as_str(), value.as_str());
            }
            let response = builder
                .send()
                .await
                .map_err(|e| format!("fyers http: {e}"))?;
            let status = response.status().as_u16();
            let text = response
                .text()
                .await
                .map_err(|e| format!("fyers http body: {e}"))?;
            Ok((status, text))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHttp {
        last_json: std::sync::Mutex<Option<String>>,
        response: (u16, String),
    }

    impl MockHttp {
        fn ok_json(body: &str) -> Self {
            Self {
                last_json: std::sync::Mutex::new(None),
                response: (200, body.into()),
            }
        }
    }

    impl FyersSessionHttp for MockHttp {
        fn post_json(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            body: &str,
        ) -> Result<(u16, String), String> {
            *self.last_json.lock().unwrap() = Some(body.to_string());
            Ok(self.response.clone())
        }
    }

    #[test]
    fn app_id_hash_matches_official_concat_sha256_hex() {
        let hash = fyers_app_id_hash("APP-100", "secret");
        let mut hasher = Sha256::new();
        hasher.update(b"APP-100:secret");
        assert_eq!(hash, hex::encode(hasher.finalize()));
    }

    #[test]
    fn authorize_url_includes_generate_authcode_params() {
        let url = fyers_authorize_url("APP-100", "abcstate");
        assert!(url.contains(FYERS_GENERATE_AUTHCODE_PATH));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=APP-100"));
        assert!(url.contains("state=abcstate"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("%2Fapi%2Fdaemon%2Fbroker%2Ffyers%2Fcallback"));
    }

    #[test]
    fn callback_base_url_uses_agent_port_default() {
        std::env::remove_var("AGENT_PORT");
        assert!(fyers_callback_base_url().starts_with("https://127.0.0.1:9140/"));
    }

    #[test]
    fn authorization_header_uses_app_id_colon_token() {
        assert_eq!(
            fyers_authorization_header_value("APP-100", "jwt.tok"),
            "APP-100:jwt.tok"
        );
    }

    #[test]
    fn access_token_expiry_targets_next_330am_ist() {
        let ten_utc_ms = 1_759_632_000_000i64;
        let expiry = access_token_expiry_unix_ms(ten_utc_ms);
        let expiry_secs = expiry / 1000;
        let ist_offset = 5 * 3600 + 30 * 60;
        let ist_secs = expiry_secs + ist_offset;
        let seconds_today = ist_secs % 86_400;
        assert_eq!(seconds_today, 3 * 3600 + 30 * 60);
    }

    #[test]
    fn begin_mints_state_and_authorize_url_without_persisting_auth_code() {
        let (state, login) = begin_connect("prod", "conn-f1", "APP-100", "sec").unwrap();
        assert_eq!(state.len(), 32);
        assert!(login.contains("client_id=APP-100"));
        assert!(take_pending_connect(&state).is_some());
        assert!(take_pending_connect(&state).is_none(), "state is single-use");
    }

    #[test]
    fn exchange_posts_json_validate_authcode_and_builds_blob() {
        let body = r#"{"s":"ok","code":200,"access_token":"atok","refresh_token":"rtok"}"#;
        let http = MockHttp::ok_json(body);
        let minted = exchange_auth_code(
            &http,
            "APP-100",
            "sec",
            "authcode",
            1_700_000_000_000,
        )
        .unwrap();
        let json = http.last_json.lock().unwrap().clone().unwrap();
        assert!(json.contains("appIdHash"));
        assert!(json.contains("authorization_code"));
        assert_eq!(minted.access_token, "atok");
        assert_eq!(minted.refresh_token.as_deref(), Some("rtok"));
        let blob = minted.into_credential_blob();
        match blob {
            CredentialBlob::FyersOAuthJsonAppIdHashSession {
                access_token,
                refresh_token,
                app_id,
                ..
            } => {
                assert_eq!(app_id, "APP-100");
                assert_eq!(access_token, "atok");
                assert_eq!(refresh_token.as_deref(), Some("rtok"));
            }
            _ => panic!("expected fyers blob"),
        }
    }
}
