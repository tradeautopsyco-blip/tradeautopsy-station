//! Wasmtime host for `tradeautopsy:ubi/broker-adapter` (Phase 1 stub).

use crate::ubi::allowlist::host_allowed;
use std::collections::HashMap;
use std::path::Path;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "wit",
    world: "broker-adapter",
});

pub use tradeautopsy::ubi::types::{
    BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpHeader,
};

/// Headers a component must never set; host strips or rejects (R6).
pub const FORBIDDEN_COMPONENT_HEADERS: &[&str] = &[
    "authorization",
    "x-mbx-apikey",
    "auth",
    "sid",
];

/// Host-only credential material. Never copied into Wasm component memory / HTTP responses.
#[derive(Debug, Clone)]
pub struct HostCredentialBlob {
    pub api_key: String,
    pub api_secret: String,
}

/// Fixture response keyed by path (Phase 1 — no live HTTP).
#[derive(Debug, Clone)]
pub struct BrokerHttpFixture {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub error_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UbiHostConfig {
    pub connection_id: String,
    pub broker_slug: String,
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
    /// path → fixture (host must still be allowlisted)
    pub fixtures: HashMap<String, BrokerHttpFixture>,
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
        Self {
            config,
            fixtures,
            calls: Vec::new(),
            credential_leak_attempted: false,
            ctx: WasiCtx::builder().build(),
            table: ResourceTable::new(),
        }
    }

    fn response_contains_secret(&self, body: &str, headers: &[(String, String)]) -> bool {
        let key = &self.config.credentials.api_key;
        let secret = &self.config.credentials.api_secret;
        if !key.is_empty() && (body.contains(key) || headers.iter().any(|(_, v)| v.contains(key))) {
            return true;
        }
        if !secret.is_empty()
            && (body.contains(secret) || headers.iter().any(|(_, v)| v.contains(secret)))
        {
            return true;
        }
        false
    }
}

impl tradeautopsy::ubi::types::Host for UbiHostState {}

impl tradeautopsy::ubi::broker_http::Host for UbiHostState {
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

        if !host_allowed(&request.host) {
            return Ok(BrokerHttpResponse {
                status: 0,
                headers: vec![],
                body: String::new(),
                error_class: Some("host_blocked".to_string()),
            });
        }

        self.calls.push(request.clone());

        let fixture = self.fixtures.get(&request.path).ok_or_else(|| {
            format!(
                "no fixture for path {} on host {}",
                request.path, request.host
            )
        })?;

        // Phase 1 stub: host *would* attach auth from Keychain.
        // Credentials stay in HostState only — never in the response body/headers.
        let mut out_headers: Vec<HttpHeader> = fixture
            .headers
            .iter()
            .map(|(n, v)| HttpHeader {
                name: n.clone(),
                value: v.clone(),
            })
            .collect();

        // Simulate host-attached auth *presence* without secret values.
        if !self.config.credentials.api_key.is_empty() {
            out_headers.push(HttpHeader {
                name: "x-mbx-apikey-present".to_string(),
                value: "true".to_string(),
            });
        }

        let header_pairs: Vec<(String, String)> = out_headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();
        if self.response_contains_secret(&fixture.body, &header_pairs) {
            self.credential_leak_attempted = true;
            return Err("host refused to return response containing raw credentials".into());
        }

        Ok(BrokerHttpResponse {
            status: fixture.status,
            headers: out_headers,
            body: fixture.body.clone(),
            error_class: fixture.error_class.clone(),
        })
    }
}

/// Load a fixture Wasm component, call `fetch-fills`, inject identity fields (R5 §3.5).
pub fn run_fetch_fills(
    component_path: &Path,
    state: UbiHostState,
    cursor: FillCursor,
) -> Result<(Vec<FillEvent>, UbiHostState), UbiHostError> {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, component_path)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    BrokerAdapter::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut store = Store::new(&engine, state);
    let bindings = BrokerAdapter::instantiate(&mut store, &component, &linker)?;

    let result = bindings
        .tradeautopsy_ubi_adapter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ubi::allowlist::host_allowed;
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
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob {
                    api_key: "k".into(),
                    api_secret: "s".into(),
                },
            },
            HashMap::new(),
        );
        let resp = tradeautopsy::ubi::broker_http::Host::broker_http_call(
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
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob {
                    api_key: "k".into(),
                    api_secret: "s".into(),
                },
            },
            HashMap::new(),
        );
        let err = tradeautopsy::ubi::broker_http::Host::broker_http_call(
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
                asset_class: "crypto_spot".into(),
                credentials: HostCredentialBlob {
                    api_key: "KEY".into(),
                    api_secret: secret.into(),
                },
            },
            fixtures,
        );
        let resp = tradeautopsy::ubi::broker_http::Host::broker_http_call(
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
}
