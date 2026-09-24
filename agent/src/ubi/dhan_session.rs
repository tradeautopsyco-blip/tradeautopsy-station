//! Dhan consent-login session (host-side generate/consume exchange).
//!
//! ADR 0009 · B6 `dhan` rows 10/14–16. The consent `generate` (POST) runs at
//! `connect/begin` time so the browser can be pointed at the consent-login URL;
//! the `tokenId` landing on the loopback callback is consumed (GET, D1) at once.
//!
//! `consentAppId` and `tokenId` are single-use and never persisted — memory-only
//! pending across the browser hop, dropped on consume. `app_secret` is
//! vault-only: it is presented at generate/consume and never logged.

use crate::ubi::credentials::CredentialBlob;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DHAN_AUTH_HOST: &str = "auth.dhan.co";
pub const DHAN_API_HOST: &str = "api.dhan.co";
pub const DHAN_BOOK_ID: &str = "dhan-nse-bse-cash";
pub const DHAN_GENERATE_CONSENT_PATH: &str = "/app/generate-consent";
pub const DHAN_CONSUME_CONSENT_PATH: &str = "/app/consumeApp-consent";
pub const DHAN_CONSENT_LOGIN_PATH: &str = "/login/consentApp-login";

const CONNECT_STATE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone)]
pub struct PendingDhanConnect {
    pub environment: String,
    pub connection_id: String,
    pub dhan_client_id: String,
    pub app_id: String,
    pub app_secret: String,
    pub consent_app_id: String,
    pub expires_at_unix_ms: i64,
}

static PENDING: OnceLock<Mutex<HashMap<String, PendingDhanConnect>>> = OnceLock::new();

fn pending_store() -> &'static Mutex<HashMap<String, PendingDhanConnect>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
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

pub fn dhan_callback_base_url() -> String {
    crate::oauth_loopback::https_oauth_callback_url("/api/daemon/broker/dhan/callback")
}

pub fn dhan_consent_login_url(consent_app_id: &str) -> String {
    format!(
        "https://{DHAN_AUTH_HOST}{DHAN_CONSENT_LOGIN_PATH}?consentAppId={}",
        url_encode_component(consent_app_id.trim())
    )
}

/// Log-safe `state` rendering: 8-char prefix only. `tokenId`, `consentAppId`,
/// `accessToken`, and `app_secret` never reach logs — there is no renderer for
/// them on purpose.
pub fn truncate_state(state: &str) -> String {
    let prefix: String = state.chars().take(8).collect();
    format!("{prefix}…")
}

/// Mint `state`, run consent `generate`, and return the browser login URL.
///
/// Unlike the OAuth siblings, `begin` performs one venue call: the login URL
/// carries the `consentAppId` minted by `POST /app/generate-consent`, so there
/// is no URL to return before generate succeeds. Generate failure surfaces as
/// a Connect failure, never a retry loop, never Kill.
pub fn begin_connect(
    http: &dyn DhanSessionHttp,
    environment: &str,
    connection_id: &str,
    dhan_client_id: &str,
    app_id: &str,
    app_secret: &str,
) -> Result<(String, String), DhanSessionError> {
    if environment.trim().is_empty() || connection_id.trim().is_empty() {
        return Err(DhanSessionError::invalid_request(
            "environment and connection_id are required",
        ));
    }
    if dhan_client_id.trim().is_empty() || app_id.trim().is_empty() || app_secret.trim().is_empty()
    {
        return Err(DhanSessionError::invalid_request(
            "dhan_client_id, app_id, and app_secret are required",
        ));
    }
    let consent_app_id = generate_consent(
        http,
        dhan_client_id.trim(),
        app_id.trim(),
        app_secret.trim(),
    )?;
    purge_expired_pending();
    let state = generate_state_nonce();
    let login_url = dhan_consent_login_url(&consent_app_id);
    let pending = PendingDhanConnect {
        environment: environment.trim().to_string(),
        connection_id: connection_id.trim().to_string(),
        dhan_client_id: dhan_client_id.trim().to_string(),
        app_id: app_id.trim().to_string(),
        app_secret: app_secret.trim().to_string(),
        consent_app_id,
        expires_at_unix_ms: now_unix_ms() + CONNECT_STATE_TTL.as_millis() as i64,
    };
    pending_store()
        .lock()
        .map_err(|_| {
            DhanSessionError::new(
                DhanSessionErrorClass::Upstream,
                "connect pending lock poisoned",
            )
        })?
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
pub fn take_pending_connect(state: &str) -> Option<PendingDhanConnect> {
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
pub struct MintedDhanSession {
    pub dhan_client_id: String,
    pub app_id: String,
    pub app_secret: String,
    pub access_token: String,
    pub expiry_time: String,
    pub dhan_client_ucc: String,
}

impl MintedDhanSession {
    pub fn into_credential_blob(self) -> CredentialBlob {
        CredentialBlob::DhanConsentSession {
            dhan_client_id: self.dhan_client_id,
            app_id: self.app_id,
            app_secret: self.app_secret,
            access_token: self.access_token,
            expiry_time: self.expiry_time,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhanSessionErrorClass {
    InvalidRequest,
    SessionExpired,
    RateLimited,
    Upstream,
}

impl DhanSessionErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_credentials",
            Self::SessionExpired => "session_expired",
            Self::RateLimited => "rate_limited",
            Self::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DhanSessionError {
    pub class: DhanSessionErrorClass,
    pub message: String,
}

impl DhanSessionError {
    fn new(class: DhanSessionErrorClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }

    fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(DhanSessionErrorClass::InvalidRequest, message)
    }
}

pub trait DhanSessionHttp: Send + Sync {
    /// POST with query params and an empty body (generate-consent per B6 row 10).
    fn post_empty(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<(u16, String), String>;
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<(u16, String), String>;
}

fn consent_headers(app_id: &str, app_secret: &str) -> Vec<(String, String)> {
    vec![
        ("app_id".into(), app_id.to_string()),
        ("app_secret".into(), app_secret.to_string()),
        ("Accept".into(), "application/json".into()),
    ]
}

#[derive(Debug, Deserialize)]
struct GenerateConsentResponse {
    #[serde(default, rename = "consentAppId")]
    consent_app_id: Option<String>,
    #[serde(default)]
    data: Option<GenerateConsentData>,
}

#[derive(Debug, Deserialize)]
struct GenerateConsentData {
    #[serde(default, rename = "consentAppId")]
    consent_app_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConsumeConsentResponse {
    #[serde(default, rename = "accessToken")]
    access_token: Option<String>,
    #[serde(default, rename = "dhanClientId")]
    dhan_client_id: Option<String>,
    #[serde(default, rename = "dhanClientUcc")]
    dhan_client_ucc: Option<String>,
    #[serde(default, rename = "expiryTime")]
    expiry_time: Option<String>,
    #[serde(default)]
    data: Option<ConsumeConsentData>,
}

#[derive(Debug, Deserialize)]
struct ConsumeConsentData {
    #[serde(default, rename = "accessToken")]
    access_token: Option<String>,
    #[serde(default, rename = "dhanClientId")]
    dhan_client_id: Option<String>,
    #[serde(default, rename = "dhanClientUcc")]
    dhan_client_ucc: Option<String>,
    #[serde(default, rename = "expiryTime")]
    expiry_time: Option<String>,
}

/// `POST /app/generate-consent?client_id=` → `consentAppId` (B6 row 10).
/// Response shape is accepted flat or `{data: …}`-nested; dogfood confirms.
fn generate_consent(
    http: &dyn DhanSessionHttp,
    dhan_client_id: &str,
    app_id: &str,
    app_secret: &str,
) -> Result<String, DhanSessionError> {
    let url = format!(
        "https://{DHAN_AUTH_HOST}{DHAN_GENERATE_CONSENT_PATH}?client_id={}",
        url_encode_component(dhan_client_id)
    );
    let headers = consent_headers(app_id, app_secret);
    let (status, body) = http.post_empty(&url, &headers).map_err(|e| {
        DhanSessionError::new(
            DhanSessionErrorClass::Upstream,
            sanitize_dhan_error_message(&e),
        )
    })?;
    if let Some(class) = classify_exchange_failure(status, &body) {
        return Err(DhanSessionError::new(
            class,
            exchange_failure_message("generate-consent", status, &body),
        ));
    }
    let parsed: GenerateConsentResponse =
        serde_json::from_str(&body).map_err(|_| {
            DhanSessionError::new(
                DhanSessionErrorClass::Upstream,
                "dhan generate-consent: invalid JSON",
            )
        })?;
    parsed
        .consent_app_id
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            parsed
                .data
                .and_then(|d| d.consent_app_id)
                .filter(|s| !s.trim().is_empty())
        })
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            DhanSessionError::new(
                DhanSessionErrorClass::Upstream,
                "dhan generate-consent: missing consentAppId",
            )
        })
}

/// `GET /app/consumeApp-consent?tokenId=` → session (B6 row 10, D1 = GET).
/// `dhan_client_id` is the begin-time id; the response value wins when present.
pub fn exchange_token_id(
    http: &dyn DhanSessionHttp,
    dhan_client_id: &str,
    app_id: &str,
    app_secret: &str,
    token_id: &str,
) -> Result<MintedDhanSession, DhanSessionError> {
    if dhan_client_id.trim().is_empty()
        || app_id.trim().is_empty()
        || app_secret.trim().is_empty()
        || token_id.trim().is_empty()
    {
        return Err(DhanSessionError::invalid_request(
            "dhan_client_id, app_id, app_secret, and token_id are required",
        ));
    }
    let url = format!(
        "https://{DHAN_AUTH_HOST}{DHAN_CONSUME_CONSENT_PATH}?tokenId={}",
        url_encode_component(token_id.trim())
    );
    let headers = consent_headers(app_id.trim(), app_secret.trim());
    let (status, body) = http.get(&url, &headers).map_err(|e| {
        DhanSessionError::new(
            DhanSessionErrorClass::Upstream,
            sanitize_dhan_error_message(&e),
        )
    })?;
    if let Some(class) = classify_exchange_failure(status, &body) {
        return Err(DhanSessionError::new(
            class,
            exchange_failure_message("consumeApp-consent", status, &body),
        ));
    }
    let parsed: ConsumeConsentResponse = serde_json::from_str(&body).map_err(|_| {
        DhanSessionError::new(
            DhanSessionErrorClass::Upstream,
            "dhan consume: invalid JSON",
        )
    })?;
    let access_token = parsed
        .access_token
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            parsed
                .data
                .as_ref()
                .and_then(|d| d.access_token.clone())
                .filter(|s| !s.trim().is_empty())
        })
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            DhanSessionError::new(
                DhanSessionErrorClass::Upstream,
                "dhan consume: missing accessToken",
            )
        })?;
    let expiry_time = parsed
        .expiry_time
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            parsed
                .data
                .as_ref()
                .and_then(|d| d.expiry_time.clone())
                .filter(|s| !s.trim().is_empty())
        })
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            DhanSessionError::new(
                DhanSessionErrorClass::Upstream,
                "dhan consume: missing expiryTime",
            )
        })?;
    let response_client_id = parsed
        .dhan_client_id
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            parsed
                .data
                .as_ref()
                .and_then(|d| d.dhan_client_id.clone())
                .filter(|s| !s.trim().is_empty())
        })
        .map(|s| s.trim().to_string());
    let dhan_client_ucc = parsed
        .dhan_client_ucc
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            parsed
                .data
                .as_ref()
                .and_then(|d| d.dhan_client_ucc.clone())
                .filter(|s| !s.trim().is_empty())
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    Ok(MintedDhanSession {
        dhan_client_id: response_client_id.unwrap_or_else(|| dhan_client_id.trim().to_string()),
        app_id: app_id.trim().to_string(),
        app_secret: app_secret.trim().to_string(),
        access_token,
        expiry_time,
        dhan_client_ucc,
    })
}

/// B6 row 4/9 error map: dead credential → `session_expired` (reconnect, never
/// Kill); venue backpressure → `rate_limited` (back off, never storm).
/// Returns `None` when the exchange may proceed to response parsing.
fn classify_exchange_failure(status: u16, body: &str) -> Option<DhanSessionErrorClass> {
    if dhan_body_signals_expired_session(body) {
        return Some(DhanSessionErrorClass::SessionExpired);
    }
    if status == 429 || dhan_body_signals_rate_limited(body) {
        return Some(DhanSessionErrorClass::RateLimited);
    }
    if status == 401 || status == 403 {
        return Some(DhanSessionErrorClass::SessionExpired);
    }
    if (200..=299).contains(&status) {
        return None;
    }
    Some(DhanSessionErrorClass::Upstream)
}

fn exchange_failure_message(op: &str, status: u16, body: &str) -> String {
    match extract_dhan_error_message(body) {
        Some(detail) if !detail.is_empty() => {
            format!("dhan {op} failed (http {status}): {detail}")
        }
        _ => format!("dhan {op} failed (http {status})"),
    }
}

/// `DH-901` / `807` / `808` / `809` (B6 rows 4/9) — token invalid or expired.
fn dhan_body_signals_expired_session(body: &str) -> bool {
    body_has_error_code(body, &["DH-901", "807", "808", "809"])
}

/// `805` / `DH-904` (B6 row 4) — too many requests; `805` warns the user "may
/// result in the user being blocked", so this class must back off, never retry.
fn dhan_body_signals_rate_limited(body: &str) -> bool {
    body_has_error_code(body, &["805", "DH-904"])
}

/// Match an official `{errorType, errorCode, errorMessage}` code without
/// tripping on the same digits inside timestamps or ids elsewhere in the body.
fn body_has_error_code(body: &str, codes: &[&str]) -> bool {
    let head = &body[..body.len().min(2048)];
    codes.iter().any(|code| {
        head.contains(&format!("\"errorCode\":{code}"))
            || head.contains(&format!("\"errorCode\": {code}"))
            || head.contains(&format!("\"errorCode\":\"{code}\""))
            || head.contains(&format!("\"errorCode\": \"{code}\""))
            // Trading-API `DH-*` codes are distinctive; Data-API numerics are
            // only matched in the `errorCode` position above.
            || (code.starts_with("DH-") && head.contains(code))
    })
}

/// Pull a human-readable venue message without echoing tokens / secrets.
fn extract_dhan_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    for key in ["errorMessage", "message", "error"] {
        if let Some(msg) = value.get(key).and_then(|v| v.as_str()) {
            let cleaned = sanitize_dhan_error_message(msg);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

/// Drop long token-like substrings; keep short operational messages.
/// `tokenId` / `consentAppId` / `accessToken` / `app_secret` must never survive
/// this function into a log line or an error surfaced past the agent.
fn sanitize_dhan_error_message(raw: &str) -> String {
    let mut out = String::new();
    for word in raw.split_whitespace() {
        let looks_secret = word.len() >= 24
            || (word.len() >= 8
                && word.chars().filter(|c| c.is_ascii_hexdigit()).count() > word.len() / 2);
        if looks_secret {
            out.push_str("[redacted]");
        } else {
            out.push_str(word);
        }
        out.push(' ');
    }
    out.trim().to_string()
}

pub struct ReqwestDhanSessionHttp {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

/// Read-only Dhan v2 paths for book `dhan-nse-bse-cash` (B6 row 2 / lock).
pub fn dhan_path_allowed(method: &str, path_norm: &str) -> bool {
    if method.to_ascii_uppercase() != "GET" {
        return false;
    }
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() {
        return false;
    }
    p == "/v2/trades"
        || p.starts_with("/v2/trades/")
        || p == "/v2/orders"
        || p.starts_with("/v2/orders/")
        || p == "/v2/holdings"
        || p == "/v2/positions"
        || p == "/v2/fundlimit"
        || p == "/v2/profile"
        || p.starts_with("/v2/ledger")
        || p.starts_with("/v2/statement")
}

impl ReqwestDhanSessionHttp {
    pub fn new() -> Result<Self, String> {
        let client = crate::egress::shared_client();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("dhan http runtime: {e}"))?;
        Ok(Self { client, runtime })
    }
}

impl Default for ReqwestDhanSessionHttp {
    fn default() -> Self {
        Self::new().expect("dhan session http")
    }
}

impl DhanSessionHttp for ReqwestDhanSessionHttp {
    fn post_empty(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<(u16, String), String> {
        // No egress metering: dhan has no venue slot while Planned (kite/upstox/
        // fyers precedent) and generate/consume run once per Connect — no storm
        // surface. Rate discipline (`805` → back off) is classification, above.
        self.runtime.block_on(async {
            let mut builder = self.client.post(url);
            for (name, value) in headers {
                builder = builder.header(name.as_str(), value.as_str());
            }
            let response = builder.send().await.map_err(|e| redact_http_err(&e))?;
            let status = response.status().as_u16();
            let text = response.text().await.map_err(|e| redact_http_err(&e))?;
            Ok((status, text))
        })
    }

    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<(u16, String), String> {
        self.runtime.block_on(async {
            let mut builder = self.client.get(url);
            for (name, value) in headers {
                builder = builder.header(name.as_str(), value.as_str());
            }
            let response = builder.send().await.map_err(|e| redact_http_err(&e))?;
            let status = response.status().as_u16();
            let text = response.text().await.map_err(|e| redact_http_err(&e))?;
            Ok((status, text))
        })
    }
}

fn redact_http_err(err: &reqwest::Error) -> String {
    // Never echo URL: generate/consume URLs carry `client_id` / `tokenId`.
    if err.is_timeout() {
        "dhan http: timeout".into()
    } else if err.is_connect() {
        "dhan http: connect".into()
    } else if err.is_request() {
        "dhan http: request".into()
    } else if err.is_body() || err.is_decode() {
        "dhan http: body".into()
    } else {
        "dhan http: error".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct ScriptedHttp {
        calls: Mutex<Vec<(String, String, Vec<(String, String)>)>>,
        responses: Mutex<Vec<(u16, String)>>,
    }

    impl ScriptedHttp {
        fn new(responses: Vec<(u16, String)>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(responses),
            }
        }
    }

    impl DhanSessionHttp for ScriptedHttp {
        fn post_empty(
            &self,
            url: &str,
            headers: &[(String, String)],
        ) -> Result<(u16, String), String> {
            self.calls.lock().unwrap().push((
                "POST".to_string(),
                url.to_string(),
                headers.to_vec(),
            ));
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err("no scripted response".into());
            }
            Ok(responses.remove(0))
        }

        fn get(&self, url: &str, headers: &[(String, String)]) -> Result<(u16, String), String> {
            self.calls
                .lock()
                .unwrap()
                .push(("GET".to_string(), url.to_string(), headers.to_vec()));
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err("no scripted response".into());
            }
            Ok(responses.remove(0))
        }
    }

    #[test]
    fn begin_posts_generate_and_returns_consent_login_url() {
        let http = ScriptedHttp::new(vec![(200, r#"{"consentAppId":"cap-1"}"#.into())]);
        let (state, login) =
            begin_connect(&http, "prod", "conn-d1", "1001234567", "app-id-1", "shh-secret")
                .expect("begin");
        assert_eq!(state.len(), 32);
        assert!(login.starts_with(
            "https://auth.dhan.co/login/consentApp-login?consentAppId=cap-1"
        ));
        let calls = http.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "POST");
        assert!(calls[0].1.starts_with(
            "https://auth.dhan.co/app/generate-consent?client_id=1001234567"
        ));
        assert!(calls[0]
            .2
            .iter()
            .any(|(n, v)| n == "app_id" && v == "app-id-1"));
        assert!(calls[0]
            .2
            .iter()
            .any(|(n, v)| n == "app_secret" && v == "shh-secret"));
        // Pending is single-use and bound to the connection.
        let pending = take_pending_connect(&state).expect("pending");
        assert_eq!(pending.connection_id, "conn-d1");
        assert_eq!(pending.consent_app_id, "cap-1");
        assert!(take_pending_connect(&state).is_none(), "state is single-use");
    }

    #[test]
    fn begin_rejects_empty_inputs_without_dialing() {
        let http = ScriptedHttp::new(vec![]);
        let err = begin_connect(&http, "prod", "conn-d1", "", "app", "sec")
            .expect_err("empty client id");
        assert_eq!(err.class, DhanSessionErrorClass::InvalidRequest);
        assert!(http.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn begin_generate_failure_is_connect_failure_not_retry_loop() {
        let http = ScriptedHttp::new(vec![(
            500,
            r#"{"errorType":"ServerError","errorCode":"DH-999","errorMessage":"boom"}"#.into(),
        )]);
        let err = begin_connect(&http, "prod", "conn-d1", "1001234567", "app", "sec")
            .expect_err("generate fails");
        assert_eq!(err.class, DhanSessionErrorClass::Upstream);
        assert!(err.message.contains("generate-consent"));
    }

    #[test]
    fn exchange_gets_consume_and_builds_blob_without_handshake_values() {
        let http = ScriptedHttp::new(vec![(200, r#"{
            "accessToken": "jwt.at",
            "dhanClientId": "1001234567",
            "dhanClientName": "Test User",
            "dhanClientUcc": "UCC1",
            "expiryTime": "2026-09-25T09:00:00+05:30"
        }"#.into())]);
        let minted = exchange_token_id(&http, "1001234567", "app-id-1", "shh-secret", "tok-1")
            .expect("exchange");
        let calls = http.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "GET");
        assert!(calls[0].1.starts_with(
            "https://auth.dhan.co/app/consumeApp-consent?tokenId=tok-1"
        ));
        assert!(calls[0]
            .2
            .iter()
            .any(|(n, v)| n == "app_id" && v == "app-id-1"));
        assert!(calls[0]
            .2
            .iter()
            .any(|(n, v)| n == "app_secret" && v == "shh-secret"));
        assert_eq!(minted.access_token, "jwt.at");
        assert_eq!(minted.expiry_time, "2026-09-25T09:00:00+05:30");
        assert_eq!(minted.dhan_client_ucc, "UCC1");
        let blob = minted.into_credential_blob();
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("dhan_consent_session"));
        assert!(!json.contains("tok-1"), "tokenId must never persist");
        assert!(!json.contains("consentAppId"));
        match blob {
            CredentialBlob::DhanConsentSession {
                dhan_client_id,
                access_token,
                expiry_time,
                ..
            } => {
                assert_eq!(dhan_client_id, "1001234567");
                assert_eq!(access_token, "jwt.at");
                assert_eq!(expiry_time, "2026-09-25T09:00:00+05:30");
            }
            _ => panic!("expected dhan blob"),
        }
    }

    #[test]
    fn exchange_accepts_data_nested_consume_shape() {
        let http = ScriptedHttp::new(vec![(200, r#"{
            "status": "success",
            "data": {
                "accessToken": "jwt.nested",
                "dhanClientId": "1001234567",
                "expiryTime": "2026-09-25T09:00:00+05:30"
            }
        }"#.into())]);
        let minted = exchange_token_id(&http, "fallback-id", "app", "sec", "tok-1")
            .expect("nested exchange");
        assert_eq!(minted.access_token, "jwt.nested");
        assert_eq!(minted.dhan_client_id, "1001234567");
    }

    #[test]
    fn exchange_falls_back_to_begin_time_client_id() {
        let http = ScriptedHttp::new(vec![(200, r#"{
            "accessToken": "jwt.at",
            "expiryTime": "2026-09-25T09:00:00+05:30"
        }"#.into())]);
        let minted = exchange_token_id(&http, "begin-id", "app", "sec", "tok-1")
            .expect("fallback exchange");
        assert_eq!(minted.dhan_client_id, "begin-id");
        assert!(minted.dhan_client_ucc.is_empty());
    }

    #[test]
    fn dead_credential_codes_map_to_session_expired() {
        for body in [
            r#"{"errorType":"AuthError","errorCode":"DH-901","errorMessage":"Invalid token"}"#,
            r#"{"errorType":"AuthError","errorCode":807,"errorMessage":"expired"}"#,
            r#"{"errorType":"AuthError","errorCode":"808","errorMessage":"auth failed"}"#,
            r#"{"errorType":"AuthError","errorCode": 809,"errorMessage":"auth failed"}"#,
        ] {
            let http = ScriptedHttp::new(vec![(200, body.into())]);
            let err =
                exchange_token_id(&http, "id", "app", "sec", "tok-1").expect_err("expired");
            assert_eq!(
                err.class,
                DhanSessionErrorClass::SessionExpired,
                "body: {body}"
            );
            assert_eq!(err.class.as_str(), "session_expired");
        }
        for status in [401u16, 403] {
            let http = ScriptedHttp::new(vec![(status, "forbidden".into())]);
            let err =
                exchange_token_id(&http, "id", "app", "sec", "tok-1").expect_err("auth status");
            assert_eq!(err.class, DhanSessionErrorClass::SessionExpired);
        }
    }

    #[test]
    fn backpressure_maps_to_rate_limited_never_kill() {
        for body in [
            r#"{"errorType":"RateLimit","errorCode":805,"errorMessage":"too many requests"}"#,
            r#"{"errorType":"RateLimit","errorCode":"805","errorMessage":"slow down"}"#,
            r#"{"errorType":"RateLimit","errorCode":"DH-904","errorMessage":"rate limit"}"#,
        ] {
            let http = ScriptedHttp::new(vec![(200, body.into())]);
            let err =
                exchange_token_id(&http, "id", "app", "sec", "tok-1").expect_err("limited");
            assert_eq!(
                err.class,
                DhanSessionErrorClass::RateLimited,
                "body: {body}"
            );
            assert_eq!(err.class.as_str(), "rate_limited");
        }
        let http = ScriptedHttp::new(vec![(429, "slow down".into())]);
        let err = exchange_token_id(&http, "id", "app", "sec", "tok-1").expect_err("429");
        assert_eq!(err.class, DhanSessionErrorClass::RateLimited);
    }

    #[test]
    fn error_digits_outside_error_code_do_not_false_positive() {
        // 807/808/809-looking digits in timestamps must not read as expired.
        let http = ScriptedHttp::new(vec![(200, r#"{
            "accessToken": "jwt.at",
            "expiryTime": "2026-09-25T08:07:09+05:30"
        }"#.into())]);
        let minted = exchange_token_id(&http, "begin-id", "app", "sec", "tok-1")
            .expect("timestamp digits must not trip the expired matcher");
        assert_eq!(minted.expiry_time, "2026-09-25T08:07:09+05:30");
    }

    #[test]
    fn secrets_never_survive_into_error_messages() {
        let token_id = "tok-9f8e7d6c5b4a39482716150403020100";
        let app_secret = "supersecret-app-secret-value-1";
        let consent_leak = "consentAppId-cap-abcdef1234567890abcdef";
        let access_leak = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig";
        let body = format!(
            r#"{{"errorType":"ServerError","errorCode":"DH-999","errorMessage":"ref {token_id} {consent_leak} {access_leak}"}}"#
        );
        let http = ScriptedHttp::new(vec![(500, body)]);
        let err = exchange_token_id(&http, "id", "app", app_secret, token_id)
            .expect_err("upstream");
        assert_eq!(err.class, DhanSessionErrorClass::Upstream);
        assert!(
            !err.message.contains(token_id),
            "tokenId leaked: {}",
            err.message
        );
        assert!(
            !err.message.contains(consent_leak),
            "consentAppId leaked: {}",
            err.message
        );
        assert!(
            !err.message.contains(access_leak),
            "accessToken leaked: {}",
            err.message
        );
        assert!(
            !err.message.contains(app_secret),
            "app_secret leaked: {}",
            err.message
        );
        assert!(err.message.contains("[redacted]"));
    }

    #[test]
    fn truncate_state_renders_prefix_only() {
        assert_eq!(truncate_state("abcdef1234567890"), "abcdef12…");
        assert_eq!(truncate_state(""), "…");
    }

    #[test]
    fn consent_login_url_and_callback_base_share_auth_host() {
        let login = dhan_consent_login_url("cap 1");
        assert!(login.starts_with("https://auth.dhan.co/login/consentApp-login?consentAppId="));
        assert!(login.contains("cap%201"));
        std::env::remove_var("AGENT_PORT");
        assert_eq!(
            dhan_callback_base_url(),
            "https://127.0.0.1:9140/api/daemon/broker/dhan/callback"
        );
    }
}
