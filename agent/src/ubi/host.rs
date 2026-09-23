//! Wasmtime host for `tradeautopsy:ubi-data/broker-adapter-data` (v0.3.0, ADR 0004).
//!
//! `agent/wit/ubi.wit` v0.1.0 `fetch_fills` world is unchanged and is not merged here.
//! Phase 1 fixture mode + Phase 3 live mode: the host attaches auth from the vault
//! credential blob inside `broker_http_call`, so no secret ever enters component memory.
//!
//! Taxonomy (ADR 0004): the host stamps `asset-class`, `instrument-class`, and
//! `is-inverse` on every fill from the connection's shipping **book** (`UbiHostConfig`
//! axes, resolved via `descriptor_for_book_id`). The adapter never classifies.
//! Pre-bump (0.2.0) components fail instantiation and are rejected loudly — there is
//! no silent legacy-string mapping.

use crate::ubi::http::{
    classify_response, effective_host, prepare_request, redact_response_headers,
    BrokerHttpTransport,
};
use super::catalog::{AssetClass, InstrumentClass};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

pub use crate::ubi::http::HostCredentialBlob;

/// Live UBI data contract version (ADR 0004). Components built against any older
/// package (notably `tradeautopsy:ubi-data@0.2.0`, where `asset-class` was a free
/// string) fail host instantiation and are rejected, never silently mapped.
pub const UBI_DATA_CONTRACT_VERSION: &str = "0.3.0";

wasmtime::component::bindgen!({
    path: "../docs/contracts",
    world: "broker-adapter-data",
});

pub use tradeautopsy::ubi_data::types::{
    BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpHeader,
};
/// WIT-side axis enums (bindgen of `docs/contracts/ubi-data.wit`). The catalog
/// [`AssetClass`]/[`InstrumentClass`] are the host-canonical taxonomy; the `From`
/// impls below convert catalog → WIT at the stamp boundary. Two types with one
/// mapping table, kept total by exhaustive matches.
pub use tradeautopsy::ubi_data::types::{
    AssetClass as WitAssetClass, InstrumentClass as WitInstrumentClass,
};

impl From<AssetClass> for WitAssetClass {
    fn from(value: AssetClass) -> Self {
        match value {
            AssetClass::Fx => Self::Fx,
            AssetClass::Equity => Self::Equity,
            AssetClass::Commodity => Self::Commodity,
            AssetClass::Debt => Self::Debt,
            AssetClass::Index => Self::Index,
            AssetClass::Cryptocurrency => Self::Cryptocurrency,
            AssetClass::Alternative => Self::Alternative,
        }
    }
}

impl From<InstrumentClass> for WitInstrumentClass {
    fn from(value: InstrumentClass) -> Self {
        match value {
            InstrumentClass::Spot => Self::Spot,
            InstrumentClass::Swap => Self::Swap,
            InstrumentClass::Future => Self::Future,
            InstrumentClass::FuturesSpread => Self::FuturesSpread,
            InstrumentClass::Forward => Self::Forward,
            InstrumentClass::Cfd => Self::Cfd,
            InstrumentClass::Bond => Self::Bond,
            InstrumentClass::Option => Self::Option,
            InstrumentClass::OptionSpread => Self::OptionSpread,
            InstrumentClass::Warrant => Self::Warrant,
            InstrumentClass::SportsBetting => Self::SportsBetting,
            InstrumentClass::BinaryOption => Self::BinaryOption,
        }
    }
}

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
    /// Book's asset axis (ADR 0004). Stamped onto every fill; adapter output ignored.
    pub asset_class: AssetClass,
    /// Book's instrument axis (ADR 0004). Stamped onto every fill; adapter output ignored.
    pub instrument_class: InstrumentClass,
    /// Coin-M margining (ADR 0004). Stamped onto every fill; adapter output ignored.
    pub is_inverse: bool,
    /// Host-only. Must never appear in `broker_http_call` responses returned to the component.
    pub credentials: HostCredentialBlob,
}

#[derive(Debug)]
pub enum UbiHostError {
    Wasmtime(wasmtime::Error),
    Adapter(String),
    Io(std::io::Error),
    /// The component bytes are not a loadable `UBI_DATA_CONTRACT_ID` component —
    /// in particular any pre-bump 0.2.0 build (string `asset-class`, no
    /// `instrument-class`/`is-inverse`). Loud by design: no silent mapping.
    ContractMismatch(String),
}

impl UbiHostError {
    /// Loud pre-bump rejection. Names the expected contract so a 0.2.0-vs-0.3.0
    /// mismatch is diagnosable from the message alone.
    pub fn contract_mismatch(detail: impl Into<String>) -> Self {
        Self::ContractMismatch(format!(
            "component rejected: expected tradeautopsy:ubi-data@{} but the bytes failed to load ({}) — \
             pre-bump (0.2.0) components are rejected, never silently mapped; \
             rebuild the adapter against docs/contracts/ubi-data.wit",
            UBI_DATA_CONTRACT_VERSION,
            detail.into()
        ))
    }
}

impl std::fmt::Display for UbiHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wasmtime(e) => write!(f, "wasmtime: {e}"),
            Self::Adapter(e) => write!(f, "adapter: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::ContractMismatch(e) => write!(f, "contract mismatch: {e}"),
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
    /// Budgets, freezes, and bans for this connection's venue. Injectable so a
    /// test that exercises a 429 does not freeze the venue for the whole process.
    egress: Arc<crate::egress::VenueEgress>,
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
        Self::with_mode_and_egress(config, mode, crate::egress::shared_engine())
    }

    pub fn with_mode_and_egress(
        config: UbiHostConfig,
        mode: BrokerHttpMode,
        egress: Arc<crate::egress::VenueEgress>,
    ) -> Self {
        Self {
            config,
            mode,
            calls: Vec::new(),
            credential_leak_attempted: false,
            egress,
            ctx: WasiCtx::builder().build(),
            table: ResourceTable::new(),
        }
    }

    /// An isolated engine, for tests that must drive a 429 or a 418 without
    /// freezing the venue for every other test in the process.
    #[cfg(test)]
    fn live_isolated(config: UbiHostConfig, transport: Arc<dyn BrokerHttpTransport>) -> Self {
        Self::with_mode_and_egress(
            config,
            BrokerHttpMode::Live(transport),
            Arc::new(crate::egress::VenueEgress::with_system_clock()),
        )
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
                // Admission. The fence already passed above; this is the
                // budget, the freeze, and the ban — the parts that need a clock.
                let query_string = query
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&");
                let lane = if attach_private {
                    crate::egress::Lane::PrivateRead
                } else {
                    crate::egress::Lane::MarketData
                };
                let engine = self.egress.clone();
                let egress_req = crate::egress::EgressRequest {
                    book_id,
                    host: &target_host,
                    method: &request.method,
                    path: &request.path,
                    query: &query_string,
                    lane,
                    weight_hint: None,
                };
                let permit = match engine.admit(&egress_req) {
                    crate::egress::Decision::Admit(permit) => permit,
                    // The component is told to stop. It never sleeps and retries;
                    // the engine owns the clock.
                    crate::egress::Decision::Refuse(reason) => {
                        return Ok(BrokerHttpResponse {
                            status: 0,
                            headers: vec![],
                            body: String::new(),
                            error_class: Some(reason.as_str().to_string()),
                        });
                    }
                };

                match transport.send(&prepared) {
                    Ok(response) => {
                        let headers = redact_response_headers(&response.headers);
                        engine.record(
                            permit,
                            &crate::egress::Outcome::http(
                                response.status,
                                headers.clone(),
                                &response.body,
                            ),
                        );
                        let error_class = classify_response(response.status, &response.body);
                        (response.status, headers, response.body, error_class)
                    }
                    // Transport errors carry no body the component can trust — and
                    // are not a venue instruction, so nothing freezes.
                    Err(_) => {
                        engine.record(permit, &crate::egress::Outcome::Transport);
                        (0, vec![], String::new(), Some("network".to_string()))
                    }
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

/// Stamp host-owned identity onto one fill (ADR 0004). Connection, slug, and all
/// three taxonomy axes come from the connection's shipping **book** config; any
/// adapter-supplied values are overwritten so a component cannot spoof another
/// connection or smuggle a legacy class string. Pure function so the stamp table
/// is unit-testable without a Wasm component (component round-trips stay in the
/// `ubi_*_component` / realign integration tests).
pub fn stamp_fill_identity(config: &UbiHostConfig, mut fill: FillEvent) -> FillEvent {
    fill.connection_id = config.connection_id.clone();
    fill.broker_slug = config.broker_slug.clone();
    fill.asset_class = WitAssetClass::from(config.asset_class);
    fill.instrument_class = WitInstrumentClass::from(config.instrument_class);
    fill.is_inverse = config.is_inverse;
    fill
}

/// Load a fixture Wasm component, call `fetch-fills`, inject identity fields (R5 §3.5, ADR 0004).
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

    let config = store.data().config.clone();

    // Host injects / overwrites identity fields so a component cannot spoof another connection.
    let fills: Vec<FillEvent> = result
        .into_iter()
        .map(|f| stamp_fill_identity(&config, f))
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
    // A missing file is an operational Io error, not a contract mismatch — keep
    // the distinction so "adapter not built" never misreports as "pre-bump".
    if !component_path.exists() {
        return Err(UbiHostError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("adapter component not found: {}", component_path.display()),
        )));
    }
    // Present-but-unloadable bytes (including every pre-bump 0.2.0 build, whose
    // exports no longer match the 0.3.0 world) are rejected loudly here.
    let component = Component::from_file(&engine, component_path)
        .map_err(|e| UbiHostError::contract_mismatch(format!("load: {e}")))?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    BrokerAdapterData::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut store = Store::new(&engine, state);
    let bindings = BrokerAdapterData::instantiate(&mut store, &component, &linker)
        .map_err(|e| UbiHostError::contract_mismatch(format!("instantiate: {e}")))?;
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
        let mut state = UbiHostState::live_isolated(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
        let mut state = UbiHostState::live_isolated(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
        let mut state = UbiHostState::live_isolated(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "kotak_neo".into(),
                book_id: crate::data::KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
                asset_class: AssetClass::Equity, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
        let mut state = UbiHostState::live_isolated(
            UbiHostConfig {
                connection_id: "c".into(),
                broker_slug: "binance_com".into(),
                book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
        assert_eq!(resp.error_class, None, "egress refused the call");
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
                asset_class: AssetClass::Cryptocurrency, instrument_class: InstrumentClass::Spot, is_inverse: false,
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
            asset_class: AssetClass::Equity, instrument_class: InstrumentClass::Spot, is_inverse: false,
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

    // ---- ADR 0004: stamp-by-book + pre-bump rejection ----

    fn unstamped_fill() -> FillEvent {
        // Adapter-side placeholders the host must overwrite (spoof values).
        FillEvent {
            fill_id: "F-1".into(),
            broker_slug: "spoof".into(),
            connection_id: "spoof".into(),
            asset_class: WitAssetClass::Fx,
            instrument_class: WitInstrumentClass::Swap,
            is_inverse: true,
            symbol: "X".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 1.0,
            currency: "USD".into(),
            filled_at_unix_ms: 1,
            fee_amount: None,
            fee_currency: None,
            exchange_segment: None,
            product: None,
            trade_id: None,
        }
    }

    fn config_for_book(book_id: &str, broker_slug: &str) -> UbiHostConfig {
        let descriptor = super::super::catalog::descriptor_for_book_id(book_id)
            .unwrap_or_else(|| panic!("no book row for {book_id}"));
        assert_eq!(
            descriptor.slug, broker_slug,
            "book {book_id} slug drift"
        );
        UbiHostConfig {
            connection_id: "conn-book".into(),
            broker_slug: broker_slug.into(),
            book_id: book_id.into(),
            asset_class: descriptor.asset_class,
            instrument_class: descriptor.instrument_class,
            is_inverse: descriptor.is_inverse,
            credentials: HostCredentialBlob::hmac("k", "s"),
        }
    }

    #[test]
    fn all_six_books_stamp_own_axes_from_book_config() {
        let cases: &[(&str, &str, WitAssetClass, WitInstrumentClass, bool)] = &[
            (
                "binance-com-spot",
                "binance_com",
                WitAssetClass::Cryptocurrency,
                WitInstrumentClass::Spot,
                false,
            ),
            (
                "binance-com-options",
                "binance_com",
                WitAssetClass::Cryptocurrency,
                WitInstrumentClass::Option,
                false,
            ),
            (
                "binance-com-usdm",
                "binance_com",
                WitAssetClass::Cryptocurrency,
                WitInstrumentClass::Future,
                false,
            ),
            (
                "binance-com-coinm",
                "binance_com",
                WitAssetClass::Cryptocurrency,
                WitInstrumentClass::Future,
                true,
            ),
            (
                "kotak-nse-bse-cash",
                "kotak_neo",
                WitAssetClass::Equity,
                WitInstrumentClass::Spot,
                false,
            ),
            (
                "kotak-nse-nfo",
                "kotak_neo",
                WitAssetClass::Equity,
                WitInstrumentClass::Option,
                false,
            ),
        ];
        for (book_id, slug, asset, instrument, inverse) in cases {
            let config = config_for_book(book_id, slug);
            let stamped = stamp_fill_identity(&config, unstamped_fill());
            assert_eq!(stamped.asset_class, *asset, "book {book_id} asset axis");
            assert_eq!(
                stamped.instrument_class, *instrument,
                "book {book_id} instrument axis"
            );
            assert_eq!(stamped.is_inverse, *inverse, "book {book_id} inverse leg");
            assert_eq!(stamped.connection_id, "conn-book");
            assert_eq!(stamped.broker_slug, *slug);
            assert_ne!(stamped.broker_slug, "spoof");
        }
    }

    #[test]
    fn nfo_stamp_is_nfo_axes_not_slug_cash_pair_f3_flip() {
        // The F3 flip at host level: an NFO-book fill must carry NFO axes even
        // though the slug's first pair (cash) is (equity, spot). The old
        // `host_stamps_equities_on_nfo_row_f3` integration test (tail lane) goes
        // red; this inline test locks the flipped behavior.
        let nfo = stamp_fill_identity(
            &config_for_book("kotak-nse-nfo", "kotak_neo"),
            unstamped_fill(),
        );
        let cash = stamp_fill_identity(
            &config_for_book("kotak-nse-bse-cash", "kotak_neo"),
            unstamped_fill(),
        );
        assert_eq!(nfo.asset_class, WitAssetClass::Equity);
        assert_eq!(nfo.instrument_class, WitInstrumentClass::Option);
        assert_eq!(cash.instrument_class, WitInstrumentClass::Spot);
        assert_ne!(
            nfo.instrument_class, cash.instrument_class,
            "NFO must not inherit the cash stamp"
        );
    }

    #[test]
    fn coinm_inverse_leg_is_bool_not_class_string() {
        let stamped = stamp_fill_identity(
            &config_for_book("binance-com-coinm", "binance_com"),
            unstamped_fill(),
        );
        assert!(stamped.is_inverse);
        assert_eq!(stamped.instrument_class, WitInstrumentClass::Future);
        let usdm = stamp_fill_identity(
            &config_for_book("binance-com-usdm", "binance_com"),
            unstamped_fill(),
        );
        assert!(!usdm.is_inverse);
        assert_eq!(usdm.instrument_class, WitInstrumentClass::Future);
    }

    #[test]
    fn contract_mismatch_names_expected_version() {
        assert_eq!(UBI_DATA_CONTRACT_VERSION, "0.3.0");
        let err = UbiHostError::contract_mismatch("test detail");
        let msg = err.to_string();
        assert!(msg.contains("tradeautopsy:ubi-data@0.3.0"), "{msg}");
        assert!(msg.contains("rejected"), "{msg}");
        assert!(msg.contains("0.2.0"), "{msg}");
        assert!(msg.contains("test detail"), "{msg}");
    }

    #[test]
    fn non_component_bytes_are_rejected_loudly_not_mapped() {
        // Any present-but-unloadable bytes — the shape a stale 0.2.0 build takes
        // at the 0.3.0 host — must fail as ContractMismatch naming 0.3.0.
        let path = std::env::temp_dir().join(format!(
            "ta-p1a-not-a-component-{}-{}.wasm",
            std::process::id(),
            // Tiny uniqueness without new deps: nanos since epoch.
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, b"definitely not a wasm component").expect("temp write");
        let state = UbiHostState::new(
            config_for_book("binance-com-spot", "binance_com"),
            HashMap::new(),
        );
        let err = match run_fetch_fills(
            &path,
            state,
            FillCursor {
                since_unix_ms: None,
                from_id: None,
                symbol: None,
            },
        ) {
            Err(e) => e,
            Ok(_) => panic!("garbage bytes must not instantiate"),
        };
        let msg = err.to_string();
        assert!(
            matches!(err, UbiHostError::ContractMismatch(_)),
            "expected ContractMismatch, got: {msg}"
        );
        assert!(msg.contains("tradeautopsy:ubi-data@0.3.0"), "{msg}");
        assert!(msg.contains("rejected"), "{msg}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_component_stays_io_not_mismatch() {
        let state = UbiHostState::new(
            config_for_book("binance-com-spot", "binance_com"),
            HashMap::new(),
        );
        let err = match run_fetch_fills(
            Path::new("/nonexistent-dir-p1a/no-adapter.wasm"),
            state,
            FillCursor {
                since_unix_ms: None,
                from_id: None,
                symbol: None,
            },
        ) {
            Err(e) => e,
            Ok(_) => panic!("missing file must fail"),
        };
        assert!(
            matches!(err, UbiHostError::Io(_)),
            "missing file must stay Io, got: {err}"
        );
    }
}
