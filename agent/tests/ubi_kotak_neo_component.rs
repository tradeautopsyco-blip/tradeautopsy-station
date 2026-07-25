//! `kotak_neo` adapter component contract (Phase 3 · ADR 0001 · B6 kotak_neo · R5/R6).
//!
//! Day-book mapping, cash-only refuse list, IST timestamp parsing, and proof that the
//! session token / Sid are attached by the Enforcer rather than the component.

mod ubi_support;

use std::collections::HashMap;
use std::sync::Arc;
use tradeautopsy_agent::{
    run_fetch_fills, BrokerHttpFixture, FillCursor, RecordingTransport, UbiHostConfig, UbiHostState,
};
use ubi_support::{
    assert_component_never_saw_secrets, component_wasm, read_fixture, sentinel_kotak_session,
};

const TRADES_PATH: &str = "/quick/user/trades";
/// 25-07-2026 10:15:30 IST → 04:45:30 UTC.
const ITBEES_FILLED_AT_MS: i64 = 1_784_954_730_000;
/// 25-07-2026 14:05:00 IST → 08:35:00 UTC.
const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-kotak-001".into(),
        broker_slug: "kotak_neo".into(),
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

    assert_eq!(fills.len(), 2, "only nse_cm CNC/MIS rows are ingested");
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

    let reliance = &fills[1];
    assert_eq!(reliance.symbol, "RELIANCE");
    assert_eq!(reliance.side, "SELL");
    assert_eq!(reliance.product.as_deref(), Some("MIS"));
    assert_eq!(reliance.filled_at_unix_ms, RELIANCE_FILLED_AT_MS);
    // No flId on this row: falls back to the exchange order id.
    assert_eq!(reliance.fill_id, "NSE998877");

    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn fno_segments_and_refused_products_are_dropped() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, read_fixture("kotak_neo_trade_book.json"));

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    // B6 §12: nse_fo/NRML and CO rows must not appear as equities cash fills.
    assert!(fills.iter().all(|f| f.exchange_segment.as_deref() == Some("nse_cm")));
    assert!(fills
        .iter()
        .all(|f| matches!(f.product.as_deref(), Some("CNC") | Some("MIS"))));
    assert!(!fills.iter().any(|f| f.symbol.contains("NIFTY")));
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
            since_unix_ms: Some(ITBEES_FILLED_AT_MS + 1),
            from_id: None,
            symbol: None,
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "RELIANCE");
}

#[test]
fn dead_session_surfaces_as_session_expired() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(
        200,
        r#"{"stat":"Not_Ok","stCode":1003,"errMsg":"Invalid Session"}"#.to_string(),
    );

    let err = run_fetch_fills(&wasm, state, empty_cursor()).map(|_| ()).expect_err("session expired");
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

    assert_eq!(fills.len(), 2);
    let sent = transport.last().expect("request sent");
    // Base URL (including its /trading prefix) comes from the credential blob, not Wasm.
    assert_eq!(
        sent.url,
        "https://cis.kotaksecurities.com/trading/quick/user/trades"
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
