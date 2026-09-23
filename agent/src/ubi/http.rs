//! Live `broker_http_call` transport + host auth attach (Phase 3, R6 §3.5).
//!
//! Secrets live only in [`HostCredentialBlob`]. The component supplies method/path/
//! query/body shape; the host signs (Binance HMAC) or attaches session headers
//! (Kotak `Auth`/`Sid`) here, outside Wasm memory, and redacts the response.

use crate::ubi::credentials::CredentialBlob;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::{Arc, OnceLock};

type HmacSha256 = Hmac<Sha256>;

/// Host-only credential material. Never copied into Wasm component memory / HTTP responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCredentialBlob {
    Hmac {
        api_key: String,
        api_secret: String,
    },
    KotakSession {
        consumer_key: String,
        trade_token: String,
        sid: String,
        base_url: String,
        hs_server_id: String,
    },
    KiteSession {
        api_key: String,
        access_token: String,
    },
    UpstoxSession {
        access_token: String,
    },
}

impl HostCredentialBlob {
    pub fn hmac(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self::Hmac {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    /// Values that must never reach the component (leak scan input).
    pub fn secret_values(&self) -> Vec<&str> {
        match self {
            Self::Hmac {
                api_key,
                api_secret,
            } => vec![api_key.as_str(), api_secret.as_str()],
            Self::KotakSession {
                consumer_key,
                trade_token,
                sid,
                hs_server_id,
                ..
            } => vec![
                consumer_key.as_str(),
                trade_token.as_str(),
                sid.as_str(),
                hs_server_id.as_str(),
            ],
            Self::KiteSession {
                api_key,
                access_token,
            } => vec![api_key.as_str(), access_token.as_str()],
            Self::UpstoxSession { access_token } => vec![access_token.as_str()],
        }
    }

    pub fn api_key_for_tests(&self) -> Option<&str> {
        match self {
            Self::Hmac { api_key, .. } => Some(api_key),
            Self::KotakSession { .. } | Self::KiteSession { .. } | Self::UpstoxSession { .. } => {
                None
            }
        }
    }
}

impl From<&CredentialBlob> for HostCredentialBlob {
    fn from(blob: &CredentialBlob) -> Self {
        match blob {
            CredentialBlob::HmacApiKeySecret {
                api_key,
                api_secret,
            } => Self::hmac(api_key, api_secret),
            CredentialBlob::KotakNeoTotpSession {
                consumer_key,
                trade_token,
                sid,
                base_url,
                hs_server_id,
                ..
            } => Self::KotakSession {
                consumer_key: consumer_key.clone(),
                trade_token: trade_token.clone(),
                sid: sid.clone(),
                base_url: base_url.clone(),
                hs_server_id: hs_server_id.clone(),
            },
            CredentialBlob::KiteChecksumSession {
                api_key,
                access_token,
                ..
            } => Self::KiteSession {
                api_key: api_key.clone(),
                access_token: access_token.clone(),
            },
            CredentialBlob::UpstoxOAuthBearerSession { access_token, .. } => {
                Self::UpstoxSession {
                    access_token: access_token.clone(),
                }
            }
        }
    }
}

/// Response headers the host is willing to hand back to a component.
/// Everything else (notably `set-cookie`) is dropped.
pub const RESPONSE_HEADER_ALLOWLIST: &[&str] = &[
    "content-type",
    "retry-after",
    "x-mbx-used-weight",
    "x-mbx-used-weight-1m",
    "x-mbx-order-count-10s",
    "x-mbx-order-count-1d",
];

/// Fully-formed, auth-attached request. Host-side only — never crosses into Wasm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub trait BrokerHttpTransport: Send + Sync {
    fn send(&self, request: &PreparedHttpRequest) -> Result<TransportResponse, String>;
}

/// Host the request actually goes to. Kotak's base URL is credential-carried (R6 §3.4),
/// so the component's placeholder host is replaced before the allowlist check.
pub fn effective_host(component_host: &str, credentials: &HostCredentialBlob) -> String {
    match credentials {
        HostCredentialBlob::KotakSession { base_url, .. } => {
            host_of(base_url).unwrap_or_else(|| component_host.trim().to_ascii_lowercase())
        }
        HostCredentialBlob::KiteSession { .. } => {
            crate::ubi::zerodha_session::KITE_API_HOST.to_string()
        }
        HostCredentialBlob::UpstoxSession { .. } => {
            crate::ubi::upstox_session::UPSTOX_API_HOST.to_string()
        }
        HostCredentialBlob::Hmac { .. } => component_host.trim().to_ascii_lowercase(),
    }
}

/// Host from a Kotak session `baseUrl`, without scheme, userinfo, or `:port`.
pub fn kotak_base_host(base_url: &str) -> Option<String> {
    host_of(base_url)
}

fn host_of(base_url: &str) -> Option<String> {
    let without_scheme = base_url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let hostport = without_scheme.split('/').next()?.trim();
    let hostport = hostport
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(hostport);
    let host = match hostport.rsplit_once(':') {
        Some((name, port)) if port.chars().all(|c| c.is_ascii_digit()) => name,
        _ => hostport,
    };
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// Path prefix carried by a Kotak base URL (e.g. `/trading`).
fn base_path_prefix(base_url: &str) -> String {
    let without_scheme = base_url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    match without_scheme.find('/') {
        Some(idx) => without_scheme[idx..].trim_end_matches('/').to_string(),
        None => String::new(),
    }
}

fn encode(value: &str) -> String {
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

fn sign_query(api_secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Attach auth for the connection's scheme. `now_unix_ms` is injected for deterministic tests.
pub fn prepare_request(
    method: &str,
    host: &str,
    path: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    body: Option<&str>,
    credentials: &HostCredentialBlob,
    now_unix_ms: i64,
) -> PreparedHttpRequest {
    let mut out_headers: Vec<(String, String)> = headers
        .iter()
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();

    match credentials {
        HostCredentialBlob::Hmac {
            api_key,
            api_secret,
        } => {
            let mut pairs: Vec<(String, String)> = query.to_vec();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs.push(("timestamp".to_string(), now_unix_ms.to_string()));
            let canonical = pairs
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            let signature = sign_query(api_secret, &canonical);
            out_headers.push(("X-MBX-APIKEY".to_string(), api_key.clone()));
            PreparedHttpRequest {
                method: method.to_ascii_uppercase(),
                url: format!("https://{host}{path}?{canonical}&signature={signature}"),
                headers: out_headers,
                body: body.map(|b| b.to_string()),
            }
        }
        HostCredentialBlob::KotakSession {
            trade_token,
            sid,
            base_url,
            hs_server_id,
            ..
        } => {
            // SDK TradeReportAPI always sends `sId` = hsServerId (docs/reference/equities/kotak-neo).
            let mut pairs: Vec<(String, String)> = query.to_vec();
            let has_sid = pairs.iter().any(|(k, _)| k.eq_ignore_ascii_case("sId"));
            if !has_sid && !hs_server_id.trim().is_empty() {
                pairs.push(("sId".to_string(), hs_server_id.clone()));
            }
            let canonical = pairs
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            let prefix = base_path_prefix(base_url);
            let url = if canonical.is_empty() {
                format!("https://{host}{prefix}{path}")
            } else {
                format!("https://{host}{prefix}{path}?{canonical}")
            };
            out_headers.push(("Auth".to_string(), trade_token.clone()));
            out_headers.push(("Sid".to_string(), sid.clone()));
            out_headers.push(("Accept".to_string(), "application/json".to_string()));
            PreparedHttpRequest {
                method: method.to_ascii_uppercase(),
                url,
                headers: out_headers,
                body: body.map(|b| b.to_string()),
            }
        }
        HostCredentialBlob::KiteSession {
            api_key,
            access_token,
        } => {
            let canonical = query
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            let url = if canonical.is_empty() {
                format!("https://{host}{path}")
            } else {
                format!("https://{host}{path}?{canonical}")
            };
            out_headers.push((
                "Authorization".to_string(),
                crate::ubi::zerodha_session::kite_authorization_header_value(
                    api_key,
                    access_token,
                ),
            ));
            out_headers.push((
                "X-Kite-Version".to_string(),
                crate::ubi::zerodha_session::KITE_API_VERSION_HEADER.to_string(),
            ));
            out_headers.push(("Accept".to_string(), "application/json".to_string()));
            PreparedHttpRequest {
                method: method.to_ascii_uppercase(),
                url,
                headers: out_headers,
                body: body.map(|b| b.to_string()),
            }
        }
        HostCredentialBlob::UpstoxSession { access_token } => {
            let canonical = query
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            let url = if canonical.is_empty() {
                format!("https://{host}{path}")
            } else {
                format!("https://{host}{path}?{canonical}")
            };
            out_headers.push((
                "Authorization".to_string(),
                crate::ubi::upstox_session::upstox_bearer_authorization_header_value(
                    access_token,
                ),
            ));
            out_headers.push(("Accept".to_string(), "application/json".to_string()));
            PreparedHttpRequest {
                method: method.to_ascii_uppercase(),
                url,
                headers: out_headers,
                body: body.map(|b| b.to_string()),
            }
        }
    }
}

/// Scrip-master file-paths and REST quotes GET (`scrip_master_api.py` /
/// `quotes_neo_symbol_api.py`): `Authorization` = consumer key, Auth+Sid for the
/// completed 2FA session, **no** trade-book `sId`. Extra `sId` on this GET is
/// unspecified and can 400 the catalog so cash CSVs never start.
pub fn prepare_kotak_catalog_get(
    path: &str,
    credentials: &HostCredentialBlob,
) -> Result<PreparedHttpRequest, String> {
    let HostCredentialBlob::KotakSession {
        consumer_key,
        trade_token,
        sid,
        base_url,
        ..
    } = credentials
    else {
        return Err("kotak catalog GET requires Kotak session credentials".into());
    };
    let host = host_of(base_url).ok_or_else(|| "kotak baseUrl has no host".to_string())?;
    let prefix = base_path_prefix(base_url);
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    Ok(PreparedHttpRequest {
        method: "GET".into(),
        url: format!("https://{host}{prefix}{path}"),
        headers: vec![
            ("Authorization".into(), consumer_key.clone()),
            (
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            ),
            ("Auth".into(), trade_token.clone()),
            ("Sid".into(), sid.clone()),
            ("Accept".into(), "application/json".into()),
        ],
        body: None,
    })
}

/// Scrip-master file-paths GET — SDK first (`scrip_master_api.py`): consumer-key
/// `Authorization` only. No Auth, Sid, or trade-book `sId`. Host from `kotak_base_host`
/// (port already stripped). REST quotes keep [`prepare_kotak_catalog_get`].
pub fn prepare_kotak_file_paths_get(
    path: &str,
    credentials: &HostCredentialBlob,
) -> Result<PreparedHttpRequest, String> {
    let HostCredentialBlob::KotakSession {
        consumer_key,
        base_url,
        ..
    } = credentials
    else {
        return Err("kotak file-paths GET requires Kotak session credentials".into());
    };
    let host = host_of(base_url).ok_or_else(|| "kotak baseUrl has no host".to_string())?;
    let prefix = base_path_prefix(base_url);
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    Ok(PreparedHttpRequest {
        method: "GET".into(),
        url: format!("https://{host}{prefix}{path}"),
        headers: vec![
            ("Authorization".into(), consumer_key.clone()),
            (
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            ),
            ("Accept".into(), "application/json".into()),
        ],
        body: None,
    })
}

/// Market-data GET (`historical_data` / later expiries). SDK `get_url_details`:
/// `{host}/market-data/…` — consumer_key `Authorization` only, **no** session
/// `/trading` prefix, no Auth/Sid. `historical_data.py` + `test_neo_utility.py`.
pub fn prepare_kotak_market_data_get(
    path: &str,
    credentials: &HostCredentialBlob,
) -> Result<PreparedHttpRequest, String> {
    let HostCredentialBlob::KotakSession {
        consumer_key,
        base_url,
        ..
    } = credentials
    else {
        return Err("kotak market-data GET requires Kotak session credentials".into());
    };
    let host = host_of(base_url).ok_or_else(|| "kotak baseUrl has no host".to_string())?;
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    Ok(PreparedHttpRequest {
        method: "GET".into(),
        url: format!("https://{host}{path}"),
        headers: vec![
            ("Authorization".into(), consumer_key.clone()),
            (
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            ),
            ("Accept".into(), "application/json".into()),
        ],
        body: None,
    })
}

/// One retry after SDK 401/403 / "Complete the 2fa process": attach Auth+Sid, still no `sId`.
pub fn attach_kotak_file_paths_session(
    mut prepared: PreparedHttpRequest,
    credentials: &HostCredentialBlob,
) -> Result<PreparedHttpRequest, String> {
    let HostCredentialBlob::KotakSession {
        trade_token, sid, ..
    } = credentials
    else {
        return Err("kotak file-paths session attach requires Kotak session credentials".into());
    };
    prepared.headers.retain(|(name, _)| {
        !name.eq_ignore_ascii_case("Auth") && !name.eq_ignore_ascii_case("Sid")
    });
    prepared.headers.push(("Auth".into(), trade_token.clone()));
    prepared.headers.push(("Sid".into(), sid.clone()));
    Ok(prepared)
}

/// Public market call: no Keychain material. Host still allowlists host/path.
pub fn prepare_unsigned_request(
    method: &str,
    host: &str,
    path: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    body: Option<&str>,
) -> PreparedHttpRequest {
    let canonical = query
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let url = if canonical.is_empty() {
        format!("https://{host}{path}")
    } else {
        format!("https://{host}{path}?{canonical}")
    };
    PreparedHttpRequest {
        method: method.to_ascii_uppercase(),
        url,
        headers: headers.to_vec(),
        body: body.map(|b| b.to_string()),
    }
}

/// Host classification the component may act on without seeing credentials (R6 §3.3).
pub fn classify_response(status: u16, body: &str) -> Option<String> {
    if body_signals_expired_session(body) {
        return Some("session_expired".to_string());
    }
    match status {
        200..=299 => None,
        401 | 403 => Some("unauthorized".to_string()),
        418 | 429 => Some("rate_limited".to_string()),
        500..=599 => Some("server_error".to_string()),
        _ => Some("http_error".to_string()),
    }
}

/// Kotak returns HTTP 200 with `stCode` 1003 for a dead session (B6 kotak_neo §9).
fn body_signals_expired_session(body: &str) -> bool {
    let head = &body[..body.len().min(2048)];
    head.contains("\"stCode\":1003")
        || head.contains("\"stCode\": 1003")
        || head.contains("Invalid Session")
        || head.contains("Complete the 2fa process")
        || head.contains("TokenException")
        || head.contains("UDAPI100050")
        || head.contains("Invalid token")
}

pub fn redact_response_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| {
            RESPONSE_HEADER_ALLOWLIST
                .iter()
                .any(|allowed| name.eq_ignore_ascii_case(allowed))
        })
        .map(|(n, v)| (n.to_ascii_lowercase(), v.clone()))
        .collect()
}

/// Live transport: async reqwest driven by a dedicated current-thread runtime so the
/// synchronous Wasm import can block without borrowing the agent's runtime workers.
pub struct ReqwestBrokerHttpTransport {
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
}

impl ReqwestBrokerHttpTransport {
    pub fn new() -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("ubi http runtime: {e}"))?;
        Ok(Self {
            // Not a new pool: a clone of the egress client, so this path and the
            // async desk paths share one set of sockets. Admission happens in
            // `ubi::host::broker_http_call`, which is where the book id and auth
            // mode are in scope.
            client: crate::egress::shared_client(),
            runtime,
        })
    }

    /// Process-wide transport (one runtime, reused across syncs).
    pub fn shared() -> Result<Arc<dyn BrokerHttpTransport>, String> {
        static SHARED: OnceLock<Result<Arc<ReqwestBrokerHttpTransport>, String>> = OnceLock::new();
        SHARED
            .get_or_init(|| Self::new().map(Arc::new))
            .clone()
            .map(|t| t as Arc<dyn BrokerHttpTransport>)
    }
}

impl BrokerHttpTransport for ReqwestBrokerHttpTransport {
    fn send(&self, request: &PreparedHttpRequest) -> Result<TransportResponse, String> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|e| format!("bad method {}: {e}", request.method))?;
        let mut builder = self.client.request(method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        self.runtime.block_on(async move {
            let response = builder
                .send()
                .await
                .map_err(|e| format!("network: {}", redact_url(&e.to_string())))?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(n, v)| {
                    (
                        n.as_str().to_string(),
                        v.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            let body = response
                .text()
                .await
                .map_err(|e| format!("network: {}", redact_url(&e.to_string())))?;
            Ok(TransportResponse {
                status,
                headers,
                body,
            })
        })
    }
}

/// Offline transport for tests: routes by URL substring and records what the host sent,
/// which is how we prove auth is attached host-side rather than inside the component.
pub struct RecordingTransport {
    routes: Vec<(String, TransportResponse)>,
    fallback: Result<TransportResponse, String>,
    sent: std::sync::Mutex<Vec<PreparedHttpRequest>>,
}

impl RecordingTransport {
    pub fn new(fallback: Result<TransportResponse, String>) -> Self {
        Self {
            routes: Vec::new(),
            fallback,
            sent: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn ok(status: u16, body: impl Into<String>) -> Self {
        Self::new(Ok(TransportResponse {
            status,
            headers: vec![("content-type".into(), "application/json".into())],
            body: body.into(),
        }))
    }

    /// `(url substring, response)` — first match wins.
    pub fn routed(routes: Vec<(String, TransportResponse)>) -> Self {
        Self {
            routes,
            fallback: Ok(TransportResponse {
                status: 404,
                headers: vec![],
                body: "{\"error\":\"no route\"}".into(),
            }),
            sent: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn json_route(url_contains: &str, status: u16, body: &str) -> (String, TransportResponse) {
        (
            url_contains.to_string(),
            TransportResponse {
                status,
                headers: vec![("content-type".into(), "application/json".into())],
                body: body.to_string(),
            },
        )
    }

    pub fn sent(&self) -> Vec<PreparedHttpRequest> {
        self.sent.lock().expect("sent log").clone()
    }

    pub fn last(&self) -> Option<PreparedHttpRequest> {
        self.sent.lock().expect("sent log").last().cloned()
    }
}

impl BrokerHttpTransport for RecordingTransport {
    fn send(&self, request: &PreparedHttpRequest) -> Result<TransportResponse, String> {
        self.sent.lock().expect("sent log").push(request.clone());
        for (needle, response) in &self.routes {
            if request.url.contains(needle.as_str()) {
                return Ok(response.clone());
            }
        }
        self.fallback.clone()
    }
}

/// reqwest error strings embed the URL — which carries the HMAC signature.
fn redact_url(message: &str) -> String {
    match message.find("http") {
        Some(idx) => format!("{}<redacted-url>", &message[..idx]),
        None => message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hmac_creds() -> HostCredentialBlob {
        HostCredentialBlob::hmac("KEY123", "secret")
    }

    fn kotak_creds() -> HostCredentialBlob {
        HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt-token".into(),
            sid: "sid-1".into(),
            base_url: "https://cis.kotaksecurities.com/trading".into(),
            hs_server_id: "server4".into(),
        }
    }

    #[test]
    fn hmac_attach_signs_query_with_timestamp_and_sets_api_key_header() {
        let prepared = prepare_request(
            "get",
            "api.binance.com",
            "/api/v3/myTrades",
            &[("symbol".into(), "BTCUSDT".into())],
            &[],
            None,
            &hmac_creds(),
            1_700_000_000_000,
        );
        assert_eq!(prepared.method, "GET");
        let expected_query = "symbol=BTCUSDT&timestamp=1700000000000";
        let expected_sig = sign_query("secret", expected_query);
        assert_eq!(
            prepared.url,
            format!(
                "https://api.binance.com/api/v3/myTrades?{expected_query}&signature={expected_sig}"
            )
        );
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "X-MBX-APIKEY" && v == "KEY123"));
    }

    #[test]
    fn unsigned_public_call_has_no_signature_or_api_key() {
        let prepared = prepare_unsigned_request(
            "GET",
            "api.binance.com",
            "/api/v3/ticker/price",
            &[("symbol".into(), "BTCUSDT".into())],
            &[],
            None,
        );
        assert_eq!(
            prepared.url,
            "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT"
        );
        assert!(!prepared.url.contains("signature="));
        assert!(!prepared.url.contains("timestamp="));
        assert!(prepared.headers.is_empty());
    }

    #[test]
    fn hmac_query_is_sorted_before_timestamp_like_native_client() {
        let prepared = prepare_request(
            "GET",
            "api.binance.com",
            "/api/v3/myTrades",
            &[
                ("symbol".into(), "BTCUSDT".into()),
                ("limit".into(), "500".into()),
            ],
            &[],
            None,
            &hmac_creds(),
            42,
        );
        assert!(prepared
            .url
            .contains("limit=500&symbol=BTCUSDT&timestamp=42"));
    }

    #[test]
    fn kotak_attach_uses_blob_base_url_and_session_headers() {
        let creds = kotak_creds();
        assert_eq!(
            effective_host("cis.kotaksecurities.com", &creds),
            "cis.kotaksecurities.com"
        );
        let prepared = prepare_request(
            "GET",
            "cis.kotaksecurities.com",
            "/quick/user/trades",
            &[],
            &[],
            None,
            &creds,
            0,
        );
        assert_eq!(
            prepared.url,
            "https://cis.kotaksecurities.com/trading/quick/user/trades?sId=server4"
        );
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Auth" && v == "tt-token"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Sid" && v == "sid-1"));
        assert!(!prepared.url.contains("tt-token"));
    }

    #[test]
    fn kotak_does_not_duplicate_s_id_when_query_already_has_it() {
        let prepared = prepare_request(
            "GET",
            "cis.kotaksecurities.com",
            "/quick/user/trades",
            &[("sId".into(), "existing".into())],
            &[],
            None,
            &kotak_creds(),
            0,
        );
        assert_eq!(
            prepared.url,
            "https://cis.kotaksecurities.com/trading/quick/user/trades?sId=existing"
        );
        assert!(!prepared.url.contains("server4"));
    }

    #[test]
    fn kotak_base_url_host_overrides_component_supplied_host() {
        let creds = HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://neo.kotaksecurities.com".into(),
            hs_server_id: "server4".into(),
        };
        assert_eq!(
            effective_host("cis.kotaksecurities.com", &creds),
            "neo.kotaksecurities.com"
        );
    }

    #[test]
    fn kotak_catalog_get_uses_consumer_key_and_omits_tradebook_sid() {
        let prepared =
            prepare_kotak_catalog_get("/script-details/1.0/masterscrip/file-paths", &kotak_creds())
                .unwrap();
        assert_eq!(
            prepared.url,
            "https://cis.kotaksecurities.com/trading/script-details/1.0/masterscrip/file-paths"
        );
        assert!(!prepared.url.contains("sId"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Authorization" && v == "ck"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Auth" && v == "tt-token"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Sid" && v == "sid-1"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Content-Type" && v == "application/x-www-form-urlencoded"));
    }

    #[test]
    fn kotak_file_paths_sdk_omits_sid_auth_and_session_headers() {
        let prepared = prepare_kotak_file_paths_get(
            "/script-details/1.0/masterscrip/file-paths",
            &kotak_creds(),
        )
        .unwrap();
        assert_eq!(
            prepared.url,
            "https://cis.kotaksecurities.com/trading/script-details/1.0/masterscrip/file-paths"
        );
        assert!(!prepared.url.contains("sId"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Authorization" && v == "ck"));
        assert!(!prepared
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Auth")));
        assert!(!prepared
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Sid")));
        assert!(!prepared
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("sId")));
    }

    #[test]
    fn kotak_market_data_get_omits_trading_prefix_and_session_headers() {
        let creds = kotak_creds();
        let prepared = prepare_kotak_market_data_get(
            "/market-data/1.0/historical/details?neosymbol=nse_cm%7C1333&interval=15min&fromdate=2026-08-20&todate=2026-09-01",
            &creds,
        )
        .unwrap();
        assert_eq!(
            prepared.url,
            "https://cis.kotaksecurities.com/market-data/1.0/historical/details?neosymbol=nse_cm%7C1333&interval=15min&fromdate=2026-08-20&todate=2026-09-01"
        );
        assert!(!prepared.url.contains("/trading/"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Authorization" && v == "ck"));
        assert!(!prepared
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Auth")));
        assert!(!prepared
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Sid")));
        let ported = HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://e22.kotaksecurities.com:443/trading".into(),
            hs_server_id: "server4".into(),
        };
        let sdk = prepare_kotak_market_data_get(
            "/market-data/1.0/historical/details?neosymbol=nse_cm%7C1333&interval=10min&fromdate=2026-08-20&todate=2026-09-01",
            &ported,
        )
        .unwrap();
        assert_eq!(
            sdk.url,
            "https://e22.kotaksecurities.com/market-data/1.0/historical/details?neosymbol=nse_cm%7C1333&interval=10min&fromdate=2026-08-20&todate=2026-09-01"
        );
        assert!(!sdk.url.contains(":443"));
        assert!(!sdk.url.contains("/trading/"));
    }

    #[test]
    fn kotak_file_paths_session_fallback_attaches_auth_sid_not_sid_query() {
        let sdk = prepare_kotak_file_paths_get(
            "/script-details/1.0/masterscrip/file-paths",
            &kotak_creds(),
        )
        .unwrap();
        let prepared = attach_kotak_file_paths_session(sdk, &kotak_creds()).unwrap();
        assert!(!prepared.url.contains("sId"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Auth" && v == "tt-token"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Sid" && v == "sid-1"));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "Authorization" && v == "ck"));
    }

    #[test]
    fn kotak_file_paths_get_strips_port_from_v2_base_url() {
        let creds = HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://e21.kotaksecurities.com:443/trading".into(),
            hs_server_id: "server4".into(),
        };
        let prepared =
            prepare_kotak_file_paths_get("/script-details/1.0/masterscrip/file-paths", &creds)
                .unwrap();
        assert_eq!(
            prepared.url,
            "https://e21.kotaksecurities.com/trading/script-details/1.0/masterscrip/file-paths"
        );
        assert!(!prepared.url.contains(":443"));
        assert!(!prepared.url.contains("sId"));
    }

    #[test]
    fn kotak_catalog_get_strips_port_from_v2_base_url() {
        let creds = HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://e21.kotaksecurities.com:443/trading".into(),
            hs_server_id: "server4".into(),
        };
        let prepared =
            prepare_kotak_catalog_get("/script-details/1.0/masterscrip/file-paths", &creds)
                .unwrap();
        assert_eq!(
            prepared.url,
            "https://e21.kotaksecurities.com/trading/script-details/1.0/masterscrip/file-paths"
        );
        assert!(!prepared.url.contains(":443"));
        assert!(!prepared.url.contains("sId"));
        assert_eq!(
            kotak_base_host("https://e21.kotaksecurities.com:443/trading").as_deref(),
            Some("e21.kotaksecurities.com")
        );
    }

    #[test]
    fn kotak_base_url_strips_port_and_accepts_v2_data_center() {
        let creds = HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://e21.kotaksecurities.com:443/".into(),
            hs_server_id: String::new(),
        };
        assert_eq!(
            effective_host("cis.kotaksecurities.com", &creds),
            "e21.kotaksecurities.com"
        );
    }

    #[test]
    fn classify_maps_rate_limit_unauthorized_and_kotak_session() {
        assert_eq!(classify_response(200, "[]"), None);
        assert_eq!(classify_response(429, ""), Some("rate_limited".into()));
        assert_eq!(classify_response(418, ""), Some("rate_limited".into()));
        assert_eq!(classify_response(401, ""), Some("unauthorized".into()));
        assert_eq!(classify_response(503, ""), Some("server_error".into()));
        assert_eq!(
            classify_response(200, r#"{"stat":"Not_Ok","stCode":1003}"#),
            Some("session_expired".into())
        );
        assert_eq!(
            classify_response(403, r#"{"status":"error","error_type":"TokenException"}"#),
            Some("session_expired".into())
        );
    }

    #[test]
    fn kite_session_attach_authorization_on_private_paths() {
        let creds = HostCredentialBlob::KiteSession {
            api_key: "kite_key".into(),
            access_token: "kite_tok".into(),
        };
        assert_eq!(
            effective_host("api.kite.trade", &creds),
            "api.kite.trade"
        );
        let prepared = prepare_request(
            "GET",
            "api.kite.trade",
            "/orders",
            &[],
            &[],
            None,
            &creds,
            0,
        );
        assert_eq!(prepared.url, "https://api.kite.trade/orders");
        assert!(prepared.headers.iter().any(|(n, v)| {
            n == "Authorization" && v == "token kite_key:kite_tok"
        }));
        assert!(prepared
            .headers
            .iter()
            .any(|(n, v)| n == "X-Kite-Version" && v == "3"));
    }

    #[test]
    fn response_headers_drop_set_cookie_and_keep_rate_limit_headers() {
        let redacted = redact_response_headers(&[
            ("Set-Cookie".into(), "session=abc".into()),
            ("X-MBX-USED-WEIGHT-1M".into(), "12".into()),
            ("Authorization".into(), "Bearer x".into()),
        ]);
        assert_eq!(
            redacted,
            vec![("x-mbx-used-weight-1m".to_string(), "12".to_string())]
        );
    }

    #[test]
    fn credential_blob_converts_from_keychain_shapes() {
        let hmac: HostCredentialBlob = (&CredentialBlob::hmac("k", "s")).into();
        assert_eq!(hmac, HostCredentialBlob::hmac("k", "s"));
        let kotak: HostCredentialBlob = (&CredentialBlob::KotakNeoTotpSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: "https://cis.kotaksecurities.com".into(),
            hs_server_id: "server4".into(),
            expires_at: None,
        })
            .into();
        assert!(kotak.secret_values().contains(&"tt"));
        assert!(kotak.api_key_for_tests().is_none());
    }

    #[test]
    fn network_error_strings_never_leak_the_signed_url() {
        let redacted = redact_url(
            "error sending request for url (https://api.binance.com/x?signature=deadbeef)",
        );
        assert!(!redacted.contains("signature"));
    }
}
