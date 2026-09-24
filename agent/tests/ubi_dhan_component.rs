//! `dhan` adapter component contract (Phase 3 · ADR 0001 · B6 dhan · R5/R6).

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

const TRADES_PATH: &str = "/v2/trades";
const DHAN_NSE_BSE_CASH_BOOK_ID: &str = "dhan-nse-bse-cash";
/// 2026-07-25 10:15:30 IST → 04:45:30 UTC.
const ITBEES_FILLED_AT_MS: i64 = 1_784_954_730_000;
/// 2026-07-25 12:00:00 IST → 06:30:00 UTC.
const SBIN_FILLED_AT_MS: i64 = 1_784_961_000_000;
/// 2026-07-25 14:05:00 IST → 08:35:00 UTC.
const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;
/// 2026-07-25 15:30:00 IST → 10:00:00 UTC.
const INFY_FILLED_AT_MS: i64 = 1_784_973_600_000;

fn dhan_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-dhan-adapter", "ubi_dhan_adapter.wasm")
}

fn read_dhan_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/dhan")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-dhan-001".into(),
        broker_slug: "dhan".into(),
        book_id: DHAN_NSE_BSE_CASH_BOOK_ID.into(),
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
fn obtain_history_is_unsupported_not_empty_success() {
    let wasm = dhan_component_wasm();
    let request = serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "instrument-id": "RELIANCE",
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
    let wasm = dhan_component_wasm();
    let (body, _state) = run_describe(&wasm, fixture_state(200, "{}".into())).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("describe json");
    let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
    assert!(
        !implemented.iter().any(|v| v.as_str() == Some("history")),
        "describe must not claim history: {body}"
    );
    assert_eq!(
        json["manifest_id"].as_str(),
        Some("tradeautopsy:dhan-cash@0.1.0")
    );
}

#[test]
fn trades_day_book_maps_to_fill_events() {
    let wasm = dhan_component_wasm();
    let state = fixture_state(200, read_dhan_fixture("trades_day_book.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(
        fills.len(),
        4,
        "cash CNC/MIS only; MARGIN, NSE_FNO, crossCurrency, IDX_I dropped"
    );
    let itbees = &fills[0];
    assert_eq!(itbees.fill_id, "900001");
    assert_eq!(itbees.broker_slug, "dhan");
    assert_eq!(itbees.connection_id, "conn-dhan-001");
    assert_eq!(itbees.asset_class, WitAssetClass::Equity);
    assert_eq!(itbees.symbol, "ITBEES");
    assert_eq!(itbees.side, "BUY");
    assert!((itbees.qty - 100.0).abs() < 1e-9);
    assert!((itbees.price - 25.5).abs() < 1e-9);
    assert_eq!(itbees.currency, "INR");
    assert_eq!(itbees.exchange_segment.as_deref(), Some("NSE_EQ"));
    assert_eq!(itbees.product.as_deref(), Some("CNC"));
    assert_eq!(itbees.trade_id.as_deref(), Some("100001"));
    assert_eq!(itbees.filled_at_unix_ms, ITBEES_FILLED_AT_MS);
    assert!(itbees.fee_amount.is_none());

    let sbin = &fills[1];
    assert_eq!(sbin.symbol, "SBIN");
    assert_eq!(sbin.exchange_segment.as_deref(), Some("BSE_EQ"));
    assert_eq!(sbin.filled_at_unix_ms, SBIN_FILLED_AT_MS);

    let reliance = &fills[2];
    assert_eq!(reliance.symbol, "RELIANCE");
    assert_eq!(reliance.side, "SELL");
    assert_eq!(reliance.product.as_deref(), Some("MIS"));
    assert_eq!(reliance.filled_at_unix_ms, RELIANCE_FILLED_AT_MS);
    assert_eq!(reliance.fill_id, "900002");

    let infy = &fills[3];
    assert_eq!(infy.symbol, "INFY");
    assert_eq!(infy.fill_id, "100008#7");
    assert_eq!(infy.filled_at_unix_ms, INFY_FILLED_AT_MS);

    assert!(!fills.iter().any(|f| f.symbol == "TCS"
        || f.symbol == "NIFTY26JUL24000CE"
        || f.symbol == "USDINR"
        || f.symbol == "NIFTY 50"));
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn ranged_charge_fields_aggregate_to_fee() {
    // The mapper accepts ranged rows on any body: feed the ranged page through
    // the day-book fixture key to prove charge aggregation end to end.
    let wasm = dhan_component_wasm();
    let state = fixture_state(200, read_dhan_fixture("trades_ranged_page.json"));

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 2);
    assert!((fills[0].fee_amount.unwrap_or(-1.0) - 44.79).abs() < 1e-9);
    assert_eq!(fills[0].fee_currency.as_deref(), Some("INR"));
    assert!((fills[1].fee_amount.unwrap_or(-1.0) - 48.95).abs() < 1e-9);
    assert_eq!(fills[1].fee_currency.as_deref(), Some("INR"));
}

#[test]
fn cursor_since_filters_the_day_book() {
    let wasm = dhan_component_wasm();
    let state = fixture_state(200, read_dhan_fixture("trades_day_book.json"));

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

    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].symbol, "RELIANCE");
    assert_eq!(fills[1].symbol, "INFY");
}

#[test]
fn empty_day_book_is_zero_fills() {
    let wasm = dhan_component_wasm();
    for body in [
        r#"{"status":"success","data":[]}"#,
        r#"{"status":"success","data":null}"#,
        r#"{"status":"success"}"#,
    ] {
        let (fills, _state) =
            run_fetch_fills(&wasm, fixture_state(200, body.to_string()), empty_cursor())
                .expect("empty data is success");
        assert!(fills.is_empty(), "body={body}");
    }
}

#[test]
fn dh901_surfaces_as_session_expired() {
    let wasm = dhan_component_wasm();
    for body in [
        r#"{"errorType":"InvalidToken","errorCode":"DH-901","errorMessage":"Invalid token"}"#,
        r#"{"status":"failure","remarks":{"error_code":"DH-901","error_type":"Auth","error_message":"expired"}}"#,
    ] {
        let err = run_fetch_fills(&wasm, fixture_state(200, body.to_string()), empty_cursor())
            .map(|_| ())
            .expect_err("DH-901");
        assert!(
            err.to_string().contains("session_expired"),
            "body={body} unexpected: {err}"
        );
    }
    let err = run_fetch_fills(
        &wasm,
        fixture_state(401, r#"{"errorType":"Auth","errorCode":807,"errorMessage":"expired"}"#.to_string()),
        empty_cursor(),
    )
    .map(|_| ())
    .expect_err("401");
    assert!(
        err.to_string().contains("session_expired"),
        "unexpected: {err}"
    );
}
