//! `okx_com` adapter component contract (Phase 4 · ADR 0016 · B6 okx_com · R5/R6).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tradeautopsy_agent::{
    host_allowed, run_fetch_fills, run_obtain, AssetClass, BrokerHttpFixture, FillCursor,
    HostCredentialBlob, InstrumentClass, RecordingTransport, TransportResponse, UbiHostConfig,
    UbiHostState, WitAssetClass, OKX_COM_SPOT_BOOK_ID,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm_from_crate};

const FILLS_PATH: &str = "/api/v5/trade/fills";

fn okx_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-okx-adapter", "ubi_okx_adapter.wasm")
}

fn sentinel_okx() -> HostCredentialBlob {
    HostCredentialBlob::OkxSession {
        api_key: "TEST_OKX_KEY_NEVER_IN_COMPONENT".into(),
        api_secret: "TEST_OKX_SECRET_NEVER_IN_COMPONENT".into(),
        passphrase: "TEST_OKX_PASSPHRASE_NEVER_IN_COMPONENT".into(),
    }
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-okx-001".into(),
        broker_slug: "okx_com".into(),
        book_id: OKX_COM_SPOT_BOOK_ID.into(),
        asset_class: AssetClass::Cryptocurrency,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: sentinel_okx(),
    }
}

fn fixture_state() -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        FILLS_PATH.to_string(),
        BrokerHttpFixture {
            status: 200,
            body: ubi_support::read_fixture("okx_com_fills.json"),
            headers: vec![],
            error_class: None,
        },
    );
    UbiHostState::new(config(), fixtures)
}

fn empty_cursor() -> FillCursor {
    FillCursor {
        since_unix_ms: None,
        from_id: None,
        symbol: None,
    }
}

#[test]
fn fills_row_maps_to_fill_event() {
    let wasm = okx_component_wasm();
    let state = fixture_state();

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    let fill = &fills[0];
    assert_eq!(fill.fill_id, "28457");
    assert_eq!(fill.broker_slug, "okx_com");
    assert_eq!(fill.connection_id, "conn-okx-001");
    assert_eq!(fill.asset_class, WitAssetClass::Cryptocurrency);
    assert_eq!(fill.symbol, "BTC-USDT");
    assert_eq!(fill.side, "BUY");
    assert!((fill.qty - 0.012).abs() < 1e-9);
    assert!((fill.price - 40000.1).abs() < 1e-9);
    assert_eq!(fill.currency, "USDT");
    assert_eq!(fill.filled_at_unix_ms, 1_499_865_549_590);
    assert!((fill.fee_amount.unwrap() - 0.012).abs() < 1e-9);
    assert_eq!(fill.fee_currency.as_deref(), Some("USDT"));
    assert_eq!(fill.trade_id.as_deref(), Some("100234"));

    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn calls_target_www_okx_com_only() {
    let wasm = okx_component_wasm();
    let (_fills, state) = run_fetch_fills(&wasm, fixture_state(), empty_cursor()).expect("fetch");

    assert_eq!(state.calls.len(), 1);
    assert_eq!(state.calls[0].host, "www.okx.com");
    assert_eq!(state.calls[0].path, FILLS_PATH);
    assert!(state.calls.iter().all(|c| host_allowed(&c.host)));
    assert!(state.calls[0]
        .query
        .iter()
        .any(|q| q.name == "instType" && q.value == "SPOT"));
}

#[test]
fn sibling_hosts_are_not_allowlisted() {
    assert!(host_allowed("www.okx.com"));
    assert!(!host_allowed("us.okx.com"));
    assert!(!host_allowed("eea.okx.com"));
}

#[test]
fn live_transport_attaches_ok_access_headers() {
    let wasm = okx_component_wasm();
    let transport = Arc::new(RecordingTransport::new(Ok(TransportResponse {
        status: 200,
        headers: vec![],
        body: ubi_support::read_fixture("okx_com_fills.json"),
    })));

    let (_fills, state) = run_fetch_fills(
        &wasm,
        UbiHostState::live(config(), transport.clone()),
        empty_cursor(),
    )
    .expect("live fetch");

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    let req = &sent[0];
    assert!(req.url.starts_with("https://www.okx.com/"));
    assert!(req.headers.iter().any(|(n, _)| n == "OK-ACCESS-KEY"));
    assert!(req.headers.iter().any(|(n, _)| n == "OK-ACCESS-SIGN"));
    assert!(req.headers.iter().any(|(n, _)| n == "OK-ACCESS-TIMESTAMP"));
    assert!(req.headers.iter().any(|(n, _)| n == "OK-ACCESS-PASSPHRASE"));
    assert!(!req
        .headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("x-simulated-trading")));
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn obtain_history_candles_fixture() {
    let wasm = okx_component_wasm();
    let mut fixtures = HashMap::new();
    fixtures.insert(
        "/api/v5/market/candles".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: r#"{"code":"0","msg":"","data":[["0","1","2","3","4","5","6","7","8"]]}"#.into(),
            headers: vec![],
            error_class: None,
        },
    );
    let request = serde_json::json!({
        "operation": "history",
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "instrument-id": "BTC-USDT",
        "interval": "1m",
    })
    .to_string();
    let (body, state) =
        run_obtain(&wasm, UbiHostState::new(config(), fixtures), &request).expect("obtain");
    assert!(body.contains("\"code\":\"0\""));
    assert_component_never_saw_secrets(&state, &wasm);
}
