//! `kraken` adapter component contract (Phase 4 · ADR 0017 · B6 kraken · R5/R6).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use tradeautopsy_agent::{
    prepare_request, run_describe, run_fetch_fills, run_obtain, AssetClass, BrokerHttpFixture,
    FillCursor, HostCredentialBlob, InstrumentClass, UbiHostConfig, UbiHostState, WitAssetClass,
    KRAKEN_API_HOST, KRAKEN_BOOK_ID,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm_from_crate};

const TRADES_HISTORY_PATH: &str = "/0/private/TradesHistory";

fn kraken_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-kraken-adapter", "ubi_kraken_adapter.wasm")
}

fn read_kraken_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/kraken")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn kraken_hmac_creds() -> HostCredentialBlob {
    HostCredentialBlob::hmac(
        "TEST_KRAKEN_KEY_NEVER_IN_COMPONENT",
        "c2VjcmV0", // base64("secret") — Kraken secrets are base64 on the wire
    )
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-kraken-001".into(),
        broker_slug: "kraken".into(),
        book_id: KRAKEN_BOOK_ID.into(),
        asset_class: AssetClass::Cryptocurrency,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: kraken_hmac_creds(),
    }
}

fn fixture_state(body: &str) -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        TRADES_HISTORY_PATH.to_string(),
        BrokerHttpFixture {
            status: 200,
            body: body.to_string(),
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
fn trades_history_maps_desk_quote_pairs() {
    let wasm = kraken_wasm();
    let state = fixture_state(&read_kraken_fixture("trades_history.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 2);
    let btc = &fills[0];
    assert_eq!(btc.fill_id, "TCCCTY-WJ6EO-4FP4VZ");
    assert_eq!(btc.broker_slug, "kraken");
    assert_eq!(btc.connection_id, "conn-kraken-001");
    assert_eq!(btc.asset_class, WitAssetClass::Cryptocurrency);
    assert_eq!(btc.symbol, "XBTUSDT");
    assert_eq!(btc.side, "BUY");
    assert_eq!(btc.currency, "USDT");
    assert!((btc.qty - 0.5).abs() < 1e-9);
    assert!((btc.price - 29500.50).abs() < 1e-9);
    assert_eq!(btc.filled_at_unix_ms, 1_688_585_840_892);
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn quote_filter_skips_non_desk_pairs_without_failing_sync() {
    let wasm = kraken_wasm();
    let state = fixture_state(&read_kraken_fixture("trades_history.json"));

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert!(fills.iter().all(|f| f.symbol != "ETHXBT"));
}

#[test]
fn host_attaches_kraken_api_sign_not_binance_query() {
    let creds = kraken_hmac_creds();
    let prepared = prepare_request(
        "POST",
        KRAKEN_API_HOST,
        TRADES_HISTORY_PATH,
        &[],
        &[],
        Some("type=trade&ofs=0"),
        &creds,
        1_700_000_000_000,
    );
    assert!(!prepared.url.contains("signature="));
    assert!(prepared.headers.iter().any(|(n, _)| n == "API-Sign"));
    assert!(prepared
        .headers
        .iter()
        .any(|(n, v)| n == "API-Key" && v.contains("TEST_KRAKEN_KEY")));
    assert!(prepared.body.as_ref().is_some_and(|b| b.starts_with("nonce=")));
}

#[test]
fn kraken_error_array_surfaces_as_error() {
    let wasm = kraken_wasm();
    let body = r#"{"error":["EGeneral:Invalid arguments"],"result":{}}"#;
    let state = fixture_state(body);

    let err = run_fetch_fills(&wasm, state, empty_cursor())
        .map(|_| ())
        .expect_err("error array");
    assert!(err.to_string().contains("Invalid arguments"));
}

#[test]
fn cursor_since_filters_trades_client_side() {
    let wasm = kraken_wasm();
    let state = fixture_state(&read_kraken_fixture("trades_history.json"));

    let (fills, _state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: Some(1_688_586_440_000),
            from_id: None,
            symbol: None,
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "ETHUSDT");
}

#[test]
fn pinned_symbol_filters_pair() {
    let wasm = kraken_wasm();
    let state = fixture_state(&read_kraken_fixture("trades_history.json"));

    let (fills, state) = run_fetch_fills(
        &wasm,
        state,
        FillCursor {
            since_unix_ms: None,
            from_id: None,
            symbol: Some("XBTUSDT".into()),
        },
    )
    .expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, "XBTUSDT");
    assert_eq!(state.calls.len(), 1);
    assert_eq!(state.calls[0].path, TRADES_HISTORY_PATH);
    assert_eq!(state.calls[0].method, "POST");
}

#[test]
fn describe_claims_tradebook_only() {
    let wasm = kraken_wasm();
    let (body, _state) = run_describe(&wasm, fixture_state("{}")).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["manifest_id"].as_str(), Some("kraken.spot.v1"));
    let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
    assert_eq!(implemented.len(), 1);
    assert_eq!(implemented[0].as_str(), Some("tradebook"));

    let request = serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
    })
    .to_string();
    let err = run_obtain(&wasm, fixture_state("{}"), &request)
        .map(|_| ())
        .expect_err("unsupported obtain");
    assert!(err.to_string().contains("unsupported"));
}
