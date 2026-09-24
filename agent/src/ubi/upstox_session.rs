//! Upstox OAuth authorization_code session mint (host-side form POST exchange).
//!
//! ADR 0006 · B6 `upstox` rows 10/14–16. `code` is single-use and never persisted.

use crate::ubi::credentials::CredentialBlob;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const UPSTOX_API_HOST: &str = "api.upstox.com";
pub const UPSTOX_TOKEN_PATH: &str = "/v2/login/authorization/token";
pub const UPSTOX_AUTHORIZE_DIALOG: &str =
    "https://api.upstox.com/v2/login/authorization/dialog";

const CONNECT_STATE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone)]
pub struct PendingUpstoxConnect {
    pub environment: String,
    pub connection_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub expires_at_unix_ms: i64,
}

static PENDING: OnceLock<Mutex<HashMap<String, PendingUpstoxConnect>>> = OnceLock::new();

fn pending_store() -> &'static Mutex<HashMap<String, PendingUpstoxConnect>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn upstox_bearer_authorization_header_value(access_token: &str) -> String {
    format!("Bearer {}", access_token.trim())
}

pub fn upstox_callback_base_url() -> String {
    crate::oauth_loopback::https_oauth_callback_url("/api/daemon/broker/upstox/callback")
}

pub fn upstox_authorize_url(client_id: &str, state: &str) -> String {
    let redirect = upstox_callback_base_url();
    format!(
        "{UPSTOX_AUTHORIZE_DIALOG}?response_type=code&client_id={}&redirect_uri={}&state={}",
        url_encode_component(client_id),
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

/// Mint OAuth `state` + authorize dialog URL (ADR 0006).
pub fn begin_connect(
    environment: &str,
    connection_id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(String, String), String> {
    if environment.trim().is_empty() || connection_id.trim().is_empty() {
        return Err("environment and connection_id are required".into());
    }
    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        return Err("client_id and client_secret are required".into());
    }
    purge_expired_pending();
    let state = generate_state_nonce();
    let redirect_uri = upstox_callback_base_url();
    let login_url = upstox_authorize_url(client_id.trim(), &state);
    let pending = PendingUpstoxConnect {
        environment: environment.trim().to_string(),
        connection_id: connection_id.trim().to_string(),
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
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

pub fn take_pending_connect(state: &str) -> Option<PendingUpstoxConnect> {
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
pub struct MintedUpstoxSession {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: String,
    pub access_token_expiry_unix_ms: i64,
    pub user_id: String,
    pub redirect_uri: String,
}

impl MintedUpstoxSession {
    pub fn into_credential_blob(self) -> CredentialBlob {
        CredentialBlob::UpstoxOAuthBearerSession {
            client_id: self.client_id,
            client_secret: self.client_secret,
            access_token: self.access_token,
            access_token_expiry_unix_ms: self.access_token_expiry_unix_ms,
            user_id: self.user_id,
            redirect_uri: self.redirect_uri,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstoxExchangeErrorClass {
    InvalidRequest,
    SessionExpired,
    Upstream,
}

impl UpstoxExchangeErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_credentials",
            Self::SessionExpired => "session_expired",
            Self::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpstoxExchangeError {
    pub class: UpstoxExchangeErrorClass,
    pub message: String,
}

pub trait UpstoxSessionHttp: Send + Sync {
    fn post_form(
        &self,
        url: &str,
        headers: &[(String, String)],
        fields: &[(String, String)],
    ) -> Result<(u16, String), String>;
}

#[derive(Debug, Deserialize)]
struct UpstoxEnvelope {
    #[serde(default)]
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Option<UpstoxTokenData>,
    #[serde(default, rename = "access_token")]
    access_token: Option<String>,
    #[serde(default, rename = "user_id")]
    user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpstoxTokenData {
    #[serde(rename = "access_token")]
    access_token: String,
    #[serde(rename = "user_id")]
    user_id: String,
}

pub fn exchange_auth_code(
    http: &dyn UpstoxSessionHttp,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
    now_unix_ms: i64,
) -> Result<MintedUpstoxSession, UpstoxExchangeError> {
    if client_id.trim().is_empty()
        || client_secret.trim().is_empty()
        || redirect_uri.trim().is_empty()
        || code.trim().is_empty()
    {
        return Err(UpstoxExchangeError {
            class: UpstoxExchangeErrorClass::InvalidRequest,
            message: "client_id, client_secret, redirect_uri, and code are required".into(),
        });
    }
    let url = format!("https://{UPSTOX_API_HOST}{UPSTOX_TOKEN_PATH}");
    let headers = vec![(
        "Content-Type".into(),
        "application/x-www-form-urlencoded".into(),
    )];
    let fields = vec![
        ("code".into(), code.to_string()),
        ("client_id".into(), client_id.to_string()),
        ("client_secret".into(), client_secret.to_string()),
        ("redirect_uri".into(), redirect_uri.to_string()),
        ("grant_type".into(), "authorization_code".into()),
    ];
    let (status, body) = http
        .post_form(&url, &headers, &fields)
        .map_err(|e| UpstoxExchangeError {
            class: UpstoxExchangeErrorClass::Upstream,
            message: e,
        })?;
    if status == 401 || status == 403 || body_signals_invalid_token(&body) {
        return Err(UpstoxExchangeError {
            class: UpstoxExchangeErrorClass::SessionExpired,
            message: "upstox token exchange: invalid or expired authorization".into(),
        });
    }
    if status != 200 {
        return Err(UpstoxExchangeError {
            class: UpstoxExchangeErrorClass::Upstream,
            message: format!("upstox token exchange HTTP {status}"),
        });
    }
    let envelope: UpstoxEnvelope = serde_json::from_str(&body).map_err(|_| UpstoxExchangeError {
        class: UpstoxExchangeErrorClass::Upstream,
        message: "upstox token exchange: invalid JSON".into(),
    })?;
    if !envelope.status.is_empty() && envelope.status != "success" {
        let class = if body_signals_invalid_token(&envelope.message) || body_signals_invalid_token(&body) {
            UpstoxExchangeErrorClass::SessionExpired
        } else {
            UpstoxExchangeErrorClass::InvalidRequest
        };
        return Err(UpstoxExchangeError {
            class,
            message: if envelope.message.is_empty() {
                "upstox token exchange failed".into()
            } else {
                envelope.message
            },
        });
    }
    let (access_token, user_id) = if let Some(data) = envelope.data {
        (data.access_token, data.user_id)
    } else if let (Some(at), Some(uid)) = (envelope.access_token, envelope.user_id) {
        (at, uid)
    } else {
        return Err(UpstoxExchangeError {
            class: UpstoxExchangeErrorClass::Upstream,
            message: "upstox token exchange: missing access_token".into(),
        });
    };
    let expiry = access_token_expiry_unix_ms(now_unix_ms);
    Ok(MintedUpstoxSession {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        access_token,
        access_token_expiry_unix_ms: expiry,
        user_id,
        redirect_uri: redirect_uri.to_string(),
    })
}

fn body_signals_invalid_token(body: &str) -> bool {
    let head = &body[..body.len().min(2048)];
    head.contains("UDAPI100050")
        || head.contains("Invalid token")
        || head.contains("invalid token")
}

/// Next 3:30 AM IST boundary after `now_unix_ms` (Upstox trading-day token TTL; B6 row 16).
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

pub struct ReqwestUpstoxSessionHttp {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl ReqwestUpstoxSessionHttp {
    pub fn new() -> Result<Self, String> {
        let client = crate::egress::shared_client();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("upstox http runtime: {e}"))?;
        Ok(Self { client, runtime })
    }
}

impl Default for ReqwestUpstoxSessionHttp {
    fn default() -> Self {
        Self::new().expect("upstox session http")
    }
}

impl UpstoxSessionHttp for ReqwestUpstoxSessionHttp {
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
                .map_err(|e| format!("upstox http: {e}"))?;
            let status = response.status().as_u16();
            let text = response
                .text()
                .await
                .map_err(|e| format!("upstox http body: {e}"))?;
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

    impl UpstoxSessionHttp for MockHttp {
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
    fn authorize_url_includes_dialog_params_and_encoded_redirect() {
        let url = upstox_authorize_url("my_client", "abcstate");
        assert!(url.starts_with(UPSTOX_AUTHORIZE_DIALOG));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my_client"));
        assert!(url.contains("state=abcstate"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("%2Fapi%2Fdaemon%2Fbroker%2Fupstox%2Fcallback"));
    }

    #[test]
    fn callback_base_url_uses_agent_port_default() {
        std::env::remove_var("AGENT_PORT");
        assert!(upstox_callback_base_url().starts_with("https://127.0.0.1:9140/"));
    }

    #[test]
    fn bearer_header_uses_bearer_scheme() {
        assert_eq!(
            upstox_bearer_authorization_header_value("tok123"),
            "Bearer tok123"
        );
    }

    #[test]
    fn access_token_expiry_targets_next_330am_ist() {
        // 2026-09-23 10:00:00 UTC = 15:30 IST same day → expiry next calendar day 03:30 IST
        let ten_utc_ms = 1_759_632_000_000i64;
        let expiry = access_token_expiry_unix_ms(ten_utc_ms);
        let expiry_secs = expiry / 1000;
        let ist_offset = 5 * 3600 + 30 * 60;
        let ist_secs = expiry_secs + ist_offset;
        let seconds_today = ist_secs % 86_400;
        assert_eq!(seconds_today, 3 * 3600 + 30 * 60);
    }

    #[test]
    fn begin_mints_state_and_authorize_url_without_persisting_code() {
        let (state, login) = begin_connect("prod", "conn-u1", "cid", "csec").unwrap();
        assert_eq!(state.len(), 32);
        assert!(login.contains("client_id=cid"));
        assert!(take_pending_connect(&state).is_some());
        assert!(take_pending_connect(&state).is_none(), "state is single-use");
    }

    #[test]
    fn exchange_posts_oauth_form_and_builds_blob() {
        let body = r#"{"status":"success","data":{"user_id":"U123","access_token":"atok"}}"#;
        let http = MockHttp::ok_json(body);
        let minted = exchange_auth_code(
            &http,
            "cid",
            "csec",
            "https://127.0.0.1:9140/api/daemon/broker/upstox/callback",
            "authcode",
            1_700_000_000_000,
        )
        .unwrap();
        let form = http.last_form.lock().unwrap().clone().unwrap();
        assert!(
            form.iter()
                .any(|(k, v)| k == "grant_type" && v == "authorization_code")
        );
        assert!(form.iter().any(|(k, _)| k == "code"));
        assert_eq!(minted.user_id, "U123");
        assert_eq!(minted.access_token, "atok");
        let blob = minted.into_credential_blob();
        match blob {
            CredentialBlob::UpstoxOAuthBearerSession {
                access_token,
                user_id,
                ..
            } => {
                assert_eq!(access_token, "atok");
                assert_eq!(user_id, "U123");
            }
            _ => panic!("expected upstox blob"),
        }
    }
}
