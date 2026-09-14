//! `kotak_neo` adapter component contract (Phase 3 · ADR 0001 · B6 kotak_neo · R5/R6).
//!
//! Day-book mapping, cash-only refuse list, IST timestamp parsing, and proof that the
//! session token / Sid are attached by the Enforcer rather than the component.

mod ubi_support;

use std::collections::HashMap;
use std::sync::Arc;
use tradeautopsy_agent::{
    run_describe, run_fetch_fills, run_obtain, BrokerHttpFixture, FillCursor, RecordingTransport,
    UbiHostConfig, UbiHostState, KOTAK_NSE_BSE_CASH_BOOK_ID,
};
use ubi_support::{
    assert_component_never_saw_secrets, component_wasm, read_fixture, sentinel_kotak_session,
};

const TRADES_PATH: &str = "/quick/user/trades";
/// 25-07-2026 10:15:30 IST → 04:45:30 UTC.
const ITBEES_FILLED_AT_MS: i64 = 1_784_954_730_000;
/// 25-07-2026 11:00:00 IST → 05:30:00 UTC.
const NFO_FILLED_AT_MS: i64 = 1_784_957_400_000;
/// 25-07-2026 14:05:00 IST → 08:35:00 UTC.
const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-kotak-001".into(),
        broker_slug: "kotak_neo".into(),
        book_id: KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
        asset_class: "equities".into(),
        credentials: sentinel_kotak_session(),
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
fn trade_book_rows_map_to_fill_events() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, read_fixture("kotak_neo_trade_book.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(
        fills.len(),
        3,
        "cash CNC/MIS plus nse_fo NRML; cash CO dropped"
    );
    let itbees = &fills[0];
    assert_eq!(itbees.fill_id, "FILL-1");
    assert_eq!(itbees.broker_slug, "kotak_neo");
    assert_eq!(itbees.connection_id, "conn-kotak-001");
    assert_eq!(itbees.asset_class, "equities");
    assert_eq!(itbees.symbol, "ITBEES");
    assert_eq!(itbees.side, "BUY");
    assert!((itbees.qty - 100.0).abs() < 1e-9);
    assert!((itbees.price - 25.50).abs() < 1e-9);
    assert_eq!(itbees.currency, "INR");
    assert_eq!(itbees.exchange_segment.as_deref(), Some("nse_cm"));
    assert_eq!(itbees.product.as_deref(), Some("CNC"));
    assert_eq!(itbees.trade_id.as_deref(), Some("NSE123456"));
    assert_eq!(itbees.filled_at_unix_ms, ITBEES_FILLED_AT_MS);
    // Trade book does not itemise statutory charges — no invented fee (B6 §7).
    assert!(itbees.fee_amount.is_none());
    assert!(itbees.fee_currency.is_none());

    let reliance = &fills[2];
    assert_eq!(reliance.symbol, "RELIANCE");
    assert_eq!(reliance.side, "SELL");
    assert_eq!(reliance.product.as_deref(), Some("MIS"));
    assert_eq!(reliance.filled_at_unix_ms, RELIANCE_FILLED_AT_MS);
    // No flId on this row: falls back to the exchange order id.
    assert_eq!(reliance.fill_id, "NSE998877");

    let nfo = &fills[1];
    assert_eq!(nfo.symbol, "NIFTY25JUL24000CE");
    assert_eq!(nfo.exchange_segment.as_deref(), Some("nse_fo"));
    assert_eq!(nfo.product.as_deref(), Some("NRML"));
    assert_eq!(nfo.currency, "INR");
    assert_eq!(nfo.side, "BUY");
    assert_ne!(nfo.product.as_deref(), Some("MIS"));
    assert_ne!(nfo.product.as_deref(), Some("CNC"));
    assert_eq!(nfo.filled_at_unix_ms, NFO_FILLED_AT_MS);

    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn nse_fo_rows_are_nfo_fills_not_cash() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, read_fixture("kotak_neo_trade_book.json"));

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    let cash: Vec<_> = fills
        .iter()
        .filter(|f| f.exchange_segment.as_deref() == Some("nse_cm"))
        .collect();
    let nfo: Vec<_> = fills
        .iter()
        .filter(|f| f.exchange_segment.as_deref() == Some("nse_fo"))
        .collect();
    assert_eq!(cash.len(), 2);
    assert!(cash
        .iter()
        .all(|f| matches!(f.product.as_deref(), Some("CNC") | Some("MIS"))));
    assert_eq!(nfo.len(), 1);
    assert_eq!(nfo[0].product.as_deref(), Some("NRML"));
    assert_eq!(nfo[0].symbol, "NIFTY25JUL24000CE");
    assert_eq!(nfo[0].currency, "INR");
    assert!(!fills.iter().any(|f| f.symbol == "TCS"));
}

#[test]
fn cursor_since_filters_the_day_book() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, read_fixture("kotak_neo_trade_book.json"));

    let (fills, _state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(NFO_FILLED_AT_MS + 1),
            from_id: None,
            symbol: None,
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "RELIANCE");
}

/// Live quiet-day body (founder dogfood 2026-08-31): HTTP 200, no `data` key.
/// SESSION-AND-TRADES.md: empty filtered day book is success — not `missing data[]`.
#[test]
fn empty_day_book_without_data_array_is_zero_fills() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, r#"{"stat":"Ok","stCode":200}"#.to_string());

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor())
        .expect("empty day book is success, not missing data[]");
    assert!(fills.is_empty());
}

#[test]
fn empty_day_book_null_or_empty_data_is_zero_fills() {
    let wasm = component_wasm("kotak_neo");
    for body in [
        // Official Trade_report.md sample uses lowercase `ok`.
        r#"{"stat":"ok","stCode":200}"#,
        r#"{"stat":"Ok","stCode":200,"data":null}"#,
        r#"{"stat":"Ok","stCode":200,"data":[]}"#,
    ] {
        let (fills, _state) =
            run_fetch_fills(&wasm, fixture_state(200, body.to_string()), empty_cursor())
                .expect("empty data is success");
        assert!(fills.is_empty(), "body={body}");
    }
}

/// HTTP 200 + `stat: Not_Ok` is a venue error (Trade_report.md 400/403 table via body).
/// Must not look like a quiet day just because `data` is absent.
#[test]
fn not_ok_without_data_is_not_an_empty_day_book() {
    let wasm = component_wasm("kotak_neo");
    let err = run_fetch_fills(
        &wasm,
        fixture_state(
            200,
            r#"{"stat":"Not_Ok","stCode":400,"errMsg":"Invalid or missing input parameters"}"#
                .to_string(),
        ),
        empty_cursor(),
    )
    .map(|_| ())
    .expect_err("Not_Ok is not empty success");
    let msg = err.to_string();
    assert!(
        msg.contains("not_ok") || msg.contains("Not_Ok"),
        "unexpected error: {msg}"
    );
    assert!(!msg.contains("missing data[]"));
}

#[test]
fn dead_session_surfaces_as_session_expired() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(
        200,
        r#"{"stat":"Not_Ok","stCode":1003,"errMsg":"Invalid Session"}"#.to_string(),
    );

    let err = run_fetch_fills(&wasm, state, empty_cursor())
        .map(|_| ())
        .expect_err("session expired");
    assert!(
        err.to_string().contains("session_expired"),
        "unexpected error: {err}"
    );
}

#[test]
fn live_mode_host_attaches_session_headers_and_blob_base_url() {
    let wasm = component_wasm("kotak_neo");
    let transport = Arc::new(RecordingTransport::routed(vec![
        RecordingTransport::json_route(
            "/quick/user/trades",
            200,
            &read_fixture("kotak_neo_trade_book.json"),
        ),
    ]));

    let (fills, state) = run_fetch_fills(
        &wasm,
        UbiHostState::live(config(), transport.clone()),
        empty_cursor(),
    )
    .expect("live fetch_fills");

    assert_eq!(fills.len(), 3);
    let sent = transport.last().expect("request sent");
    // Base URL (including its /trading prefix) comes from the credential blob, not Wasm.
    // Host attaches trade-book `sId` = hsServerId outside the component (SDK TradeReportAPI).
    assert_eq!(
        sent.url,
        "https://cis.kotaksecurities.com/trading/quick/user/trades?sId=TEST_HS_SERVER_NEVER_IN_COMPONENT"
    );
    assert!(sent
        .headers
        .iter()
        .any(|(n, v)| n == "Auth" && v == "TEST_TRADE_TOKEN_NEVER_IN_COMPONENT"));
    assert!(sent
        .headers
        .iter()
        .any(|(n, v)| n == "Sid" && v == "TEST_SID_NEVER_IN_COMPONENT"));
    assert!(state.calls[0].headers.is_empty());
    assert_component_never_saw_secrets(&state, &wasm);
}

fn history_obtain_request() -> String {
    serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "instrument-id": "RELIANCE",
        "product-use": "research",
        "operation": "history",
    })
    .to_string()
}

#[test]
fn obtain_history_is_unsupported_not_empty_success() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, read_fixture("kotak_neo_trade_book.json"));

    let (body, state) = run_obtain(&wasm, state, &history_obtain_request()).expect("obtain json");
    let json: serde_json::Value = serde_json::from_str(&body).expect("obtain json");
    assert_eq!(json["status"], "unsupported");
    assert_ne!(json["status"], "success");
    assert!(json.get("candles").is_none());
    assert!(json.get("data").is_none() || json["data"].is_null());
    assert!(
        state.calls.is_empty(),
        "kotak history must not call the host network"
    );
    assert!(state
        .calls
        .iter()
        .all(|c| c.path != "/api/v3/klines" && !c.path.contains("klines")));
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn describe_does_not_claim_history() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, read_fixture("kotak_neo_trade_book.json"));

    let (body, _state) = run_describe(&wasm, state).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("describe json");
    let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
    assert!(
        !implemented.iter().any(|v| v.as_str() == Some("history")),
        "kotak describe must not claim history: {body}"
    );
    let bindings = json["bindings"].as_array().cloned().unwrap_or_default();
    assert!(bindings.iter().all(|b| b["operation"] != "history"));
}
