//! W0.5 Binance realign audit pins (2026-09-23 IST).
//!
//! Docs+tests only: Nautilus `binance/src/spot/http` is a citation oracle (never
//! copied, never a dependency — `rg nautilus agent/Cargo.toml` is empty). These tests
//! pin Station behavior where the audit compared Station against oracle leads:
//! quote-currency honesty (oracle carries explicit `quote_asset`, Station derives by
//! suffix), unpinned-`from_id` handling (oracle passes it through, Station drops it
//! because `fromId` is per-symbol), and the `fromId`-stepping pagination loop the
//! oracle does not have (single calls only).

mod ubi_support;

use std::collections::HashMap;
use tradeautopsy_agent::{
    run_fetch_fills, BrokerHttpFixture, FillCursor, UbiHostConfig, UbiHostState,
    BINANCE_COM_SPOT_BOOK_ID,
};
use ubi_support::{
    assert_component_never_saw_secrets, component_wasm, read_fixture, sentinel_hmac,
};

const ACCOUNT_PATH: &str = "/api/v3/account";
const MY_TRADES_PATH: &str = "/api/v3/myTrades";

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-com-realign".into(),
        broker_slug: "binance_com".into(),
        book_id: BINANCE_COM_SPOT_BOOK_ID.into(),
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

#[test]
fn realign_unknown_quote_reports_unknown_not_usd() {
    // Oracle lead: Nautilus decodes an explicit quote asset per symbol
    // (exchange-info symbol model). Station derives `currency` by quote suffix,
    // so an unrecognized suffix must surface as UNKNOWN for the desk to flag
    // (B6 §7 honesty rule) — never a silent USD-ish booking.
    let wasm = component_wasm("binance_com");
    let body = r#"[{
        "symbol": "FOOXYZ",
        "id": 7,
        "orderId": 42,
        "price": "1.50000000",
        "qty": "2.00000000",
        "commission": "0.00100000",
        "commissionAsset": "BNB",
        "time": 1700000000000,
        "isBuyer": true
    }]"#;
    let state = fixture_state(vec![(MY_TRADES_PATH, 200, body.to_string())]);

    let (fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: None,
            from_id: None,
            symbol: Some("FOOXYZ".into()),
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "FOOXYZ");
    assert_eq!(fills[0].currency, "UNKNOWN");
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn realign_unpinned_from_id_is_dropped_time_window_used() {
    // Oracle lead: `AccountTradesParams` serializes any provided `fromId`.
    // Station: `fromId` is per-symbol, so without a pinned symbol the cursor's
    // `from_id` is dropped and the since-window is used — never combined
    // (Binance -1128).
    let wasm = component_wasm("binance_com");
    let state = fixture_state(vec![
        (ACCOUNT_PATH, 200, read_fixture("binance_com_account.json")),
        (
            MY_TRADES_PATH,
            200,
            read_fixture("binance_com_my_trades.json"),
        ),
    ]);

    let (_fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(1_699_000_000_000),
            from_id: Some("900".into()),
            symbol: None,
        },
    )
    .expect("fetch_fills");

    assert_eq!(state.calls.len(), 2);
    assert_eq!(state.calls[1].path, MY_TRADES_PATH);
    let query: HashMap<&str, &str> = state.calls[1]
        .query
        .iter()
        .map(|q| (q.name.as_str(), q.value.as_str()))
        .collect();
    assert!(!query.contains_key("fromId"));
    assert_eq!(query.get("startTime"), Some(&"1699000000000"));
    assert_eq!(query.get("endTime"), Some(&"1699086399999"));
}

#[test]
fn realign_from_id_steps_past_full_pages_and_stalls_safe() {
    // Oracle lead: single myTrades calls, no pagination loop. Station steps
    // `fromId=maxId+1` past full (500-row) pages and breaks when the page max
    // stalls. The fixture transport is path-keyed, so the repeated page must
    // stall-break after exactly one stepped re-request.
    let wasm = component_wasm("binance_com");
    let mut rows = String::from("[");
    for id in 1..=500 {
        if id > 1 {
            rows.push(',');
        }
        rows.push_str(&format!(
            r#"{{"symbol":"BTCUSDT","id":{id},"orderId":{id},"price":"1.0","qty":"1.0","commission":"0.0","commissionAsset":"BNB","time":1700000000000,"isBuyer":true}}"#
        ));
    }
    rows.push(']');
    let state = fixture_state(vec![(MY_TRADES_PATH, 200, rows)]);

    let (fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: None,
            from_id: None,
            symbol: Some("BTCUSDT".into()),
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 500);
    assert_eq!(state.calls.len(), 2);
    let first: HashMap<&str, &str> = state.calls[0]
        .query
        .iter()
        .map(|q| (q.name.as_str(), q.value.as_str()))
        .collect();
    assert!(!first.contains_key("fromId"));
    let second: HashMap<&str, &str> = state.calls[1]
        .query
        .iter()
        .map(|q| (q.name.as_str(), q.value.as_str()))
        .collect();
    assert_eq!(second.get("fromId"), Some(&"501"));
    assert_component_never_saw_secrets(&state, &wasm);
}
