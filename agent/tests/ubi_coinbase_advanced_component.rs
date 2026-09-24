//! `coinbase_advanced` adapter component contract (Phase 4 · ADR 0018 · B6 · R5/R6).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use tradeautopsy_agent::{
    run_describe, run_fetch_fills, run_obtain, AssetClass, BrokerHttpFixture, FillCursor,
    HostCredentialBlob, InstrumentClass, UbiHostConfig, UbiHostState, WitAssetClass,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm_from_crate};

const FILLS_PATH: &str = "/api/v3/brokerage/orders/historical/fills";
const COINBASE_ADVANCED_SPOT_BOOK_ID: &str = "coinbase-advanced-spot";

fn coinbase_component_wasm() -> PathBuf {
    component_wasm_from_crate(
        "ubi-coinbase-advanced-adapter",
        "ubi_coinbase_advanced_adapter.wasm",
    )
}

fn read_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/coinbase_advanced")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-coinbase-001".into(),
        broker_slug: "coinbase_advanced".into(),
        book_id: COINBASE_ADVANCED_SPOT_BOOK_ID.into(),
        asset_class: AssetClass::Cryptocurrency,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: HostCredentialBlob::CoinbaseJwt {
            api_key: "organizations/test/apiKeys/test-key".into(),
            pem_private_key: "-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIBdummy\n-----END EC PRIVATE KEY-----\n".into(),
        },
    }
}

fn fills_fixture_state() -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        FILLS_PATH.to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_fixture("fills_page.json"),
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
fn describe_manifest_matches_book() {
    let wasm = coinbase_component_wasm();
    let (body, state) = run_describe(&wasm, fills_fixture_state()).expect("describe");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        json.get("manifest_id").and_then(|v| v.as_str()),
        Some("tradeautopsy:coinbase-advanced-spot@0.1.0")
    );
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn fetch_fills_maps_usd_pairs_and_stamps_identity() {
    let wasm = coinbase_component_wasm();
    let (fills, state) =
        run_fetch_fills(&wasm, fills_fixture_state(), empty_cursor()).expect("fetch");
    assert_eq!(fills.len(), 2);
    let first = &fills[0];
    assert_eq!(first.broker_slug, "coinbase_advanced");
    assert_eq!(first.connection_id, "conn-coinbase-001");
    assert_eq!(first.currency, "USD");
    assert_eq!(WitAssetClass::from(first.asset_class), WitAssetClass::Cryptocurrency);
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn obtain_history_is_unsupported() {
    let wasm = coinbase_component_wasm();
    let request = serde_json::json!({
        "family": "market",
        "capability-id": "ohlcv",
        "physics": "historical_series",
        "operation": "history",
    })
    .to_string();
    let (body, state) = run_obtain(&wasm, fills_fixture_state(), &request).expect("obtain");
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["status"], "unsupported");
    assert!(state.calls.is_empty());
}
