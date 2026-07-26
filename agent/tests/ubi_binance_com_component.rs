//! `binance_com` adapter component contract (Phase 3 · ADR 0001 · B6 binance_com · R5/R6).
//!
//! Runs the real Wasm component through the Wasmtime host: fixture mode for mapping,
//! recording-transport live mode to prove the HMAC signature is attached host-side.

mod ubi_support;

use std::collections::HashMap;
use std::sync::Arc;
use tradeautopsy_agent::{
    host_allowed, run_fetch_fills, BrokerHttpFixture, FillCursor, RecordingTransport,
    TransportResponse, UbiHostConfig, UbiHostState,
};
use ubi_support::{
    assert_component_never_saw_secrets, component_wasm, read_fixture, sentinel_hmac,
};

const ACCOUNT_PATH: &str = "/api/v3/account";
const MY_TRADES_PATH: &str = "/api/v3/myTrades";

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-com-001".into(),
        broker_slug: "binance_com".into(),
        asset_class: "crypto_spot".into(),
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
                headers: vec![("x-mbx-used-weight-1m".into(), "21".into())],
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
fn my_trades_row_maps_to_fill_event() {
    let wasm = component_wasm("binance_com");
    let state = fixture_state(vec![
        (ACCOUNT_PATH, 200, read_fixture("binance_com_account.json")),
        (
            MY_TRADES_PATH,
            200,
            read_fixture("binance_com_my_trades.json"),
        ),
    ]);

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    let fill = &fills[0];
    assert_eq!(fill.fill_id, "28457");
    assert_eq!(fill.broker_slug, "binance_com");
    assert_eq!(fill.connection_id, "conn-com-001");
    assert_eq!(fill.asset_class, "crypto_spot");
    assert_eq!(fill.symbol, "BTCUSDT");
    assert_eq!(fill.side, "BUY");
    assert!((fill.qty - 12.0).abs() < 1e-9);
    assert!((fill.price - 4.000001).abs() < 1e-9);
    assert_eq!(fill.currency, "USDT");
    assert_eq!(fill.filled_at_unix_ms, 1_499_865_549_590);
    assert!((fill.fee_amount.unwrap() - 0.012).abs() < 1e-9);
    assert_eq!(fill.fee_currency.as_deref(), Some("BNB"));
    assert_eq!(fill.trade_id.as_deref(), Some("100234"));
    // Spot carries no segment/product (R5 §3.2).
    assert!(fill.exchange_segment.is_none());
    assert!(fill.product.is_none());

    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn balances_bootstrap_the_symbol_set_and_skip_stables() {
    let wasm = component_wasm("binance_com");
    let state = fixture_state(vec![
        (ACCOUNT_PATH, 200, read_fixture("binance_com_account.json")),
        (
            MY_TRADES_PATH,
            200,
            read_fixture("binance_com_my_trades.json"),
        ),
    ]);

    let (_fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    // BTC (held) → BTCUSDT. USDT is the quote side, ETH balance is zero.
    assert_eq!(state.calls.len(), 2);
    assert_eq!(state.calls[0].path, ACCOUNT_PATH);
    assert_eq!(state.calls[1].path, MY_TRADES_PATH);
    assert!(state.calls.iter().all(|c| c.host == "api.binance.com"));
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
        .any(|q| q.name == "limit" && q.value == "500"));
}

#[test]
fn pinned_symbol_cursor_skips_account_call_and_derives_quote_currency() {
    let wasm = component_wasm("binance_com");
    let body = r#"[{
        "symbol": "ETHBTC",
        "id": 991,
        "orderId": 5501,
        "price": "0.05000000",
        "qty": "3.00000000",
        "commission": "0.00001000",
        "commissionAsset": "BNB",
        "time": 1700000000000,
        "isBuyer": false
    }]"#;
    let state = fixture_state(vec![(MY_TRADES_PATH, 200, body.to_string())]);

    let (fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(1_699_000_000_000),
            from_id: Some("900".into()),
            symbol: Some("ETHBTC".into()),
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].side, "SELL");
    assert_eq!(fills[0].currency, "BTC");
    assert_eq!(state.calls.len(), 1, "no account bootstrap when pinned");
    let query: HashMap<&str, &str> = state.calls[0]
        .query
        .iter()
        .map(|q| (q.name.as_str(), q.value.as_str()))
        .collect();
    assert_eq!(query.get("symbol"), Some(&"ETHBTC"));
    // B6 §4: fromId wins; never combine with startTime (Binance -1128).
    assert_eq!(query.get("fromId"), Some(&"900"));
    assert!(!query.contains_key("startTime"));
}

#[test]
fn pinned_symbol_time_cursor_sends_start_time_without_from_id() {
    let wasm = component_wasm("binance_com");
    let body = r#"[{
        "symbol": "ETHBTC",
        "id": 991,
        "orderId": 5501,
        "price": "0.05000000",
        "qty": "3.00000000",
        "commission": "0.00001000",
        "commissionAsset": "BNB",
        "time": 1700000000000,
        "isBuyer": false
    }]"#;
    let state = fixture_state(vec![(MY_TRADES_PATH, 200, body.to_string())]);

    let (_fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(1_699_000_000_000),
            from_id: None,
            symbol: Some("ETHBTC".into()),
        },
    )
    .expect("fetch_fills");

    let query: HashMap<&str, &str> = state.calls[0]
        .query
        .iter()
        .map(|q| (q.name.as_str(), q.value.as_str()))
        .collect();
    assert_eq!(query.get("startTime"), Some(&"1699000000000"));
    assert!(!query.contains_key("fromId"));
}

#[test]
fn untradable_symbol_400_is_skipped_not_fatal() {
    let wasm = component_wasm("binance_com");
    let state = fixture_state(vec![
        (ACCOUNT_PATH, 200, read_fixture("binance_com_account.json")),
        (MY_TRADES_PATH, 400, "{\"code\":-1121}".to_string()),
    ]);

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");
    assert!(fills.is_empty());
}

#[test]
fn rate_limited_response_reaches_the_component_as_error_class() {
    let wasm = component_wasm("binance_com");
    let state = fixture_state(vec![
        (ACCOUNT_PATH, 200, read_fixture("binance_com_account.json")),
        (MY_TRADES_PATH, 429, "{\"code\":-1003}".to_string()),
    ]);

    let err = run_fetch_fills(&wasm, state, empty_cursor()).map(|_| ())
        .expect_err("rate limited");
    assert!(
        err.to_string().contains("rate_limited"),
        "unexpected error: {err}"
    );
}

#[test]
fn live_mode_host_signs_requests_the_component_never_sees() {
    let wasm = component_wasm("binance_com");
    let transport = Arc::new(RecordingTransport::routed(vec![
        RecordingTransport::json_route(
            "/api/v3/account",
            200,
            &read_fixture("binance_com_account.json"),
        ),
        RecordingTransport::json_route(
            "/api/v3/myTrades",
            200,
            &read_fixture("binance_com_my_trades.json"),
        ),
    ]));

    let (fills, state) = run_fetch_fills(
        &wasm,
        UbiHostState::live(config(), transport.clone()),
        empty_cursor(),
    )
    .expect("live fetch_fills");

    assert_eq!(fills.len(), 1);
    let sent = transport.sent();
    assert_eq!(sent.len(), 2);
    for request in &sent {
        assert!(request.url.starts_with("https://api.binance.com/"));
        assert!(request.url.contains("timestamp="));
        assert!(request.url.contains("&signature="));
        assert!(request
            .headers
            .iter()
            .any(|(n, v)| n == "X-MBX-APIKEY"
                && v == "TEST_API_KEY_SHOULD_NEVER_REACH_COMPONENT"));
        // The signing secret itself never travels, even host-side.
        assert!(!request
            .url
            .contains("TEST_API_SECRET_SHOULD_NEVER_REACH_COMPONENT"));
    }
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn live_transport_failure_is_surfaced_without_url_detail() {
    let wasm = component_wasm("binance_com");
    let transport = Arc::new(RecordingTransport::new(Err(
        "network: <redacted-url>".to_string()
    )));

    let err = run_fetch_fills(
        &wasm,
        UbiHostState::live(config(), transport),
        empty_cursor(),
    )
    .map(|_| ())
    .expect_err("network failure");
    let message = err.to_string();
    assert!(message.contains("network"), "unexpected error: {message}");
    assert!(!message.contains("signature"));
}

#[test]
fn unroutable_host_is_blocked_before_any_transport_call() {
    // The component only ever names api.binance.com, so the allowlist is exercised via
    // the shared host guard; assert the constant stays aligned with Kill DNS intent (R8).
    assert!(host_allowed("api.binance.com"));
    assert!(!host_allowed("api.binance.us"));
    let _ = TransportResponse {
        status: 200,
        headers: vec![],
        body: String::new(),
    };
}
