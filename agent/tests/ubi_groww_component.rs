//! `groww` adapter component contract (Phase 3 · ADR 0014 · B6 groww · R5/R6).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use tradeautopsy_agent::{
    run_describe, run_fetch_fills, run_obtain, AssetClass, BrokerHttpFixture, FillCursor,
    HostCredentialBlob, InstrumentClass, UbiHostConfig, UbiHostState, WitAssetClass,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm_from_crate};

const ORDER_LIST_PATH: &str = "/v1/order/list";
const GROWW_NSE_BSE_CASH_BOOK_ID: &str = "groww-nse-bse-cash";
/// 2026-09-24 10:15:30 +05:30.
const FIRST_FILL_MS: i64 = 1_790_225_130_000;
/// 2026-09-24 14:05:00 +05:30.
const SINGLE_FILL_MS: i64 = 1_790_238_900_000;

fn groww_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-groww-adapter", "ubi_groww_adapter.wasm")
}

fn read_groww_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/groww")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-groww-001".into(),
        broker_slug: "groww".into(),
        book_id: GROWW_NSE_BSE_CASH_BOOK_ID.into(),
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: HostCredentialBlob::GrowwSession {
            token: "TEST_GROWW_TOKEN_NEVER_IN_COMPONENT".into(),
        },
    }
}

fn fan_out_fixture_state() -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        ORDER_LIST_PATH.to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("order_list_page.json"),
            headers: vec![],
            error_class: None,
        },
    );
    fixtures.insert(
        "/v1/order/trades/GWKFILLMULTI01".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("trades_order_multi.json"),
            headers: vec![],
            error_class: None,
        },
    );
    fixtures.insert(
        "/v1/order/trades/GWKFILLSINGLE02".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("trades_order_single.json"),
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
fn obtain_history_is_unsupported_not_empty_success() {
    let wasm = groww_component_wasm();
    let request = serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "operation": "history",
    })
    .to_string();
    let (body, state) =
        run_obtain(&wasm, fan_out_fixture_state(), &request).expect("obtain");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["status"], "unsupported");
    assert!(state.calls.is_empty());
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn describe_claims_tradebook_fan_out_only() {
    let wasm = groww_component_wasm();
    let (body, _state) = run_describe(&wasm, fan_out_fixture_state()).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("describe json");
    let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
    assert!(
        !implemented.iter().any(|v| v.as_str() == Some("history")),
        "describe must not claim history: {body}"
    );
    assert_eq!(
        json["manifest_id"].as_str(),
        Some("tradeautopsy:groww-cash@0.1.0")
    );
}

#[test]
fn fan_out_maps_filled_orders_and_skips_open_and_rejected() {
    let wasm = groww_component_wasm();
    let state = fan_out_fixture_state();

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 4);
    assert_eq!(state.calls.len(), 3);
    assert_eq!(state.calls[0].path, ORDER_LIST_PATH);
    assert_eq!(state.calls[1].path, "/v1/order/trades/GWKFILLMULTI01");
    assert_eq!(state.calls[2].path, "/v1/order/trades/GWKFILLSINGLE02");

    let first = &fills[0];
    assert_eq!(first.fill_id, "11000012345678");
    assert_eq!(first.broker_slug, "groww");
    assert_eq!(first.connection_id, "conn-groww-001");
    assert_eq!(first.asset_class, WitAssetClass::Equity);
    assert_eq!(first.symbol, "RELIANCE");
    assert_eq!(first.side, "BUY");
    assert!((first.qty - 20.0).abs() < 1e-9);
    assert!((first.price - 1450.75).abs() < 1e-9);
    assert_eq!(first.currency, "INR");
    assert_eq!(first.exchange_segment.as_deref(), Some("CASH"));
    assert_eq!(first.product.as_deref(), Some("CNC"));
    assert_eq!(first.trade_id.as_deref(), Some("20000012345678"));
    assert_eq!(first.filled_at_unix_ms, FIRST_FILL_MS);
    assert!(first.fee_amount.is_none());

    let single = &fills[3];
    assert_eq!(single.symbol, "SBIN");
    assert_eq!(single.side, "SELL");
    assert_eq!(single.product.as_deref(), Some("MIS"));
    assert_eq!(single.filled_at_unix_ms, SINGLE_FILL_MS);

    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn per_order_404_is_a_named_gap_never_synthetic() {
    let wasm = groww_component_wasm();
    let mut fixtures = HashMap::new();
    fixtures.insert(
        ORDER_LIST_PATH.to_string(),
        BrokerHttpFixture {
            status: 200,
            body: r#"{"status":"SUCCESS","payload":{"order_list":[
                {"groww_order_id":"GWKGAP01","order_status":"EXECUTED","filled_quantity":10}
            ]}}"#
            .into(),
            headers: vec![],
            error_class: None,
        },
    );
    fixtures.insert(
        "/v1/order/trades/GWKGAP01".to_string(),
        BrokerHttpFixture {
            status: 404,
            body: r#"{"status":"FAILURE","error":{"code":"GA004","message":"Order does not exist"}}"#
                .into(),
            headers: vec![],
            error_class: Some("http_error".to_string()),
        },
    );
    let state = UbiHostState::new(config(), fixtures);

    let err = run_fetch_fills(&wasm, state, empty_cursor())
        .map(|_| ())
        .expect_err("404 gap");
    assert!(err.to_string().contains("trades_gap"));
    assert!(err.to_string().contains("GWKGAP01"));
    assert!(!err.to_string().contains("synthetic"));
}

#[test]
fn ga005_surfaces_as_session_expired() {
    let wasm = groww_component_wasm();
    let mut fixtures = HashMap::new();
    fixtures.insert(
        ORDER_LIST_PATH.to_string(),
        BrokerHttpFixture {
            status: 200,
            body: r#"{"status":"FAILURE","error":{"code":"GA005","message":"not authorised"}}"#
                .into(),
            headers: vec![],
            error_class: None,
        },
    );
    let state = UbiHostState::new(config(), fixtures);
    let err = run_fetch_fills(&wasm, state, empty_cursor())
        .map(|_| ())
        .expect_err("GA005");
    assert!(err.to_string().contains("session_expired"));
}

#[test]
fn cursor_since_filters_fan_out() {
    let wasm = groww_component_wasm();
    let state = fan_out_fixture_state();

    let (fills, _state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(SINGLE_FILL_MS),
            from_id: None,
            symbol: None,
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "SBIN");
}
