//! `bybit` adapter component contract (Phase 4 · ADR 0015 · B6 bybit · R5/R6).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tradeautopsy_agent::{
    host_allowed, prepare_request, run_fetch_fills, run_obtain, AssetClass, BrokerHttpFixture,
    FillCursor, InstrumentClass, RecordingTransport, UbiHostConfig, UbiHostState, WitAssetClass,
};
use tradeautopsy_agent::{BYBIT_API_HOST, BYBIT_BOOK_ID, BYBIT_RECV_WINDOW};
use ubi_support::{assert_component_never_saw_secrets, component_wasm_from_crate, sentinel_hmac};

const WALLET_PATH: &str = "/v5/account/wallet-balance";
const EXECUTION_PATH: &str = "/v5/execution/list";
const KLINE_PATH: &str = "/v5/market/kline";

fn bybit_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-bybit-adapter", "ubi_bybit_adapter.wasm")
}

fn read_bybit_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/bybit")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-bybit-001".into(),
        broker_slug: "bybit".into(),
        book_id: BYBIT_BOOK_ID.into(),
        asset_class: AssetClass::Cryptocurrency,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: sentinel_hmac(),
    }
}

fn fixture_state(entries: Vec<(&str, u16, String)>) -> UbiHostState {
    let mut fixtures = HashMap::new();
    for (path, status, body) in entries {
        fixtures.insert(
            path.to_string(),
            BrokerHttpFixture {
                status,
                body,
                headers: vec![],
                error_class: None,
            },
        );
    }
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
fn execution_row_maps_to_fill_event() {
    let wasm = bybit_wasm();
    let state = fixture_state(vec![
        (WALLET_PATH, 200, read_bybit_fixture("bybit_wallet_balance.json")),
        (EXECUTION_PATH, 200, read_bybit_fixture("bybit_execution_list.json")),
    ]);

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    let fill = &fills[0];
    assert_eq!(fill.fill_id, "e0cbe81d-0f18-5866-9415-cf319b5dab3b");
    assert_eq!(fill.broker_slug, "bybit");
    assert_eq!(fill.connection_id, "conn-bybit-001");
    assert_eq!(fill.asset_class, WitAssetClass::Cryptocurrency);
    assert_eq!(fill.symbol, "BTCUSDT");
    assert_eq!(fill.side, "BUY");
    assert!((fill.qty - 0.01).abs() < 1e-9);
    assert!((fill.price - 42100.5).abs() < 1e-9);
    assert_eq!(fill.currency, "USDT");
    assert_eq!(fill.filled_at_unix_ms, 1_700_000_123_456);
    assert!((fill.fee_amount.unwrap() - 0.000015).abs() < 1e-9);
    assert_eq!(fill.fee_currency.as_deref(), Some("BTC"));
    assert_eq!(
        fill.trade_id.as_deref(),
        Some("8c065341-7b52-4ca9-ac2c-37e31ac55c94")
    );
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn wallet_bootstrap_derives_btcusdt_and_skips_stables() {
    let wasm = bybit_wasm();
    let state = fixture_state(vec![
        (WALLET_PATH, 200, read_bybit_fixture("bybit_wallet_balance.json")),
        (EXECUTION_PATH, 200, read_bybit_fixture("bybit_execution_list.json")),
    ]);

    let (_fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(state.calls.len(), 2);
    assert_eq!(state.calls[0].path, WALLET_PATH);
    assert_eq!(state.calls[1].path, EXECUTION_PATH);
    assert!(state.calls.iter().all(|c| c.host == BYBIT_API_HOST));
    assert!(state.calls.iter().all(|c| host_allowed(&c.host)));
    let symbols: Vec<&str> = state.calls[1]
        .query
        .iter()
        .filter(|q| q.name == "symbol")
        .map(|q| q.value.as_str())
        .collect();
    assert_eq!(symbols, vec!["BTCUSDT"]);
    assert!(state.calls[1]
        .query
        .iter()
        .any(|q| q.name == "category" && q.value == "spot"));
}

#[test]
fn pinned_symbol_skips_wallet_and_honors_time_window() {
    let wasm = bybit_wasm();
    let state = fixture_state(vec![(
        EXECUTION_PATH,
        200,
        read_bybit_fixture("bybit_execution_list.json"),
    )]);

    let (_fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(1_699_000_000_000),
            from_id: None,
            symbol: Some("BTCUSDT".into()),
        },
    )
    .expect("fetch_fills");

    assert_eq!(state.calls.len(), 1);
    let query: HashMap<&str, &str> = state.calls[0]
        .query
        .iter()
        .map(|q| (q.name.as_str(), q.value.as_str()))
        .collect();
    assert_eq!(query.get("symbol"), Some(&"BTCUSDT"));
    assert!(query.contains_key("startTime"));
    assert!(query.contains_key("endTime"));
    assert!(!query.contains_key("cursor"));
}

#[test]
fn quote_filter_refuses_non_usd_stable_quotes() {
    let wasm = bybit_wasm();
    let state = fixture_state(vec![]);

    let err = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: None,
            from_id: None,
            symbol: Some("ETHBTC".into()),
        },
    )
    .map(|_| ())
    .expect_err("quote filter")
    .to_string();
    assert!(err.contains("quote-filter"));
}

#[test]
fn host_attaches_bybit_hmac_headers_not_binance_shape() {
    let creds = sentinel_hmac();
    let prepared = prepare_request(
        "GET",
        BYBIT_API_HOST,
        EXECUTION_PATH,
        &[
            ("category".into(), "spot".into()),
            ("symbol".into(), "BTCUSDT".into()),
        ],
        &[],
        None,
        &creds,
        1_700_000_000_000,
    );
    assert!(!prepared.url.contains("signature="));
    assert!(!prepared.url.contains("timestamp="));
    assert!(prepared.headers.iter().any(|(n, _)| n == "X-BAPI-SIGN"));
    assert!(prepared
        .headers
        .iter()
        .any(|(n, v)| n == "X-BAPI-API-KEY" && v.contains("TEST_API_KEY")));
    assert!(prepared
        .headers
        .iter()
        .any(|(n, v)| n == "X-BAPI-RECV-WINDOW" && v == BYBIT_RECV_WINDOW));
}

#[test]
fn ret_code_nonzero_surfaces_as_error() {
    let wasm = bybit_wasm();
    let body = r#"{"retCode":10006,"retMsg":"Too many visits","result":{},"time":1}"#;
    let state = fixture_state(vec![(EXECUTION_PATH, 200, body.to_string())]);

    let err = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: None,
            from_id: None,
            symbol: Some("BTCUSDT".into()),
        },
    )
    .map(|_| ())
    .expect_err("retCode")
    .to_string();
    assert!(err.contains("10006"));
}

#[test]
fn obtain_kline_maps_candles_for_usdt_pair() {
    let wasm = bybit_wasm();
    let kline_body = r#"{
        "retCode": 0,
        "retMsg": "OK",
        "result": {
            "category": "spot",
            "list": [
                ["1700000000000","42000","42100","41900","42050","1.5","1700003599999"]
            ]
        }
    }"#;
    let state = fixture_state(vec![(KLINE_PATH, 200, kline_body.to_string())]);
    let request = serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "instrument-id": "BTCUSDT",
        "interval": "1",
        "limit": 10
    })
    .to_string();

    let (body, state) = run_obtain(&wasm, state, &request).expect("obtain");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["instrument_id"], "BTCUSDT");
    assert_eq!(json["candles"].as_array().map(|a| a.len()), Some(1));
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn live_mode_host_signs_bybit_requests_the_component_never_sees() {
    let wasm = bybit_wasm();
    let transport = Arc::new(RecordingTransport::routed(vec![
        RecordingTransport::json_route(
            EXECUTION_PATH,
            200,
            &read_bybit_fixture("bybit_execution_list.json"),
        ),
    ]));

    let (fills, state) = run_fetch_fills(
        &wasm,
        UbiHostState::live(config(), transport.clone()),
        FillCursor {
            symbol: Some("BTCUSDT".into()),
            ..empty_cursor()
        },
    )
    .expect("live fetch_fills");

    assert_eq!(fills.len(), 1);
    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    for request in &sent {
        assert!(request.url.starts_with("https://api.bybit.com/"));
        assert!(!request.url.contains("signature="));
        assert!(request.headers.iter().any(|(n, _)| n == "X-BAPI-SIGN"));
        assert!(request
            .headers
            .iter()
            .any(|(n, v)| n == "X-BAPI-API-KEY"
                && v == "TEST_API_KEY_SHOULD_NEVER_REACH_COMPONENT"));
        assert!(!request
            .url
            .contains("TEST_API_SECRET_SHOULD_NEVER_REACH_COMPONENT"));
    }
    assert_component_never_saw_secrets(&state, &wasm);
}
