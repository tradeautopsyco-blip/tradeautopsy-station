//! Groww approval-checksum session mint (host-side).
//!
//! ADR 0014 · B6 `groww` rows 10/16. No browser, no callback — Connect posts
//! api key + secret once; the agent mints and writes the tagged Keychain blob.

use crate::ubi::credentials::CredentialBlob;
use crate::ubi::{GROWW_API_HOST, GROWW_API_VERSION_HEADER};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const GROWW_ACCESS_PATH: &str = "/v1/token/api/access";
const MINT_CAP_24H: usize = 150;
const MINT_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowwMintErrorClass {
    InvalidRequest,
    InvalidCredentials,
    RateLimited,
    Upstream,
}

impl GrowwMintErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_credentials",
            Self::InvalidCredentials => "invalid_credentials",
            Self::RateLimited => "rate_limited",
            Self::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrowwMintError {
    pub class: GrowwMintErrorClass,
    pub message: String,
}

impl GrowwMintError {
    fn new(class: GrowwMintErrorClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintedGrowwSession {
    pub api_key: String,
    pub api_secret: String,
    pub token: String,
    pub expiry: String,
    pub token_ref_id: String,
    pub minted_at: String,
}

impl MintedGrowwSession {
    pub fn into_credential_blob(self) -> CredentialBlob {
        CredentialBlob::GrowwChecksumSession {
            api_key: self.api_key,
            api_secret: self.api_secret,
            token: self.token,
            expiry: self.expiry,
            token_ref_id: self.token_ref_id,
            minted_at: self.minted_at,
        }
    }
}

pub trait GrowwSessionHttp: Send + Sync {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
    ) -> Result<(u16, String), String>;
}

/// SHA-256(`api_secret` + `timestamp`) as lowercase hex — timestamp is a STRING (D1).
pub fn groww_approval_checksum(api_secret: &str, timestamp: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{api_secret}{timestamp}").as_bytes());
    hex::encode(hasher.finalize())
}

fn now_unix_secs_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn access_url() -> String {
    format!("https://{GROWW_API_HOST}{GROWW_ACCESS_PATH}")
}

/// Mint a Groww session token (approval checksum). Enforces 150/24h + single-flight per connection.
pub fn connect_mint(
    http: &dyn GrowwSessionHttp,
    connection_id: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<MintedGrowwSession, GrowwMintError> {
    if connection_id.trim().is_empty() {
        return Err(GrowwMintError::new(
            GrowwMintErrorClass::InvalidRequest,
            "connection_id is required",
        ));
    }
    if api_key.trim().is_empty() || api_secret.trim().is_empty() {
        return Err(GrowwMintError::new(
            GrowwMintErrorClass::InvalidRequest,
            "api_key and api_secret are required",
        ));
    }
    let connection_id = connection_id.trim();
    let api_key = api_key.trim();
    let api_secret = api_secret.trim();

    mint_guard().begin(connection_id)?;

    let timestamp = now_unix_secs_string();
    let checksum = groww_approval_checksum(api_secret, &timestamp);
    let body = serde_json::json!({
        "key_type": "approval",
        "checksum": checksum,
        "timestamp": timestamp,
    });
    let headers = vec![
        (
            "Authorization".to_string(),
            format!("Bearer {}", api_key),
        ),
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Accept".to_string(), "application/json".to_string()),
        (
            "X-API-VERSION".to_string(),
            GROWW_API_VERSION_HEADER.to_string(),
        ),
    ];

    let result = mint_access_token(http, api_key, api_secret, &headers, &body);
    match result {
        Ok(session) => {
            mint_guard().record_success(connection_id);
            Ok(session)
        }
        Err(e) => {
            mint_guard().end(connection_id);
            Err(e)
        }
    }
}

fn mint_access_token(
    http: &dyn GrowwSessionHttp,
    api_key: &str,
    api_secret: &str,
    headers: &[(String, String)],
    body: &serde_json::Value,
) -> Result<MintedGrowwSession, GrowwMintError> {
    let (status, text) = http
        .post_json(&access_url(), headers, body)
        .map_err(|_| GrowwMintError::new(GrowwMintErrorClass::Upstream, "groww http error"))?;

    if status == 429 {
        return Err(GrowwMintError::new(
            GrowwMintErrorClass::RateLimited,
            "groww mint rate limited (150/24h cap) — wait before retrying",
        ));
    }
    if status == 401 || status == 403 {
        return Err(GrowwMintError::new(
            GrowwMintErrorClass::InvalidCredentials,
            "groww mint rejected credentials",
        ));
    }
    if !(200..300).contains(&status) {
        return Err(GrowwMintError::new(
            GrowwMintErrorClass::Upstream,
            format!("groww mint failed (http {status})"),
        ));
    }

    let parsed: GrowwAccessResponse = serde_json::from_str(&text).map_err(|_| {
        GrowwMintError::new(GrowwMintErrorClass::Upstream, "groww mint invalid JSON")
    })?;

    if parsed.is_session_expired(&text) {
        return Err(GrowwMintError::new(
            GrowwMintErrorClass::InvalidCredentials,
            "groww mint rejected (session expired)",
        ));
    }

    let token = parsed
        .token
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| {
            GrowwMintError::new(GrowwMintErrorClass::Upstream, "groww mint missing token")
        })?;
    let expiry = parsed
        .expiry
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| {
            GrowwMintError::new(GrowwMintErrorClass::Upstream, "groww mint missing expiry")
        })?;
    let token_ref_id = parsed.token_ref_id.unwrap_or_default();

    Ok(MintedGrowwSession {
        api_key: api_key.to_string(),
        api_secret: api_secret.to_string(),
        token,
        expiry,
        token_ref_id,
        minted_at: now_rfc3339(),
    })
}

#[derive(Debug, Deserialize)]
struct GrowwAccessResponse {
    token: Option<String>,
    expiry: Option<String>,
    #[serde(rename = "tokenRefId")]
    token_ref_id: Option<String>,
    #[serde(default)]
    error: Option<GrowwErrorBody>,
}

#[derive(Debug, Deserialize)]
struct GrowwErrorBody {
    code: Option<String>,
}

impl GrowwAccessResponse {
    fn is_session_expired(&self, body: &str) -> bool {
        if self
            .error
            .as_ref()
            .and_then(|e| e.code.as_deref())
            .is_some_and(|c| c.eq_ignore_ascii_case("GA005"))
        {
            return true;
        }
        body.contains("GA005")
    }
}

struct ConnectionMintGuard {
    in_flight: bool,
    mint_times: Vec<i64>,
}

struct MintGuard {
    connections: HashMap<String, ConnectionMintGuard>,
}

impl MintGuard {
    fn begin(&mut self, connection_id: &str) -> Result<(), GrowwMintError> {
        let now = now_unix_ms();
        let entry = self
            .connections
            .entry(connection_id.to_string())
            .or_insert_with(|| ConnectionMintGuard {
                in_flight: false,
                mint_times: Vec::new(),
            });
        entry.mint_times.retain(|t| now - *t < MINT_WINDOW.as_millis() as i64);
        if entry.mint_times.len() >= MINT_CAP_24H {
            return Err(GrowwMintError::new(
                GrowwMintErrorClass::RateLimited,
                "groww mint budget exhausted (150/24h) — try again tomorrow",
            ));
        }
        if entry.in_flight {
            return Err(GrowwMintError::new(
                GrowwMintErrorClass::Upstream,
                "groww mint already in progress for this connection",
            ));
        }
        entry.in_flight = true;
        Ok(())
    }

    fn record_success(&mut self, connection_id: &str) {
        let now = now_unix_ms();
        if let Some(entry) = self.connections.get_mut(connection_id) {
            entry.in_flight = false;
            entry.mint_times.push(now);
        }
    }

    fn end(&mut self, connection_id: &str) {
        if let Some(entry) = self.connections.get_mut(connection_id) {
            entry.in_flight = false;
        }
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn mint_guard() -> std::sync::MutexGuard<'static, MintGuard> {
    static GUARD: OnceLock<Mutex<MintGuard>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(MintGuard {
            connections: HashMap::new(),
        }))
        .lock()
        .expect("groww mint guard lock")
}

#[cfg(test)]
fn mint_guard_reset_for_tests() {
    let mut guard = mint_guard();
    guard.connections.clear();
}

pub struct ReqwestGrowwSessionHttp {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl ReqwestGrowwSessionHttp {
    pub fn new() -> Result<Self, String> {
        let client = crate::egress::shared_client();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("groww http runtime: {e}"))?;
        Ok(Self { client, runtime })
    }
}

impl Default for ReqwestGrowwSessionHttp {
    fn default() -> Self {
        Self::new().expect("groww session http")
    }
}

impl GrowwSessionHttp for ReqwestGrowwSessionHttp {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
    ) -> Result<(u16, String), String> {
        self.runtime.block_on(async {
            let mut builder = self.client.post(url).json(body);
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
    if err.is_timeout() {
        "groww http: timeout".into()
    } else if err.is_connect() {
        "groww http: connect".into()
    } else if err.is_request() {
        "groww http: request".into()
    } else if err.is_body() || err.is_decode() {
        "groww http: body".into()
    } else {
        "groww http: error".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct ScriptedHttp {
        calls: AtomicUsize,
        response: (u16, String),
    }

    impl GrowwSessionHttp for ScriptedHttp {
        fn post_json(
            &self,
            _url: &str,
            headers: &[(String, String)],
            body: &serde_json::Value,
        ) -> Result<(u16, String), String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert!(
                headers
                    .iter()
                    .any(|(n, v)| n == "Authorization" && v.starts_with("Bearer groww-key"))
            );
            assert!(
                headers
                    .iter()
                    .any(|(n, v)| n == "X-API-VERSION" && v == "1.0")
            );
            let ts = body["timestamp"].as_str().expect("string timestamp");
            assert!(ts.chars().all(|c| c.is_ascii_digit()));
            let checksum = body["checksum"].as_str().expect("checksum");
            assert_eq!(checksum, groww_approval_checksum("groww-secret", ts));
            Ok(self.response.clone())
        }
    }

    #[test]
    fn checksum_uses_string_timestamp_preimage() {
        let digest = groww_approval_checksum("secret", "1719830400");
        assert_eq!(digest.len(), 64);
        assert_ne!(digest, groww_approval_checksum("secret", "1719830401"));
    }

    #[test]
    fn mint_parses_success_payload() {
        mint_guard_reset_for_tests();
        let http = ScriptedHttp {
            calls: AtomicUsize::new(0),
            response: (
                200,
                r#"{"token":"tok-1","expiry":"2026-09-25T10:00:00Z","tokenRefId":"ref-1"}"#
                    .into(),
            ),
        };
        let minted = connect_mint(&http, "conn-1", "groww-key", "groww-secret").expect("mint");
        assert_eq!(minted.token, "tok-1");
        assert_eq!(minted.expiry, "2026-09-25T10:00:00Z");
        assert_eq!(minted.token_ref_id, "ref-1");
        assert_eq!(http.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mint_guard_blocks_second_begin_while_in_flight() {
        mint_guard_reset_for_tests();
        mint_guard().begin("conn-burst").expect("first begin");
        let err = mint_guard().begin("conn-burst").expect_err("in flight");
        assert!(err.message.contains("in progress"));
        mint_guard().end("conn-burst");
        mint_guard().begin("conn-burst").expect("after end");
    }
}
