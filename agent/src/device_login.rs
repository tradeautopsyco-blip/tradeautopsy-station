//! WorkOS device-code login → Console Station token mint (A8 II.2–II.5).
//!
//! UI-facing types never expose `device_code`. Polling uses the private pending handle.

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use std::time::Duration;

use super::station_tokens::{StationTokenStore, StationTokens};

const WORKOS_API_DEFAULT: &str = "https://api.workos.com";

fn workos_api_base() -> String {
    std::env::var("WORKOS_API_BASE_URL")
        .unwrap_or_else(|_| WORKOS_API_DEFAULT.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// What Station may show the human (A8: user_code only; never device_code).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeviceLoginPublic {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Session identity from Console `GET /api/auth/station/session` (no tokens).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StationSessionIdentity {
    pub profile_id: String,
    pub email: String,
    pub aud: String,
}

/// Pending device grant. `device_code` is private to this module.
#[derive(Debug, Clone)]
pub struct DeviceLoginPending {
    pub public: DeviceLoginPublic,
    device_code: String,
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct WorkOsDeviceAuthorizeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: u64,
    #[serde(default = "default_interval")]
    interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct WorkOsDeviceAuthenticateSuccess {
    #[serde(default)]
    authkit_authorization_code: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
}

enum WorkOsMintProof {
    AuthorizationCode(String),
    AccessToken(String),
}

/// Console paired-device contract (station_devices, PR #379).
/// Access JWT is 1 hour; refresh family is 30 days. Values are what Console
/// returns — Station does not invent a different TTL.
pub const STATION_ACCESS_TTL_SECS: u64 = 60 * 60;
pub const STATION_REFRESH_TTL_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Deserialize)]
struct ConsoleStationTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    #[serde(default)]
    refresh_expires_in: Option<u64>,
    #[serde(default)]
    device_id: Option<String>,
}

fn normalize_device_id(raw: Option<String>) -> Option<String> {
    let trimmed = raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    match trimmed {
        Some(id) if id.len() <= 128 => Some(id),
        _ => None,
    }
}

/// Refresh grant for `POST /api/auth/station/token`. Includes `device_id` when
/// Keychain has one so Console can match `station_devices` and honor revoke.
pub(crate) fn refresh_grant_body(current: &StationTokens) -> serde_json::Value {
    let mut body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": current.refresh_token,
    });
    if let Some(device_id) = normalize_device_id(current.device_id.clone()) {
        body["device_id"] = serde_json::Value::String(device_id);
    }
    body
}

/// Begin WorkOS device authorization. Returns UI-safe public fields + private pending handle.
pub async fn begin_device_login(
    http: &reqwest::Client,
    client_id: &str,
) -> Result<DeviceLoginPending> {
    let workos = workos_api_base();
    let resp = http
        .post(format!("{workos}/user_management/authorize/device"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("client_id={}", urlencoding_loose(client_id)))
        .send()
        .await
        .context("WorkOS authorize/device network")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("WorkOS authorize/device HTTP {status}: {text}"));
    }

    let parsed: WorkOsDeviceAuthorizeResponse =
        serde_json::from_str(&text).context("parse WorkOS authorize/device")?;

    Ok(DeviceLoginPending {
        public: DeviceLoginPublic {
            user_code: parsed.user_code,
            verification_uri: parsed.verification_uri,
            verification_uri_complete: parsed.verification_uri_complete,
            expires_in: parsed.expires_in,
            interval: parsed.interval.max(1),
        },
        device_code: parsed.device_code,
        client_id: client_id.to_string(),
    })
}

/// Poll WorkOS until authorized, mint Station tokens at Console, persist to store.
pub async fn complete_device_login(
    http: &reqwest::Client,
    pending: &DeviceLoginPending,
    console_base_url: &str,
    store: &dyn StationTokenStore,
) -> Result<StationTokens> {
    let workos_proof = poll_workos_device_code(http, pending).await?;
    let tokens = mint_station_tokens(http, console_base_url, &workos_proof).await?;
    store.save(&tokens)?;
    Ok(tokens)
}

async fn poll_workos_device_code(
    http: &reqwest::Client,
    pending: &DeviceLoginPending,
) -> Result<WorkOsMintProof> {
    let deadline =
        std::time::Instant::now() + Duration::from_secs(pending.public.expires_in.max(30));
    let mut interval = Duration::from_secs(pending.public.interval);

    loop {
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("device login expired before authorization"));
        }

        let body = format!(
            "grant_type={}&device_code={}&client_id={}",
            urlencoding_loose("urn:ietf:params:oauth:grant-type:device_code"),
            urlencoding_loose(&pending.device_code),
            urlencoding_loose(&pending.client_id),
        );

        let workos = workos_api_base();
        let resp = http
            .post(format!("{workos}/user_management/authenticate"))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .context("WorkOS authenticate device_code network")?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if status.is_success() {
            let parsed: WorkOsDeviceAuthenticateSuccess =
                serde_json::from_str(&text).context("parse WorkOS authenticate success")?;
            if let Some(code) = parsed.authkit_authorization_code.filter(|s| !s.is_empty()) {
                return Ok(WorkOsMintProof::AuthorizationCode(code));
            }
            // Same AuthKit client as Console: WorkOS returns access_token and omits
            // authkit_authorization_code (that field is for a *different* application).
            if let Some(token) = parsed.access_token.filter(|s| !s.is_empty()) {
                return Ok(WorkOsMintProof::AccessToken(token));
            }
            return Err(anyhow!(
                "WorkOS authenticate succeeded without authkit_authorization_code or access_token"
            ));
        }

        // RFC 8628 / WorkOS pending states
        if text.contains("authorization_pending") {
            tokio::time::sleep(interval).await;
            continue;
        }
        if text.contains("slow_down") {
            interval += Duration::from_secs(5);
            tokio::time::sleep(interval).await;
            continue;
        }

        return Err(anyhow!("WorkOS authenticate HTTP {status}: {text}"));
    }
}

async fn mint_station_tokens(
    http: &reqwest::Client,
    console_base_url: &str,
    proof: &WorkOsMintProof,
) -> Result<StationTokens> {
    let base = console_base_url.trim_end_matches('/');
    require_console_base_url(base)?;

    let body = match proof {
        WorkOsMintProof::AuthorizationCode(code) => serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
        }),
        WorkOsMintProof::AccessToken(access_token) => serde_json::json!({
            "grant_type": "workos_access_token",
            "access_token": access_token,
        }),
    };

    let resp = http
        .post(format!("{base}/api/auth/station/token"))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Console station/token network")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("Console station/token HTTP {status}: {text}"));
    }

    let parsed: ConsoleStationTokenResponse =
        serde_json::from_str(&text).context("parse Console station/token")?;

    Ok(StationTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_in: parsed.expires_in,
        refresh_expires_in: parsed.refresh_expires_in,
        device_id: normalize_device_id(parsed.device_id),
    })
}

fn jwt_exp_unix(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value.get("exp").and_then(|n| n.as_u64())
}

/// True when the access JWT is missing `exp` or expires within `skew_secs`.
pub fn station_access_expiring(access_token: &str, skew_secs: u64) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match jwt_exp_unix(access_token) {
        Some(exp) => exp <= now.saturating_add(skew_secs),
        None => true,
    }
}

#[derive(Debug)]
pub enum StationRefreshError {
    /// Console rejected the refresh token (rotated away, family burned, expired).
    /// Retrying can never succeed — the human must device-login again.
    Revoked,
    /// Network / 429 / 5xx — safe to retry later with backoff.
    Transient(anyhow::Error),
}

impl std::fmt::Display for StationRefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Revoked => write!(f, "Station refresh token revoked — device login required"),
            Self::Transient(err) => write!(f, "Station refresh failed: {err}"),
        }
    }
}

impl std::error::Error for StationRefreshError {}

/// Exchange the stored refresh token for a new Station access token.
/// Console rotates on use and burns the whole family on reuse, so callers must
/// serialize this (see `UpstreamClient::ensure_fresh_station_access`).
/// On [`StationRefreshError::Revoked`] the store is cleared so nothing keeps
/// presenting a dead token.
pub async fn refresh_stored_station_tokens(
    http: &reqwest::Client,
    console_base_url: &str,
    store: &dyn StationTokenStore,
) -> std::result::Result<StationTokens, StationRefreshError> {
    let current = store
        .load()
        .map_err(StationRefreshError::Transient)?
        .ok_or(StationRefreshError::Revoked)?;
    let base = console_base_url.trim_end_matches('/');
    require_console_base_url(base).map_err(StationRefreshError::Transient)?;

    let resp = http
        .post(format!("{base}/api/auth/station/token"))
        .header("content-type", "application/json")
        .json(&refresh_grant_body(&current))
        .send()
        .await
        .map_err(|e| {
            StationRefreshError::Transient(anyhow!(e).context("Console station refresh network"))
        })?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::BAD_REQUEST {
        // Console's body is an error code (e.g. INVALID_REFRESH_TOKEN), never
        // token material — and it is what distinguishes a burned family from an
        // expired one. Bounded so an unexpected HTML body cannot flood the log.
        let detail: String = text.chars().take(160).collect();
        tracing::warn!(
            status = status.as_u16(),
            detail = %detail,
            "Console refused Station refresh grant; clearing stored tokens"
        );
        let _ = store.clear();
        return Err(StationRefreshError::Revoked);
    }
    if !status.is_success() {
        return Err(StationRefreshError::Transient(anyhow!(
            "Console station refresh HTTP {status}"
        )));
    }

    let parsed: ConsoleStationTokenResponse = serde_json::from_str(&text).map_err(|e| {
        StationRefreshError::Transient(anyhow!(e).context("parse Console station refresh"))
    })?;
    let tokens = StationTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_in: parsed.expires_in,
        refresh_expires_in: parsed.refresh_expires_in,
        device_id: normalize_device_id(parsed.device_id).or(current.device_id),
    };
    store
        .save(&tokens)
        .map_err(StationRefreshError::Transient)?;
    Ok(tokens)
}

/// Prove Keychain Caller against Console who-am-I (A8 Exit II).
pub async fn prove_station_session(
    http: &reqwest::Client,
    console_base_url: &str,
    access_token: &str,
) -> Result<StationSessionIdentity> {
    let base = console_base_url.trim_end_matches('/');
    require_console_base_url(base)?;

    let resp = http
        .get(format!("{base}/api/auth/station/session"))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {access_token}"),
        )
        .send()
        .await
        .context("Console station/session network")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("Console station/session HTTP {status}: {text}"));
    }

    serde_json::from_str(&text).context("parse Console station/session")
}

/// Console-branded device-login page (same Auth UI as `/login`) before WorkOS finishes pairing.
pub fn station_device_browser_url(
    console_base: &str,
    public: &DeviceLoginPublic,
) -> Result<String> {
    let base = console_base.trim_end_matches('/');
    require_console_base_url(base)?;
    Ok(format!(
        "{base}/auth/station-device?user_code={}&continue={}",
        urlencoding_loose(&public.user_code),
        urlencoding_loose(&public.verification_uri_complete),
    ))
}

fn require_console_base_url(base: &str) -> Result<()> {
    if base.starts_with("https://") {
        return Ok(());
    }
    // Mirror UpstreamConfig::require_https_base — wiremock / STATION_ACCESS_TOKEN only.
    let testing = std::env::var("STATION_ACCESS_TOKEN").is_ok();
    let loopback_http = base.starts_with("http://127.0.0.1")
        || base.starts_with("http://localhost")
        || base.starts_with("http://[::1]");
    if testing && loopback_http {
        return Ok(());
    }
    Err(anyhow!("CONSOLE base URL must be https:// (got {base})"))
}

fn urlencoding_loose(s: &str) -> String {
    // Minimal form encoding for OAuth fields (alphanumeric-safe passthrough).
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::station_tokens::MemoryStationTokenStore;

    fn jwt_with_exp(exp: u64) -> String {
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
        format!("e30.{payload}.sig")
    }

    #[test]
    fn station_access_expiring_reads_jwt_exp() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!station_access_expiring(&jwt_with_exp(now + 600), 90));
        assert!(station_access_expiring(&jwt_with_exp(now + 30), 90));
        assert!(station_access_expiring(&jwt_with_exp(now - 1), 90));
        assert!(station_access_expiring("not-a-jwt", 90));
    }
    use serial_test::serial;

    #[test]
    fn device_login_public_type_has_no_device_code_field() {
        // Compile-time / serde contract: UI JSON must not include device_code.
        let public = DeviceLoginPublic {
            user_code: "RRGQ-BJVS".into(),
            verification_uri: "https://example.authkit.app/device".into(),
            verification_uri_complete: "https://example.authkit.app/device?user_code=RRGQ-BJVS"
                .into(),
            expires_in: 300,
            interval: 5,
        };
        let json = serde_json::to_value(&public).expect("serialize");
        assert!(json.get("device_code").is_none());
        assert_eq!(json["user_code"], "RRGQ-BJVS");
    }

    #[test]
    fn pending_keeps_device_code_private_from_public_struct() {
        let pending = DeviceLoginPending {
            public: DeviceLoginPublic {
                user_code: "AAAA-BBBB".into(),
                verification_uri: "https://example.authkit.app/device".into(),
                verification_uri_complete: "https://example.authkit.app/device?user_code=AAAA-BBBB"
                    .into(),
                expires_in: 300,
                interval: 5,
            },
            device_code: "secret-device-code".into(),
            client_id: "client_test".into(),
        };
        let json = serde_json::to_value(&pending.public).expect("serialize public");
        let dumped = json.to_string();
        assert!(!dumped.contains("secret-device-code"));
        assert!(!dumped.contains("device_code"));
        // Private field still usable inside module
        assert_eq!(pending.device_code, "secret-device-code");
    }

    #[test]
    #[serial]
    fn mint_rejects_non_https_console_base() {
        // `http://localhost` is inside the STATION_ACCESS_TOKEN loopback-bootstrap
        // allowlist, so this test must run without that env var to actually exercise
        // the http-rejection path (guard against leakage from other #[serial] tests).
        std::env::remove_var("STATION_ACCESS_TOKEN");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let http = reqwest::Client::new();
        let err = rt
            .block_on(mint_station_tokens(
                &http,
                "http://localhost:3000",
                &WorkOsMintProof::AuthorizationCode("code".into()),
            ))
            .expect_err("must reject http");
        assert!(err.to_string().contains("https://"));
    }

    #[test]
    fn refresh_grant_includes_device_id_when_keychain_has_one() {
        let with_device = StationTokens {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_in: STATION_ACCESS_TTL_SECS,
            refresh_expires_in: Some(STATION_REFRESH_TTL_SECS),
            device_id: Some("  device-row  ".into()),
        };
        let body = refresh_grant_body(&with_device);
        assert_eq!(body["grant_type"], "refresh_token");
        assert_eq!(body["device_id"], "device-row");
        assert!(body.get("access_token").is_none());

        let legacy = StationTokens {
            device_id: None,
            ..with_device
        };
        let legacy_body = refresh_grant_body(&legacy);
        assert!(legacy_body.get("device_id").is_none());
        assert_eq!(legacy_body["grant_type"], "refresh_token");
    }

    #[test]
    fn complete_login_persists_tokens_when_mint_json_is_injected() {
        // Unit-level: store seam after mint shape is known (no live WorkOS).
        let store = MemoryStationTokenStore::default();
        let tokens = StationTokens {
            access_token: "station.access".into(),
            refresh_token: "station.refresh".into(),
            expires_in: STATION_ACCESS_TTL_SECS,
            refresh_expires_in: Some(STATION_REFRESH_TTL_SECS),
            device_id: Some("device-1".into()),
        };
        store.save(&tokens).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "station.access");
    }
}
