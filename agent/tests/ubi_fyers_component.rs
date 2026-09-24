//! `fyers` adapter component contract (Phase 3 · ADR 0001 · B6 fyers · R5/R6).

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

const TRADEBOOK_PATH: &str = "/api/v3/tradebook";
const FYERS_NSE_BSE_CASH_BOOK_ID: &str = "fyers-nse-bse-cash";
/// 2026-07-25 10:15:30 IST → 04:45:30 UTC.
const ITBEES_FILLED_AT_MS: i64 = 1_784_954_730_000;
/// 2026-07-25 14:05:00 IST → 08:35:00 UTC.
const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;

fn fyers_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-fyers-adapter", "ubi_fyers_adapter.wasm")
}

fn read_fyers_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/fyers")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-fyers-001".into(),
        broker_slug: "fyers".into(),
        book_id: FYERS_NSE_BSE_CASH_BOOK_ID.into(),
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: sentinel_hmac(),
    }
}

fn fixture_state(status: u16, body: String) -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        TRADEBOOK_PATH.to_string(),
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
fn tradebook_maps_to_fill_events() {
    let wasm = fyers_component_wasm();
    let state = fixture_state(200, read_fyers_fixture("tradebook.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(
        fills.len(),
        2,
        "cash intraday/CNC only; NFO, MTF, and MARGIN dropped"
    );
    let itbees = &fills[0];
    assert_eq!(itbees.fill_id, "500001");
    assert_eq!(itbees.broker_slug, "fyers");
    assert_eq!(itbees.connection_id, "conn-fyers-001");
    assert_eq!(itbees.asset_class, WitAssetClass::Equity);
    assert_eq!(itbees.symbol, "ITBEES-EQ");
    assert_eq!(itbees.side, "BUY");
    assert!((itbees.qty - 100.0).abs() < 1e-9);
    assert!((itbees.price - 25.5).abs() < 1e-9);
    assert_eq!(itbees.currency, "INR");
    assert_eq!(itbees.exchange_segment.as_deref(), Some("NSE"));
    assert_eq!(itbees.product.as_deref(), Some("1"));
    assert_eq!(itbees.trade_id.as_deref(), Some("23121800000001"));
    assert_eq!(itbees.filled_at_unix_ms, ITBEES_FILLED_AT_MS);
    assert!(itbees.fee_amount.is_none());

    let reliance = &fills[1];
    assert_eq!(reliance.symbol, "RELIANCE-EQ");
    assert_eq!(reliance.side, "SELL");
    assert_eq!(reliance.product.as_deref(), Some("2"));
    assert_eq!(reliance.filled_at_unix_ms, RELIANCE_FILLED_AT_MS);
    assert_eq!(reliance.fill_id, "500002");

    assert!(!fills.iter().any(|f| f.symbol.starts_with("TCS") || f.symbol.starts_with("NIFTY")));
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn numeric_product_type_codes_are_accepted() {
    let wasm = fyers_component_wasm();
    let body = r#"{
        "s":"ok",
        "code":200,
        "tradeBook":[{
            "symbol":"NSE:SBIN-EQ",
            "tradeNumber":"9001",
            "orderNumber":"8001",
            "tradedQty":1,
            "tradePrice":585.25,
            "side":1,
            "productType":1,
            "segment":10,
            "exchange":10,
            "orderDateTime":"25-07-2026 12:00:00"
        }]
    }"#;
    let (fills, _state) = run_fetch_fills(&wasm, fixture_state(200, body.into()), empty_cursor())
        .expect("fetch_fills");
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "SBIN-EQ");
    assert_eq!(fills[0].product.as_deref(), Some("1"));
}

#[test]
fn cursor_since_filters_the_tradebook() {
    let wasm = fyers_component_wasm();
    let state = fixture_state(200, read_fyers_fixture("tradebook.json"));

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
fn empty_tradebook_is_zero_fills() {
    let wasm = fyers_component_wasm();
    for body in [
        r#"{"s":"ok","code":200,"tradeBook":[]}"#,
        r#"{"s":"ok","code":200}"#,
    ] {
        let (fills, _state) =
            run_fetch_fills(&wasm, fixture_state(200, body.to_string()), empty_cursor())
                .expect("empty tradeBook is success");
        assert!(fills.is_empty(), "body={body}");
    }
}

#[test]
fn unauthorized_surfaces_as_session_expired() {
    let wasm = fyers_component_wasm();
    let err = run_fetch_fills(
        &wasm,
        fixture_state(
            401,
            r#"{"s":"error","code":-8,"message":"Invalid token"}"#.to_string(),
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
    let wasm = fyers_component_wasm();
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
    let wasm = fyers_component_wasm();
    let (body, _state) = run_describe(&wasm, fixture_state(200, "{}".into())).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("describe json");
    let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
    assert!(
        !implemented.iter().any(|v| v.as_str() == Some("history")),
        "describe must not claim history: {body}"
    );
    assert_eq!(
        json["manifest_id"].as_str(),
        Some("tradeautopsy:fyers-cash@0.1.0")
    );
}
