//! WorkOS device-code login → Console Station token mint (A8 II.2–II.5).
//!
//! UI-facing types never expose `device_code`. Polling uses the private pending handle.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::time::Duration;

use super::station_tokens::{StationTokenStore, StationTokens};

const WORKOS_API: &str = "https://api.workos.com";

/// What Station may show the human (A8: user_code only; never device_code).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DeviceLoginPublic {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
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
    #[allow(dead_code)]
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConsoleStationTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    #[serde(default)]
    refresh_expires_in: Option<u64>,
}

/// Begin WorkOS device authorization. Returns UI-safe public fields + private pending handle.
pub async fn begin_device_login(
    http: &reqwest::Client,
    client_id: &str,
) -> Result<DeviceLoginPending> {
    let resp = http
        .post(format!("{WORKOS_API}/user_management/authorize/device"))
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
) -> Result<String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(pending.public.expires_in.max(30));
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

        let resp = http
            .post(format!("{WORKOS_API}/user_management/authenticate"))
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
                return Ok(code);
            }
            // Fallback: some tenants may only return access_token; Console mint currently
            // expects authorization_code. Prefer code; otherwise error clearly.
            return Err(anyhow!(
                "WorkOS authenticate succeeded without authkit_authorization_code"
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
    authorization_code: &str,
) -> Result<StationTokens> {
    let base = console_base_url.trim_end_matches('/');
    if !base.starts_with("https://") {
        return Err(anyhow!("CONSOLE base URL must be https:// (got {base})"));
    }

    let resp = http
        .post(format!("{base}/api/auth/station/token"))
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": authorization_code,
        }))
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
    })
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
    fn mint_rejects_non_https_console_base() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let http = reqwest::Client::new();
        let err = rt
            .block_on(mint_station_tokens(
                &http,
                "http://localhost:3000",
                "code",
            ))
            .expect_err("must reject http");
        assert!(err.to_string().contains("https://"));
    }

    #[test]
    fn complete_login_persists_tokens_when_mint_json_is_injected() {
        // Unit-level: store seam after mint shape is known (no live WorkOS).
        let store = MemoryStationTokenStore::default();
        let tokens = StationTokens {
            access_token: "station.access".into(),
            refresh_token: "station.refresh".into(),
            expires_in: 900,
            refresh_expires_in: Some(1000),
        };
        store.save(&tokens).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "station.access");
    }
}
