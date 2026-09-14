//! The one owned HTTP client, plus the single-flight that keeps ten identical
//! questions from becoming ten venue calls.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::clock::{Clock, SystemClock};
use super::engine::VenueEgress;
use super::types::{Decision, EgressRequest, Lane, Outcome, RefuseReason};
use crate::ubi::{redact_response_headers, PreparedHttpRequest};

/// Everything the engine needs to judge a call, plus what reqwest needs to send it.
#[derive(Debug, Clone)]
pub struct EgressCall {
    pub book_id: String,
    pub host: String,
    pub method: String,
    pub path: String,
    /// Without the leading `?`.
    pub query: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub lane: Lane,
    /// Serve a cached body younger than this instead of calling the venue.
    /// `0` disables coalescing for this call.
    pub max_age_ms: i64,
    pub timeout: Duration,
}

impl EgressCall {
    pub fn get(book_id: &str, host: &str, path: &str, lane: Lane) -> Self {
        Self {
            book_id: book_id.to_string(),
            host: host.to_string(),
            method: "GET".to_string(),
            path: path.to_string(),
            query: String::new(),
            headers: Vec::new(),
            body: None,
            lane,
            max_age_ms: 0,
            timeout: Duration::from_secs(15),
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into().trim_start_matches('?').to_string();
        self
    }

    /// Build the query from key/value pairs, percent-encoding every **value**.
    ///
    /// Prefer this to `with_query` + `format!`. `&` and `=` are structural: a value
    /// that carries one — a malformed symbol, a leftover id — would silently become
    /// an extra parameter on a venue call. Encoding the value keeps the call the
    /// question we meant to ask. Unreserved bytes pass through, so a real dated
    /// contract (`BTC-260925-100000-C`) is unchanged.
    pub fn with_query_pairs(mut self, pairs: &[(&str, &str)]) -> Self {
        self.query = pairs
            .iter()
            .map(|(key, value)| format!("{key}={}", encode_query_value(value)))
            .collect::<Vec<_>>()
            .join("&");
        self
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn with_max_age_ms(mut self, ms: i64) -> Self {
        self.max_age_ms = ms;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn url(&self) -> String {
        if self.query.is_empty() {
            format!("https://{}{}", self.host, self.path)
        } else {
            format!("https://{}{}?{}", self.host, self.path, self.query)
        }
    }

    /// Coalescing key. Query params are sorted so the same question asked with
    /// params in a different order is still one call.
    fn coalesce_key(&self) -> String {
        let mut pairs: Vec<&str> = self.query.split('&').filter(|s| !s.is_empty()).collect();
        pairs.sort_unstable();
        format!(
            "{}|{}|{}|{}|{}",
            self.book_id,
            self.method,
            self.host,
            self.path,
            pairs.join("&")
        )
    }
}

/// Percent-encode one query value against the RFC 3986 unreserved set.
///
/// Deliberately strict: anything that is not `A-Z a-z 0-9 - _ . ~` is escaped,
/// so no value can carry query structure into a URL.
fn encode_query_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct EgressResponse {
    pub status: u16,
    /// Already redacted to `ubi::http::RESPONSE_HEADER_ALLOWLIST`.
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// True when this body came from the coalescing cache rather than the venue.
    pub coalesced: bool,
}

impl EgressResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[derive(Debug, Clone)]
pub enum EgressError {
    /// The engine said no. Carries the venue's own clock where there is one.
    Refused(RefuseReason),
    /// The call went out and the network failed. Not a venue instruction.
    Transport(String),
}

impl EgressError {
    pub fn as_str(&self) -> &'static str {
        match self {
            EgressError::Refused(reason) => reason.as_str(),
            EgressError::Transport(_) => "network",
        }
    }

    /// Absolute epoch-ms the caller may look again, when the refusal came from a
    /// clock. `None` means waiting will not help.
    pub fn until_ms(&self) -> Option<i64> {
        match self {
            EgressError::Refused(reason) => reason.until_ms,
            EgressError::Transport(_) => None,
        }
    }

    /// True when the venue itself told us to stop, as opposed to a budget or a
    /// fence decision made locally.
    pub fn is_venue_stop(&self) -> bool {
        matches!(
            self,
            EgressError::Refused(RefuseReason {
                kind: super::types::RefuseKind::Banned | super::types::RefuseKind::Frozen,
                ..
            })
        )
    }
}

impl std::fmt::Display for EgressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EgressError::Refused(reason) => write!(f, "{reason}"),
            EgressError::Transport(msg) => write!(f, "network: {msg}"),
        }
    }
}

impl std::error::Error for EgressError {}

#[derive(Debug, Clone)]
struct CacheEntry {
    at_ms: i64,
    response: EgressResponse,
}

type Flight = Arc<tokio::sync::Mutex<()>>;

pub struct EgressTransport {
    engine: Arc<VenueEgress>,
    client: reqwest::Client,
    clock: Arc<dyn Clock>,
    cache: Mutex<HashMap<String, CacheEntry>>,
    flights: Mutex<HashMap<String, Flight>>,
}

impl EgressTransport {
    pub fn new_system() -> Self {
        Self::new(
            Arc::new(VenueEgress::with_system_clock()),
            Arc::new(SystemClock),
        )
    }

    pub fn new(engine: Arc<VenueEgress>, clock: Arc<dyn Clock>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self {
            engine,
            client,
            clock,
            cache: Mutex::new(HashMap::new()),
            flights: Mutex::new(HashMap::new()),
        }
    }

    pub fn engine(&self) -> &Arc<VenueEgress> {
        &self.engine
    }

    /// The one connection pool. `reqwest::Client` clones share their pool, so the
    /// blocking Wasm transport can take a clone and still be the same egress:
    /// same sockets, same keep-alive, same policy behind it.
    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    fn cached(&self, key: &str, max_age_ms: i64) -> Option<EgressResponse> {
        if max_age_ms <= 0 {
            return None;
        }
        let now = self.clock.now_ms();
        let cache = self.cache.lock().expect("egress cache lock");
        let entry = cache.get(key)?;
        if now.saturating_sub(entry.at_ms) <= max_age_ms {
            let mut hit = entry.response.clone();
            hit.coalesced = true;
            Some(hit)
        } else {
            None
        }
    }

    fn flight_for(&self, key: &str) -> Flight {
        let mut flights = self.flights.lock().expect("egress flights lock");
        flights
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Send a request whose URL and auth headers were already built by
    /// `ubi::http::prepare_*`. The engine still decides admission and still
    /// records the outcome — signing a request is not permission to send it.
    ///
    /// Never coalesced: signed requests carry a timestamp and a signature, so two
    /// of them are not the same request even when they ask the same question.
    pub async fn send_prepared(
        &self,
        book_id: &str,
        lane: Lane,
        prepared: &PreparedHttpRequest,
        timeout: Duration,
    ) -> Result<EgressResponse, EgressError> {
        let Some((host, path, query)) = split_url(&prepared.url) else {
            return Err(EgressError::Transport("unparseable url".to_string()));
        };
        let call = EgressCall {
            book_id: book_id.to_string(),
            host,
            method: prepared.method.clone(),
            path,
            query,
            headers: prepared.headers.clone(),
            body: prepared.body.clone(),
            lane,
            max_age_ms: 0,
            timeout,
        };
        self.send(&call).await
    }

    /// Send a connect-time auth call (credential check, TOTP login). Metered and
    /// ban-respecting like everything else, but admitted by host rather than by
    /// the R0 data fence — see `VenueEgress::admit_auth`.
    pub async fn send_auth(
        &self,
        call: &EgressCall,
        weight_hint: u32,
    ) -> Result<EgressResponse, EgressError> {
        let permit = match self.engine.admit_auth(&call.host, call.lane, weight_hint) {
            Decision::Admit(permit) => permit,
            Decision::Refuse(reason) => return Err(EgressError::Refused(reason)),
        };
        self.dispatch(call, permit).await
    }

    /// Admit, send, record. The only path to a venue.
    pub async fn send(&self, call: &EgressCall) -> Result<EgressResponse, EgressError> {
        let key = call.coalesce_key();

        if let Some(hit) = self.cached(&key, call.max_age_ms) {
            return Ok(hit);
        }

        // Single-flight: the second caller with the same question waits for the
        // first and then finds the answer in the cache rather than asking again.
        let flight = self.flight_for(&key);
        let _guard = flight.lock().await;
        if let Some(hit) = self.cached(&key, call.max_age_ms) {
            return Ok(hit);
        }

        let req = EgressRequest {
            book_id: &call.book_id,
            host: &call.host,
            method: &call.method,
            path: &call.path,
            query: &call.query,
            lane: call.lane,
            weight_hint: None,
        };
        let permit = match self.engine.admit(&req) {
            Decision::Admit(permit) => permit,
            Decision::Refuse(reason) => return Err(EgressError::Refused(reason)),
        };

        let out = self.dispatch(call, permit).await?;

        if call.max_age_ms > 0 && out.is_success() {
            let mut cache = self.cache.lock().expect("egress cache lock");
            cache.insert(
                key,
                CacheEntry {
                    at_ms: self.clock.now_ms(),
                    response: out.clone(),
                },
            );
        }

        Ok(out)
    }

    /// Everything after admission: send, then record whatever came back.
    async fn dispatch(
        &self,
        call: &EgressCall,
        permit: super::types::Permit,
    ) -> Result<EgressResponse, EgressError> {
        let method = match reqwest::Method::from_bytes(call.method.as_bytes()) {
            Ok(m) => m,
            Err(e) => {
                self.engine.record(permit, &Outcome::Transport);
                return Err(EgressError::Transport(format!("bad method: {e}")));
            }
        };
        let mut builder = self
            .client
            .request(method, call.url())
            .timeout(call.timeout);
        for (name, value) in &call.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = &call.body {
            builder = builder.body(body.clone());
        }

        let sent = builder.send().await;
        let response = match sent {
            Ok(r) => r,
            Err(e) => {
                self.engine.record(permit, &Outcome::Transport);
                return Err(EgressError::Transport(sanitize(&e.to_string())));
            }
        };

        let status = response.status().as_u16();
        let raw_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(n, v)| {
                (
                    n.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let headers = redact_response_headers(&raw_headers);
        let body = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                // Status and headers are already known — the venue did answer, so
                // this is still an HTTP outcome and a 429/418 must still register.
                self.engine
                    .record(permit, &Outcome::http(status, headers.clone(), ""));
                return Err(EgressError::Transport(sanitize(&e.to_string())));
            }
        };

        self.engine
            .record(permit, &Outcome::http(status, headers.clone(), &body));

        Ok(EgressResponse {
            status,
            headers,
            body,
            coalesced: false,
        })
    }
}

/// Split `https://host/path?query` into its egress-relevant parts.
pub fn split_url(url: &str) -> Option<(String, String, String)> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let (authority, tail) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let host = authority
        .split('@')
        .next_back()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or(authority)
        .to_ascii_lowercase();
    let (path, query) = match tail.find('?') {
        Some(i) => (&tail[..i], &tail[i + 1..]),
        None => (tail, ""),
    };
    Some((host, path.to_string(), query.to_string()))
}

/// reqwest errors can carry the full URL, and Kotak URLs carry session material.
fn sanitize(msg: &str) -> String {
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(idx) = rest.find("https://") {
        out.push_str(&rest[..idx]);
        out.push_str("[url]");
        let tail = &rest[idx..];
        let end = tail.find(|c: char| c.is_whitespace()).unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_key_is_param_order_independent() {
        let a = EgressCall::get(
            "binance-com-spot",
            "api.binance.com",
            "/api/v3/depth",
            Lane::MarketData,
        )
        .with_query("symbol=BTCUSDT&limit=5000");
        let b = EgressCall::get(
            "binance-com-spot",
            "api.binance.com",
            "/api/v3/depth",
            Lane::MarketData,
        )
        .with_query("limit=5000&symbol=BTCUSDT");
        assert_eq!(a.coalesce_key(), b.coalesce_key());
    }

    #[test]
    fn coalesce_key_separates_books_and_paths() {
        let a = EgressCall::get(
            "binance-com-spot",
            "api.binance.com",
            "/api/v3/depth",
            Lane::MarketData,
        );
        let b = EgressCall::get(
            "binance-com-spot",
            "api.binance.com",
            "/api/v3/trades",
            Lane::MarketData,
        );
        let c = EgressCall::get(
            "kotak-nse-nfo",
            "api.binance.com",
            "/api/v3/depth",
            Lane::MarketData,
        );
        assert_ne!(a.coalesce_key(), b.coalesce_key());
        assert_ne!(a.coalesce_key(), c.coalesce_key());
    }

    #[test]
    fn url_builds_with_and_without_query() {
        let bare = EgressCall::get(
            "binance-com-spot",
            "api.binance.com",
            "/api/v3/time",
            Lane::MarketData,
        );
        assert_eq!(bare.url(), "https://api.binance.com/api/v3/time");
        let q = bare.clone().with_query("?symbol=BTCUSDT");
        assert_eq!(
            q.url(),
            "https://api.binance.com/api/v3/time?symbol=BTCUSDT"
        );
    }

    #[test]
    fn split_url_finds_host_path_and_query() {
        let (h, p, q) =
            split_url("https://api.binance.com/api/v3/depth?symbol=BTC&limit=5").unwrap();
        assert_eq!(h, "api.binance.com");
        assert_eq!(p, "/api/v3/depth");
        assert_eq!(q, "symbol=BTC&limit=5");

        let (h, p, q) = split_url("https://lapi.kotaksecurities.com/a/b/nse_cm.csv").unwrap();
        assert_eq!(h, "lapi.kotaksecurities.com");
        assert_eq!(p, "/a/b/nse_cm.csv");
        assert_eq!(q, "");

        assert_eq!(split_url("api.binance.com/x"), None);
    }

    #[test]
    fn split_url_normalises_case_and_strips_port() {
        let (h, _, _) = split_url("https://API.Binance.COM:443/api/v3/time").unwrap();
        assert_eq!(h, "api.binance.com");
    }

    #[test]
    fn a_query_value_may_never_inject_a_second_parameter() {
        // `&` and `=` are structural. A symbol carrying them would silently become
        // extra parameters on a venue call, so they are escaped, not trusted.
        let call = EgressCall::get(
            "binance-com-options",
            "eapi.binance.com",
            "/eapi/v1/mark",
            Lane::MarketData,
        )
        .with_query_pairs(&[("symbol", "BTC-260925-100000&evil=1-C")]);
        let url = call.url();
        assert!(
            url.ends_with("?symbol=BTC-260925-100000%26evil%3D1-C"),
            "separators must be escaped: {url}"
        );
        assert_eq!(url.matches('&').count(), 0, "no injected parameter: {url}");
        assert_eq!(
            url.matches("evil=").count(),
            0,
            "no injected parameter: {url}"
        );
    }

    #[test]
    fn a_real_dated_contract_survives_encoding_byte_for_byte() {
        // Dashes are unreserved, so the live symbol shape is unchanged — encoding
        // must not mangle the mixed-case contract the venue matches on.
        let call = EgressCall::get(
            "binance-com-options",
            "eapi.binance.com",
            "/eapi/v1/mark",
            Lane::MarketData,
        )
        .with_query_pairs(&[("symbol", "BTC-260925-100000-C")]);
        assert!(
            call.url().ends_with("?symbol=BTC-260925-100000-C"),
            "{}",
            call.url()
        );
    }

    #[test]
    fn multiple_pairs_keep_the_ampersand_as_a_separator_only() {
        let call = EgressCall::get(
            "binance-com-options",
            "eapi.binance.com",
            "/eapi/v1/openInterest",
            Lane::MarketData,
        )
        .with_query_pairs(&[("underlyingAsset", "BTC"), ("expiration", "260925")]);
        assert!(
            call.url()
                .ends_with("?underlyingAsset=BTC&expiration=260925"),
            "{}",
            call.url()
        );
    }

    #[test]
    fn sanitize_strips_urls_that_could_carry_session_material() {
        let msg = "error sending request for url https://gw-napi.kotaksecurities.com/x?Auth=secret trailing";
        let out = sanitize(msg);
        assert!(!out.contains("secret"), "{out}");
        assert!(!out.contains("kotaksecurities"), "{out}");
        assert!(out.contains("[url]"));
        assert!(out.contains("trailing"));
    }
}
