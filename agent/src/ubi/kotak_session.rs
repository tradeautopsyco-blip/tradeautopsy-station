//! Kotak Neo TOTP session mint (host-side).
//!
//! Ports the official SDK login → validate flow into the Enforcer.
//! See `docs/reference/equities/kotak-neo/SESSION-AND-TRADES.md`.
//!
//! Secrets (TOTP, MPIN, tokens) must never reach logs, SSE, or Wasm.

use crate::ubi::credentials::CredentialBlob;
use serde::Deserialize;
use serde_json::json;

/// Prod login host (`urls.BASE_URL` when `session_init=True`).
pub const KOTAK_LOGIN_BASE: &str = "https://mis.kotaksecurities.com";
/// Default `neo-fin-key` for prod (`neo_utility.get_neo_fin_key`).
pub const KOTAK_NEO_FIN_KEY: &str = "neotradeapi";
const TOTP_LOGIN_PATH: &str = "/login/1.0/tradeApiLogin";
const TOTP_VALIDATE_PATH: &str = "/login/1.0/tradeApiValidate";

#[derive(Debug, Clone)]
pub struct KotakMintRequest {
    pub consumer_key: String,
    pub mobile_number: String,
    pub ucc: String,
    pub totp: String,
    pub mpin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KotakMintErrorClass {
    InvalidCredentials,
    TotpFailed,
    MpinFailed,
    Upstream,
}

impl KotakMintErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::TotpFailed => "totp_failed",
            Self::MpinFailed => "mpin_failed",
            Self::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KotakMintError {
    pub class: KotakMintErrorClass,
    pub message: String,
}

impl KotakMintError {
    fn new(class: KotakMintErrorClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintedKotakSession {
    pub consumer_key: String,
    pub trade_token: String,
    pub sid: String,
    pub base_url: String,
    pub hs_server_id: String,
}

impl MintedKotakSession {
    pub fn into_credential_blob(self) -> CredentialBlob {
        CredentialBlob::KotakNeoTotpSession {
            consumer_key: self.consumer_key,
            trade_token: self.trade_token,
            sid: self.sid,
            base_url: self.base_url,
            hs_server_id: self.hs_server_id,
            expires_at: None,
        }
    }
}

pub trait KotakSessionHttp: Send + Sync {
    fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
    ) -> Result<(u16, String), String>;
}

pub struct ReqwestKotakSessionHttp {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl ReqwestKotakSessionHttp {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("kotak http client: {e}"))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("kotak http runtime: {e}"))?;
        Ok(Self { client, runtime })
    }
}

impl Default for ReqwestKotakSessionHttp {
    fn default() -> Self {
        Self::new().expect("kotak session http")
    }
}

impl KotakSessionHttp for ReqwestKotakSessionHttp {
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
    // Never echo URL (may carry tokens in rare misconfigs).
    if err.is_timeout() {
        "kotak http: timeout".into()
    } else if err.is_connect() {
        "kotak http: connect".into()
    } else if err.is_request() {
        "kotak http: request".into()
    } else if err.is_body() || err.is_decode() {
        "kotak http: body".into()
    } else {
        "kotak http: error".into()
    }
}

/// Mint a trade session via TOTP login → MPIN validate (SDK-aligned).
pub fn mint_totp_session(
    http: &dyn KotakSessionHttp,
    request: &KotakMintRequest,
) -> Result<MintedKotakSession, KotakMintError> {
    validate_mint_inputs(request)?;

    let login_headers = vec![
        ("Authorization".into(), request.consumer_key.clone()),
        ("neo-fin-key".into(), KOTAK_NEO_FIN_KEY.into()),
        ("Content-Type".into(), "application/json".into()),
    ];
    let login_body = json!({
        "mobileNumber": request.mobile_number,
        "ucc": request.ucc,
        "totp": request.totp,
    });
    let login_url = format!("{KOTAK_LOGIN_BASE}{TOTP_LOGIN_PATH}");
    let (login_status, login_body_text) =
        http.post_json(&login_url, &login_headers, &login_body)
            .map_err(|e| KotakMintError::new(KotakMintErrorClass::Upstream, e))?;

    let login = parse_session_payload(
        login_status,
        &login_body_text,
        KotakMintErrorClass::TotpFailed,
    )?;
    let view_token = required_field(&login, "token", KotakMintErrorClass::TotpFailed)?;
    let view_sid = required_field(&login, "sid", KotakMintErrorClass::TotpFailed)?;

    let validate_headers = vec![
        ("Authorization".into(), request.consumer_key.clone()),
        ("sid".into(), view_sid),
        ("Auth".into(), view_token),
        ("neo-fin-key".into(), KOTAK_NEO_FIN_KEY.into()),
        ("Content-Type".into(), "application/json".into()),
    ];
    let validate_body = json!({ "mpin": request.mpin });
    let validate_url = format!("{KOTAK_LOGIN_BASE}{TOTP_VALIDATE_PATH}");
    let (validate_status, validate_body_text) = http
        .post_json(&validate_url, &validate_headers, &validate_body)
        .map_err(|e| KotakMintError::new(KotakMintErrorClass::Upstream, e))?;

    let trade = parse_session_payload(
        validate_status,
        &validate_body_text,
        KotakMintErrorClass::MpinFailed,
    )?;
    let trade_token = required_field(&trade, "token", KotakMintErrorClass::MpinFailed)?;
    let sid = required_field(&trade, "sid", KotakMintErrorClass::MpinFailed)?;
    // SDK sample often returns empty hsServerId; trade_report still works with empty sId.
    // Accept alternate spellings when present.
    let hs_server_id = optional_field(&trade, &["hsServerId", "hsServerID", "serverId", "sId"])
        .unwrap_or_default();
    let base_url = optional_field(&trade, &["baseUrl", "base_url", "baseURL"])
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            KotakMintError::new(
                KotakMintErrorClass::MpinFailed,
                "kotak session missing baseUrl",
            )
        })?;

    Ok(MintedKotakSession {
        consumer_key: request.consumer_key.clone(),
        trade_token,
        sid,
        base_url,
        hs_server_id,
    })
}

fn validate_mint_inputs(request: &KotakMintRequest) -> Result<(), KotakMintError> {
    if request.consumer_key.trim().is_empty()
        || request.mobile_number.trim().is_empty()
        || request.ucc.trim().is_empty()
        || request.totp.trim().is_empty()
        || request.mpin.trim().is_empty()
    {
        return Err(KotakMintError::new(
            KotakMintErrorClass::InvalidCredentials,
            "consumer key, mobile, ucc, totp, and mpin are required",
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct SessionEnvelope {
    data: Option<serde_json::Value>,
}

fn parse_session_payload(
    status: u16,
    body: &str,
    fail_class: KotakMintErrorClass,
) -> Result<serde_json::Value, KotakMintError> {
    if !(200..=299).contains(&status) {
        let detail = extract_kotak_error_message(body)
            .unwrap_or_else(|| format!("kotak session http {status}"));
        return Err(KotakMintError::new(fail_class, detail));
    }
    let envelope: SessionEnvelope = serde_json::from_str(body)
        .map_err(|_| KotakMintError::new(fail_class, "kotak session response not json"))?;
    match envelope.data {
        Some(data) if !data.is_null() => Ok(data),
        _ => {
            let detail = extract_kotak_error_message(body)
                .unwrap_or_else(|| "kotak session response missing data".into());
            Err(KotakMintError::new(fail_class, detail))
        }
    }
}

/// Pull a human-readable Kotak error without echoing tokens / secrets.
fn extract_kotak_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(arr) = value.get("error").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(msg) = item.get("message").and_then(|v| v.as_str()) {
                let cleaned = sanitize_kotak_error_message(msg);
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
    }
    for key in ["message", "error", "emsg", "msg"] {
        if let Some(msg) = value.get(key).and_then(|v| v.as_str()) {
            let cleaned = sanitize_kotak_error_message(msg);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn sanitize_kotak_error_message(raw: &str) -> String {
    // Drop long token-like substrings; keep short operational messages.
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

fn required_field(
    data: &serde_json::Value,
    key: &str,
    fail_class: KotakMintErrorClass,
) -> Result<String, KotakMintError> {
    optional_field(data, &[key])
        .ok_or_else(|| KotakMintError::new(fail_class, format!("kotak session missing {key}")))
}

fn optional_field(data: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = data.get(*key) {
            if let Some(s) = v.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            if let Some(n) = v.as_i64() {
                return Some(n.to_string());
            }
            if let Some(n) = v.as_u64() {
                return Some(n.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct ScriptedHttp {
        calls: Mutex<Vec<(String, Vec<(String, String)>, serde_json::Value)>>,
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

    impl KotakSessionHttp for ScriptedHttp {
        fn post_json(
            &self,
            url: &str,
            headers: &[(String, String)],
            body: &serde_json::Value,
        ) -> Result<(u16, String), String> {
            self.calls
                .lock()
                .unwrap()
                .push((url.to_string(), headers.to_vec(), body.clone()));
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err("no scripted response".into());
            }
            Ok(responses.remove(0))
        }
    }

    fn sample_request() -> KotakMintRequest {
        KotakMintRequest {
            consumer_key: "ck-token".into(),
            mobile_number: "+919999996708".into(),
            ucc: "ABC12".into(),
            totp: "123456".into(),
            mpin: "654321".into(),
        }
    }

    #[test]
    fn mint_shapes_login_and_validate_like_sdk() {
        let login_json = r#"{"data":{"token":"view-tok","sid":"view-sid"}}"#;
        let validate_json = r#"{
            "data":{
                "token":"trade-tok",
                "sid":"trade-sid",
                "hsServerId":"server4",
                "baseUrl":"https://gw-napi.kotaksecurities.com/trading"
            }
        }"#;
        let http = ScriptedHttp::new(vec![(200, login_json.into()), (200, validate_json.into())]);
        let minted = mint_totp_session(&http, &sample_request()).expect("mint");
        assert_eq!(minted.trade_token, "trade-tok");
        assert_eq!(minted.sid, "trade-sid");
        assert_eq!(minted.hs_server_id, "server4");
        assert_eq!(
            minted.base_url,
            "https://gw-napi.kotaksecurities.com/trading"
        );

        let calls = http.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].0.ends_with("/login/1.0/tradeApiLogin"));
        assert!(calls[0]
            .1
            .iter()
            .any(|(n, v)| n == "Authorization" && v == "ck-token"));
        assert!(calls[0]
            .1
            .iter()
            .any(|(n, v)| n == "neo-fin-key" && v == "neotradeapi"));
        assert_eq!(calls[0].2["mobileNumber"], "+919999996708");
        assert_eq!(calls[0].2["totp"], "123456");

        assert!(calls[1].0.ends_with("/login/1.0/tradeApiValidate"));
        assert!(calls[1]
            .1
            .iter()
            .any(|(n, v)| n == "Auth" && v == "view-tok"));
        assert!(calls[1]
            .1
            .iter()
            .any(|(n, v)| n == "sid" && v == "view-sid"));
        assert_eq!(calls[1].2["mpin"], "654321");

        // Secrets must not appear in error messages from success path.
        let blob = minted.into_credential_blob();
        match blob {
            CredentialBlob::KotakNeoTotpSession {
                hs_server_id,
                trade_token,
                ..
            } => {
                assert_eq!(hs_server_id, "server4");
                assert_eq!(trade_token, "trade-tok");
            }
            _ => panic!("expected kotak blob"),
        }
    }

    #[test]
    fn empty_inputs_are_invalid_credentials() {
        let http = ScriptedHttp::new(vec![]);
        let err = mint_totp_session(
            &http,
            &KotakMintRequest {
                consumer_key: "".into(),
                mobile_number: "x".into(),
                ucc: "y".into(),
                totp: "1".into(),
                mpin: "2".into(),
            },
        )
        .expect_err("empty");
        assert_eq!(err.class, KotakMintErrorClass::InvalidCredentials);
    }

    #[test]
    fn login_http_failure_is_totp_failed() {
        let http = ScriptedHttp::new(vec![(401, r#"{"error":"no"}"#.into())]);
        let err = mint_totp_session(&http, &sample_request()).expect_err("totp");
        assert_eq!(err.class, KotakMintErrorClass::TotpFailed);
        assert!(!err.message.contains("123456"));
        assert!(!err.message.contains("654321"));
    }

    #[test]
    fn login_424_surfaces_kotak_error_message_without_token() {
        let body = r#"{"error":[{"code":"424","message":"Consumer key deadbeef-dead-beef-dead-beefdeadbeef does not exist"}]}"#;
        let http = ScriptedHttp::new(vec![(424, body.into())]);
        let err = mint_totp_session(&http, &sample_request()).expect_err("totp");
        assert_eq!(err.class, KotakMintErrorClass::TotpFailed);
        assert!(err.message.contains("does not exist"), "{}", err.message);
        assert!(!err.message.contains("deadbeef-dead-beef-dead-beefdeadbeef"));
    }

    #[test]
    fn validate_http_failure_is_mpin_failed() {
        let login_json = r#"{"data":{"token":"view-tok","sid":"view-sid"}}"#;
        let http = ScriptedHttp::new(vec![
            (200, login_json.into()),
            (
                403,
                r#"{"error":[{"code":"403","message":"Invalid MPIN"}]}"#.into(),
            ),
        ]);
        let err = mint_totp_session(&http, &sample_request()).expect_err("mpin");
        assert_eq!(err.class, KotakMintErrorClass::MpinFailed);
        assert!(err.message.contains("Invalid MPIN"), "{}", err.message);
        assert!(!err.message.contains("654321"));
    }

    #[test]
    fn mint_allows_empty_hs_server_id_like_sdk_sample() {
        let login_json = r#"{"data":{"token":"view-tok","sid":"view-sid"}}"#;
        let validate_json = r#"{
            "data":{
                "token":"trade-tok",
                "sid":"trade-sid",
                "hsServerId":"",
                "baseUrl":"https://gw-napi.kotaksecurities.com/trading"
            }
        }"#;
        let http = ScriptedHttp::new(vec![(200, login_json.into()), (200, validate_json.into())]);
        let minted = mint_totp_session(&http, &sample_request()).expect("mint");
        assert_eq!(minted.hs_server_id, "");
        assert_eq!(minted.trade_token, "trade-tok");
        assert_eq!(
            minted.base_url,
            "https://gw-napi.kotaksecurities.com/trading"
        );
    }

    #[test]
    fn mint_accepts_server_id_alias() {
        let login_json = r#"{"data":{"token":"view-tok","sid":"view-sid"}}"#;
        let validate_json = r#"{
            "data":{
                "token":"trade-tok",
                "sid":"trade-sid",
                "serverId":"server4",
                "baseUrl":"https://mnapi.kotaksecurities.com"
            }
        }"#;
        let http = ScriptedHttp::new(vec![(200, login_json.into()), (200, validate_json.into())]);
        let minted = mint_totp_session(&http, &sample_request()).expect("mint");
        assert_eq!(minted.hs_server_id, "server4");
    }
}
