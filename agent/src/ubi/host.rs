//! Wasmtime host for `tradeautopsy:ubi-data/broker-adapter-data` (v0.2.0).
//!
//! `agent/wit/ubi.wit` v0.1.0 `fetch_fills` world is unchanged and is not merged here.
//! Phase 1 fixture mode + Phase 3 live mode: the host attaches auth from the vault
//! credential blob inside `broker_http_call`, so no secret ever enters component memory.

use crate::ubi::http::{
    classify_response, effective_host, prepare_request, redact_response_headers,
    BrokerHttpTransport,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

pub use crate::ubi::http::HostCredentialBlob;

wasmtime::component::bindgen!({
    path: "../docs/contracts",
    world: "broker-adapter-data",
});

pub use tradeautopsy::ubi_data::types::{
    BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpHeader,
};

/// Headers a component must never set; host strips or rejects (R6).
pub const FORBIDDEN_COMPONENT_HEADERS: &[&str] = &["authorization", "x-mbx-apikey", "auth", "sid"];

/// Fixture response keyed by path (contract tests — no live HTTP).
#[derive(Debug, Clone)]
pub struct BrokerHttpFixture {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub error_class: Option<String>,
}

/// Where `broker_http_call` gets its response from.
#[derive(Clone)]
pub enum BrokerHttpMode {
    /// path → canned response (contract/integration tests).
    Fixtures(HashMap<String, BrokerHttpFixture>),
    /// Real network egress with host-attached auth.
    Live(Arc<dyn BrokerHttpTransport>),
}

#[derive(Debug, Clone)]
pub struct UbiHostConfig {
    pub connection_id: String,
    pub broker_slug: String,
    /// This connection's shipping book. Fence source for `broker_http_call` — not a catalog slug lookup.
    pub book_id: String,
    pub asset_class: String,
    /// Host-only. Must never appear in `broker_http_call` responses returned to the component.
    pub credentials: HostCredentialBlob,
}

#[derive(Debug)]
pub enum UbiHostError {
    Wasmtime(wasmtime::Error),
    Adapter(String),
    Io(std::io::Error),
}

impl std::fmt::Display for UbiHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wasmtime(e) => write!(f, "wasmtime: {e}"),
            Self::Adapter(e) => write!(f, "adapter: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for UbiHostError {}

impl From<wasmtime::Error> for UbiHostError {
    fn from(value: wasmtime::Error) -> Self {
        Self::Wasmtime(value)
    }
}

impl From<std::io::Error> for UbiHostError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub struct UbiHostState {
    pub config: UbiHostConfig,
    /// Fixture map or live transport (host must still be allowlisted either way).
    pub mode: BrokerHttpMode,
    /// Paths the component requested (for contract assertions).
    pub calls: Vec<BrokerHttpRequest>,
    /// True if any credential substring was about to be returned to the component.
    pub credential_leak_attempted: bool,
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for UbiHostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl UbiHostState {
    pub fn new(config: UbiHostConfig, fixtures: HashMap<String, BrokerHttpFixture>) -> Self {
        Self::with_mode(config, BrokerHttpMode::Fixtures(fixtures))
    }

    pub fn live(config: UbiHostConfig, transport: Arc<dyn BrokerHttpTransport>) -> Self {
        Self::with_mode(config, BrokerHttpMode::Live(transport))
    }

    pub fn with_mode(config: UbiHostConfig, mode: BrokerHttpMode) -> Self {
        Self {
            config,
            mode,
            calls: Vec::new(),
            credential_leak_attempted: false,
            ctx: WasiCtx::builder().build(),
            table: ResourceTable::new(),
        }
    }

    fn response_contains_secret(&self, body: &str, headers: &[(String, String)]) -> bool {
        self.config
            .credentials
            .secret_values()
            .into_iter()
            .filter(|s| !s.is_empty())
            .any(|s| body.contains(s) || headers.iter().any(|(_, v)| v.contains(s)))
    }
}

impl tradeautopsy::ubi_data::types::Host for UbiHostState {}

impl tradeautopsy::ubi_data::broker_http::Host for UbiHostState {
    fn broker_http_call(
        &mut self,
        request: BrokerHttpRequest,
    ) -> Result<BrokerHttpResponse, String> {
        // Strip / reject forbidden auth headers from the component (R6).
        for h in &request.headers {
            if FORBIDDEN_COMPONENT_HEADERS
                .iter()
                .any(|f| h.name.eq_ignore_ascii_case(f))
            {
                return Err(format!(
                    "forbidden header from component: {} (host attaches auth)",
                    h.name
                ));
            }
        }

        // Kotak's real host comes from the credential blob, not the component (R6 §3.4).
        let target_host = effective_host(&request.host, &self.config.credentials);
        // Fence this connection's book, not the catalog slug's shipping book.
        let book_id = self.config.book_id.trim();
        let (_capability_id, auth_mode) = match crate::data::authorize_book_call(
            book_id,
            &target_host,
            &request.method,
            &request.path,
            false,
        ) {
            Ok(pair) => pair,
            Err(refuse) => {
                return Ok(BrokerHttpResponse {
                    status: 0,
                    headers: vec![],
                    body: String::new(),
                    error_class: Some(refuse.as_str().to_string()),
                });
            }
        };
        let attach_private = auth_mode == crate::data::AuthMode::PrivateRead;

        self.calls.push(request.clone());

        let (status, headers, body, error_class) = match &self.mode {
            BrokerHttpMode::Fixtures(fixtures) => {
                let fixture = fixtures.get(&request.path).ok_or_else(|| {
                    format!(
                        "no fixture for path {} on host {}",
                        request.path, request.host
                    )
                })?;
                let mut headers = redact_response_headers(&fixture.headers);
                if attach_private {
                    headers.push(("x-ubi-auth-attached".to_string(), "true".to_string()));
                }
                (
                    fixture.status,
                    headers,
                    fixture.body.clone(),
                    fixture
                        .error_class
                        .clone()
                        .or_else(|| classify_response(fixture.status, &fixture.body)),
                )
            }
            BrokerHttpMode::Live(transport) => {
                let query: Vec<(String, String)> = request
                    .query
                    .iter()
                    .map(|q| (q.name.clone(), q.value.clone()))
                    .collect();
                let headers: Vec<(String, String)> = request
                    .headers
                    .iter()
                    .map(|h| (h.name.clone(), h.value.clone()))
                    .collect();
                let prepared = if attach_private {
                    prepare_request(
                        &request.method,
                        &target_host,
                        &request.path,
                        &query,
                        &headers,
                        request.body.as_deref(),
                        &self.config.credentials,
                        chrono::Utc::now().timestamp_millis(),
                    )
                } else {
                    crate::ubi::http::prepare_unsigned_request(
                        &request.method,
                        &target_host,
                        &request.path,
                        &query,
                        &headers,
                        request.body.as_deref(),
                    )
                };
                match transport.send(&prepared) {
                    Ok(response) => {
                        let error_class = classify_response(response.status, &response.body);
                        (
                            response.status,
                            redact_response_headers(&response.headers),
                            response.body,
                            error_class,
                        )
                    }
                    // Transport errors carry no body the component can trust.
                    Err(_) => (0, vec![], String::new(), Some("network".to_string())),
                }
            }
        };

        if self.response_contains_secret(&body, &headers) {
            self.credential_leak_attempted = true;
            return Err("host refused to return response containing raw credentials".into());
        }

        Ok(BrokerHttpResponse {
            status,
            headers: headers
                .into_iter()
                .map(|(name, value)| HttpHeader { name, value })
                .collect(),
            body,
            error_class,
        })
    }
}

/// Load a fixture Wasm component, call `fetch-fills`, inject identity fields (R5 §3.5).
pub fn run_fetch_fills(
    component_path: &Path,
    state: UbiHostState,
    cursor: FillCursor,
) -> Result<(Vec<FillEvent>, UbiHostState), UbiHostError> {
    let (mut store, bindings) = instantiate_adapter(component_path, state)?;

    let result = bindings
        .tradeautopsy_ubi_data_adapter()
        .call_fetch_fills(&mut store, &cursor)?
        .map_err(UbiHostError::Adapter)?;

    let connection_id = store.data().config.connection_id.clone();
    let broker_slug = store.data().config.broker_slug.clone();
    let asset_class = store.data().config.asset_class.clone();

    // Host injects / overwrites identity fields so a component cannot spoof another connection.
    let fills: Vec<FillEvent> = result
        .into_iter()
        .map(|mut f| {
            f.connection_id = connection_id.clone();
            f.broker_slug = broker_slug.clone();
            f.asset_class = asset_class.clone();
            f
        })
        .collect();

    let state = store.into_data();
    Ok((fills, state))
}

/// Call Wasm `describe`. Station loopback describe/obtain stays native (`enrich_obtain`).
pub fn run_describe(
    component_path: &Path,
    state: UbiHostState,
) -> Result<(String, UbiHostState), UbiHostError> {
    let (mut store, bindings) = instantiate_adapter(component_path, state)?;
    let result = bindings
        .tradeautopsy_ubi_data_data_adapter()
        .call_describe(&mut store)?
        .map_err(UbiHostError::Adapter)?;
    let state = store.into_data();
    Ok((result, state))
}

/// Call Wasm `obtain`. Does not replace native Station `GET /api/station/history`.
pub fn run_obtain(
    component_path: &Path,
    state: UbiHostState,
    request_json: &str,
) -> Result<(String, UbiHostState), UbiHostError> {
    let (mut store, bindings) = instantiate_adapter(component_path, state)?;
    let result = bindings
        .tradeautopsy_ubi_data_data_adapter()
        .call_obtain(&mut store, request_json)?
        .map_err(UbiHostError::Adapter)?;
    let state = store.into_data();
    Ok((result, state))
}

fn instantiate_adapter(
    component_path: &Path,
    state: UbiHostState,
) -> Result<(Store<UbiHostState>, BrokerAdapterData), UbiHostError> {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, component_path)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    BrokerAdapterData::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut store = Store::new(&engine, state);
    let bindings = BrokerAdapterData::instantiate(&mut store, &component, &linker)?;
    Ok((store, bindings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ubi::allowlist::host_allowed;
    use crate::ubi::http::{RecordingTransport, TransportResponse};
    use std::collections::HashMap;

    #[test]
    fn forbidden_headers_constant_covers_r6() {
        assert!(FORBIDDEN_COMPONENT_HEADERS.contains(&"x-mbx-apikey"));
        assert!(FORBIDDEN_COMPONENT_HEADERS.contains(&"auth"));
        assert!(FORBIDDEN_COMPONENT_HEADERS.contains(&"sid"));
    }

    #[test]
    fn allowlist_blocks_unknown_before_fixture_lookup() {
        assert!(!host_allowed("attacker.test"));
    }

    #[test]
    fn broker_http_call_returns_host_blocked_for_unknown_host() {
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("k", "s"),
            },
            HashMap::new(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "evil.example.com".into(),
                path: "/x".into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("Ok response with error_class");
        assert_eq!(resp.error_class.as_deref(), Some("host_blocked"));
        assert!(state.calls.is_empty());
    }

    #[test]
    fn broker_http_call_rejects_forbidden_component_headers() {
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("k", "s"),
            },
            HashMap::new(),
        );
        let err = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/myTrades".into(),
                query: vec![],
                headers: vec![HttpHeader {
                    name: "X-MBX-APIKEY".into(),
                    value: "k".into(),
                }],
                body: None,
            },
        )
        .expect_err("forbidden header");
        assert!(err.contains("forbidden header"));
    }

    #[test]
    fn broker_http_call_never_echoes_raw_credentials() {
        let secret = "SUPER_SECRET_VALUE_XYZ";
        let mut fixtures = HashMap::new();
        fixtures.insert(
            "/api/v3/myTrades".into(),
            BrokerHttpFixture {
                status: 200,
                body: "[]".into(),
                headers: vec![],
                error_class: None,
            },
        );
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("KEY", secret),
            },
            fixtures,
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/myTrades".into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("fixture ok");
        assert!(!resp.body.contains(secret));
        assert!(!resp.headers.iter().any(|h| h.value.contains(secret)));
        assert!(!state.credential_leak_attempted);
    }

    #[test]
    fn live_mode_attaches_hmac_auth_outside_the_component() {
        let transport = Arc::new(RecordingTransport::ok(200, "[]"));
        let mut state = UbiHostState::live(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("LIVE_KEY", "LIVE_SECRET"),
            },
            transport.clone(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/myTrades".into(),
                query: vec![
                    crate::ubi::host::tradeautopsy::ubi_data::types::HttpQueryParam {
                        name: "symbol".into(),
                        value: "BTCUSDT".into(),
                    },
                ],
                headers: vec![],
                body: None,
            },
        )
        .expect("live ok");

        let sent = transport.last().expect("request sent");
        assert!(sent
            .url
            .starts_with("https://api.binance.com/api/v3/myTrades?symbol=BTCUSDT"));
        assert!(sent.url.contains("&signature="));
        assert!(sent
            .headers
            .iter()
            .any(|(n, v)| n == "X-MBX-APIKEY" && v == "LIVE_KEY"));
        assert_eq!(resp.status, 200);
        assert!(!resp.headers.iter().any(|h| h.value.contains("LIVE_SECRET")));
    }

    #[test]
    fn live_mode_classifies_rate_limit_and_drops_set_cookie() {
        let transport = Arc::new(RecordingTransport::new(Ok(TransportResponse {
            status: 429,
            headers: vec![
                ("set-cookie".into(), "sid=leak".into()),
                ("x-mbx-used-weight-1m".into(), "6100".into()),
            ],
            body: "{\"code\":-1003}".into(),
        })));
        let mut state = UbiHostState::live(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("k", "s"),
            },
            transport,
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/myTrades".into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("live ok");
        assert_eq!(resp.error_class.as_deref(), Some("rate_limited"));
        assert!(!resp.headers.iter().any(|h| h.name == "set-cookie"));
        assert!(resp
            .headers
            .iter()
            .any(|h| h.name == "x-mbx-used-weight-1m"));
    }

    #[test]
    fn live_mode_network_failure_is_classified_not_leaked() {
        let transport = Arc::new(RecordingTransport::new(Err(
            "network: <redacted-url>".to_string()
        )));
        let mut state = UbiHostState::live(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "kotak_neo".into(),
                book_id: crate::data::KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
                asset_class: "equities".into(),
                credentials: HostCredentialBlob::KotakSession {
                    consumer_key: "ck".into(),
                    trade_token: "tt".into(),
                    sid: "sid".into(),
                    base_url: "https://cis.kotaksecurities.com".into(),
                    hs_server_id: "server4".into(),
                },
            },
            transport,
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "cis.kotaksecurities.com".into(),
                path: "/quick/user/trades".into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("live ok");
        assert_eq!(resp.error_class.as_deref(), Some("network"));
        assert_eq!(resp.status, 0);
        assert!(resp.body.is_empty());
    }

    #[test]
    fn post_order_is_mutation_forbidden_before_transport() {
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("k", "s"),
            },
            HashMap::new(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "POST".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/order".into(),
                query: vec![],
                headers: vec![],
                body: Some("{\"symbol\":\"BTCUSDT\"}".into()),
            },
        )
        .expect("Ok response with error_class");
        assert_eq!(resp.error_class.as_deref(), Some("mutation_forbidden"));
        assert!(state.calls.is_empty());
    }

    #[test]
    fn public_ticker_does_not_attach_hmac() {
        let transport = Arc::new(RecordingTransport::ok(
            200,
            r#"{"symbol":"BTCUSDT","price":"1"}"#,
        ));
        let mut state = UbiHostState::live(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("LIVE_KEY", "LIVE_SECRET"),
            },
            transport.clone(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/ticker/price".into(),
                query: vec![
                    crate::ubi::host::tradeautopsy::ubi_data::types::HttpQueryParam {
                        name: "symbol".into(),
                        value: "BTCUSDT".into(),
                    },
                ],
                headers: vec![],
                body: None,
            },
        )
        .expect("live ok");
        let sent = transport.last().expect("request sent");
        assert_eq!(
            sent.url,
            "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT"
        );
        assert!(!sent.url.contains("signature="));
        assert!(!sent
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("X-MBX-APIKEY")));
        assert_eq!(resp.status, 200);
    }

    fn ticker_price_request() -> BrokerHttpRequest {
        BrokerHttpRequest {
            method: "GET".into(),
            host: "api.binance.com".into(),
            path: "/api/v3/ticker/price".into(),
            query: vec![],
            headers: vec![],
            body: None,
        }
    }

    fn ticker_price_fixtures() -> HashMap<String, BrokerHttpFixture> {
        let mut fixtures = HashMap::new();
        fixtures.insert(
            "/api/v3/ticker/price".into(),
            BrokerHttpFixture {
                status: 200,
                body: r#"{"symbol":"BTCUSDT","price":"1"}"#.into(),
                headers: vec![],
                error_class: None,
            },
        );
        fixtures
    }

    #[test]
    fn broker_http_call_empty_book_id_fails_closed_without_slug_lookup() {
        // Catalog would map binance_com → binance-com-spot; empty config.book_id must still refuse.
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: String::new(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("k", "s"),
            },
            ticker_price_fixtures(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            ticker_price_request(),
        )
        .expect("Ok response with error_class");
        assert_eq!(resp.error_class.as_deref(), Some("path_not_allowlisted"));
        assert!(state.calls.is_empty());
    }

    #[test]
    fn broker_http_call_fences_from_config_book_id_not_catalog_slug() {
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "not_a_catalog_slug".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("TEST_KEY", "TEST_SECRET"),
            },
            ticker_price_fixtures(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            ticker_price_request(),
        )
        .expect("fixture ok");
        assert_ne!(resp.error_class.as_deref(), Some("host_blocked"));
        assert_ne!(resp.error_class.as_deref(), Some("path_not_allowlisted"));
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn broker_http_call_spot_book_refuses_eapi() {
        let mut state = UbiHostState::new(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob::hmac("k", "s"),
            },
            HashMap::new(),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "eapi.binance.com".into(),
                path: "/eapi/v1/ticker".into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("Ok response with error_class");
        assert_eq!(resp.error_class.as_deref(), Some("host_blocked"));
        assert!(state.calls.is_empty());
    }

    fn kotak_session_blob(base_url: &str) -> HostCredentialBlob {
        HostCredentialBlob::KotakSession {
            consumer_key: "ck".into(),
            trade_token: "tt".into(),
            sid: "sid".into(),
            base_url: base_url.into(),
            hs_server_id: "server4".into(),
        }
    }

    fn kotak_host_config(book_id: &str, base_url: &str) -> UbiHostConfig {
        UbiHostConfig {
            connection_id: "c".into(),
            broker_slug: "kotak_neo".into(),
            book_id: book_id.into(),
            asset_class: "equities".into(),
            credentials: kotak_session_blob(base_url),
        }
    }

    const CASH_CSV: &str = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv";
    const FO_CSV: &str = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv";
    const CASH_QUOTE: &str = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp";
    const NFO_QUOTE: &str = "/script-details/1.0/quotes/neosymbol/nse_fo%7C12345/ltp";

    fn csv_fixture(path: &str) -> HashMap<String, BrokerHttpFixture> {
        let mut fixtures = HashMap::new();
        fixtures.insert(
            path.into(),
            BrokerHttpFixture {
                status: 200,
                body: "ok".into(),
                headers: vec![],
                error_class: None,
            },
        );
        fixtures
    }

    #[test]
    fn nfo_book_host_refuses_cash_csv_spot_ticker_and_cash_quotes() {
        let mut state = UbiHostState::new(
            kotak_host_config(
                crate::data::KOTAK_NSE_NFO_BOOK_ID,
                "https://cis.kotaksecurities.com",
            ),
            csv_fixture(CASH_CSV),
        );
        let cash_csv = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "lapi.kotaksecurities.com".into(),
                path: CASH_CSV.into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("Ok response with error_class");
        assert_eq!(
            cash_csv.error_class.as_deref(),
            Some("path_not_allowlisted")
        );
        assert!(state.calls.is_empty());

        let ticker = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "api.binance.com".into(),
                path: "/api/v3/ticker/price".into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("Ok response with error_class");
        assert!(
            matches!(
                ticker.error_class.as_deref(),
                Some("host_blocked") | Some("path_not_allowlisted")
            ),
            "{:?}",
            ticker.error_class
        );

        let cash_quote = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "gw-napi.kotaksecurities.com".into(),
                path: CASH_QUOTE.into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("Ok response with error_class");
        assert_eq!(
            cash_quote.error_class.as_deref(),
            Some("path_not_allowlisted")
        );
    }

    #[test]
    fn nfo_book_host_allows_fo_csv_and_nfo_quotes_fixture() {
        let mut fo_state = UbiHostState::new(
            kotak_host_config(
                crate::data::KOTAK_NSE_NFO_BOOK_ID,
                "https://lapi.kotaksecurities.com",
            ),
            csv_fixture(FO_CSV),
        );
        let fo = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut fo_state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "lapi.kotaksecurities.com".into(),
                path: FO_CSV.into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("FO CSV fixture");
        assert_ne!(fo.error_class.as_deref(), Some("path_not_allowlisted"));
        assert_ne!(fo.error_class.as_deref(), Some("host_blocked"));
        assert_eq!(fo.status, 200);

        let mut quote_fixtures = HashMap::new();
        quote_fixtures.insert(
            NFO_QUOTE.into(),
            BrokerHttpFixture {
                status: 200,
                body: r#"{"ltp":"1"}"#.into(),
                headers: vec![],
                error_class: None,
            },
        );
        let mut quote_state = UbiHostState::new(
            kotak_host_config(
                crate::data::KOTAK_NSE_NFO_BOOK_ID,
                "https://gw-napi.kotaksecurities.com",
            ),
            quote_fixtures,
        );
        let quote = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut quote_state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "gw-napi.kotaksecurities.com".into(),
                path: NFO_QUOTE.into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("NFO quotes fixture");
        assert_ne!(quote.error_class.as_deref(), Some("path_not_allowlisted"));
        assert_eq!(quote.status, 200);
    }

    #[test]
    fn cash_book_host_refuses_fo_csv() {
        let mut state = UbiHostState::new(
            kotak_host_config(
                crate::data::KOTAK_NSE_BSE_CASH_BOOK_ID,
                "https://lapi.kotaksecurities.com",
            ),
            csv_fixture(FO_CSV),
        );
        let resp = tradeautopsy::ubi_data::broker_http::Host::broker_http_call(
            &mut state,
            BrokerHttpRequest {
                method: "GET".into(),
                host: "lapi.kotaksecurities.com".into(),
                path: FO_CSV.into(),
                query: vec![],
                headers: vec![],
                body: None,
            },
        )
        .expect("Ok response with error_class");
        assert_eq!(resp.error_class.as_deref(), Some("path_not_allowlisted"));
        assert!(state.calls.is_empty());
    }
}
