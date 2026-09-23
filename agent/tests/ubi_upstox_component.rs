//! `upstox` adapter component contract (Phase 3 · ADR 0001 · B6 upstox · R5/R6).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use tradeautopsy_agent::{
    run_describe, run_fetch_fills, run_obtain, AssetClass, BrokerHttpFixture, FillCursor,
    InstrumentClass, UbiHostConfig, UbiHostState, WitAssetClass,
};
use ubi_support::{
    assert_component_never_saw_secrets, component_wasm_from_crate, sentinel_hmac,
};

const TRADES_PATH: &str = "/v2/order/trades/get-trades-for-day";
const UPSTOX_NSE_BSE_CASH_BOOK_ID: &str = "upstox-nse-bse-cash";
/// 2026-07-25 10:15:30 IST → 04:45:30 UTC.
const ITBEES_FILLED_AT_MS: i64 = 1_784_954_730_000;
/// 2026-07-25 14:05:00 IST → 08:35:00 UTC.
const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;

fn upstox_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-upstox-adapter", "ubi_upstox_adapter.wasm")
}

fn read_upstox_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/upstox")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-upstox-001".into(),
        broker_slug: "upstox".into(),
        book_id: UPSTOX_NSE_BSE_CASH_BOOK_ID.into(),
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: sentinel_hmac(),
    }
}

fn fixture_state(status: u16, body: String) -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        TRADES_PATH.to_string(),
        BrokerHttpFixture {
            status,
            body,
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
fn trades_day_book_maps_to_fill_events() {
    let wasm = upstox_component_wasm();
    let state = fixture_state(200, read_upstox_fixture("trades_day_book.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(
        fills.len(),
        2,
        "cash I/D only; NFO, MTF, and CO dropped"
    );
    let itbees = &fills[0];
    assert_eq!(itbees.fill_id, "500001");
    assert_eq!(itbees.broker_slug, "upstox");
    assert_eq!(itbees.connection_id, "conn-upstox-001");
    assert_eq!(itbees.asset_class, WitAssetClass::Equity);
    assert_eq!(itbees.symbol, "ITBEES-EQ");
    assert_eq!(itbees.side, "BUY");
    assert!((itbees.qty - 100.0).abs() < 1e-9);
    assert!((itbees.price - 25.5).abs() < 1e-9);
    assert_eq!(itbees.currency, "INR");
    assert_eq!(itbees.exchange_segment.as_deref(), Some("NSE"));
    assert_eq!(itbees.product.as_deref(), Some("I"));
    assert_eq!(itbees.trade_id.as_deref(), Some("0800000000000001"));
    assert_eq!(itbees.filled_at_unix_ms, ITBEES_FILLED_AT_MS);
    assert!(itbees.fee_amount.is_none());

    let reliance = &fills[1];
    assert_eq!(reliance.symbol, "RELIANCE-EQ");
    assert_eq!(reliance.side, "SELL");
    assert_eq!(reliance.product.as_deref(), Some("D"));
    assert_eq!(reliance.filled_at_unix_ms, RELIANCE_FILLED_AT_MS);
    assert_eq!(reliance.fill_id, "500002");

    assert!(!fills.iter().any(|f| f.symbol.starts_with("TCS") || f.symbol.starts_with("NIFTY")));
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn symbol_falls_back_to_trading_symbol_key() {
    let wasm = upstox_component_wasm();
    let body = r#"{
        "status":"success",
        "data":[{
            "trade_id":"9001",
            "order_id":"8001",
            "trading_symbol":"SBIN-EQ",
            "exchange":"NSE",
            "transaction_type":"BUY",
            "product":"I",
            "average_price":585.25,
            "quantity":1,
            "exchange_timestamp":"2026-07-25 12:00:00"
        }]
    }"#;
    let (fills, _state) = run_fetch_fills(&wasm, fixture_state(200, body.into()), empty_cursor())
        .expect("fetch_fills");
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "SBIN-EQ");
    assert!((fills[0].qty - 1.0).abs() < 1e-9);
}

#[test]
fn cursor_since_filters_the_day_book() {
    let wasm = upstox_component_wasm();
    let state = fixture_state(200, read_upstox_fixture("trades_day_book.json"));

    let (fills, _state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(RELIANCE_FILLED_AT_MS),
            from_id: None,
            symbol: None,
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "RELIANCE-EQ");
}

#[test]
fn empty_day_book_is_zero_fills() {
    let wasm = upstox_component_wasm();
    for body in [
        r#"{"status":"success","data":[]}"#,
        r#"{"status":"success","data":null}"#,
    ] {
        let (fills, _state) =
            run_fetch_fills(&wasm, fixture_state(200, body.to_string()), empty_cursor())
                .expect("empty data is success");
        assert!(fills.is_empty(), "body={body}");
    }
}

#[test]
fn unauthorized_surfaces_as_session_expired() {
    let wasm = upstox_component_wasm();
    let err = run_fetch_fills(
        &wasm,
        fixture_state(
            401,
            r#"{"status":"error","errors":[{"errorCode":"UDAPI100038","message":"Invalid token"}]}"#
                .to_string(),
        ),
        empty_cursor(),
    )
    .map(|_| ())
    .expect_err("unauthorized");
    assert!(
        err.to_string().contains("session_expired"),
        "unexpected: {err}"
    );
}

#[test]
fn obtain_history_is_unsupported_not_empty_success() {
    let wasm = upstox_component_wasm();
    let request = serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "instrument-id": "RELIANCE-EQ",
        "operation": "history",
    })
    .to_string();
    let (body, state) = run_obtain(&wasm, fixture_state(200, "{}".into()), &request).expect("obtain");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["status"], "unsupported");
    assert!(state.calls.is_empty());
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn describe_does_not_claim_history() {
    let wasm = upstox_component_wasm();
    let (body, _state) = run_describe(&wasm, fixture_state(200, "{}".into())).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("describe json");
    let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
    assert!(
        !implemented.iter().any(|v| v.as_str() == Some("history")),
        "describe must not claim history: {body}"
    );
    assert_eq!(
        json["manifest_id"].as_str(),
        Some("tradeautopsy:upstox-cash@0.1.0")
    );
}
