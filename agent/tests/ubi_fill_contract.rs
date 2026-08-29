//! UBI Phase 1 — FillEvent contract tests (ADR 0001 / R5 / R6).
//!
//! Loads the fixture Wasm component via Wasmtime host stub.
//! No live broker keys. Host mediates HTTP fixtures; credentials never enter component responses.
//!
//! Refs:
//! - docs/adr/0001-uniform-wasm-sandboxed-broker-adapters.md
//! - /Users/bishnu/issues/brokers/research/parts/R5-fill-model-gap.md
//! - /Users/bishnu/issues/brokers/research/parts/R6-credentials-start.md

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tradeautopsy_agent::{
    host_allowed, run_fetch_fills, BrokerHttpFixture, FillCursor, HostCredentialBlob,
    UbiHostConfig, UbiHostState, BINANCE_COM_SPOT_BOOK_ID, FORBIDDEN_COMPONENT_HEADERS,
    KOTAK_NSE_BSE_CASH_BOOK_ID,
};

fn workspace_agent_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_wasm_path() -> PathBuf {
    workspace_agent_dir()
        .join("ubi-fixture-adapter/target/wasm32-wasip2/release/ubi_fixture_adapter.wasm")
}

fn ensure_fixture_wasm() -> PathBuf {
    let wasm = fixture_wasm_path();
    if wasm.is_file() {
        return wasm;
    }
    let manifest = workspace_agent_dir().join("ubi-fixture-adapter/Cargo.toml");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--manifest-path",
        ])
        .arg(&manifest)
        .status()
        .expect("spawn cargo build for ubi-fixture-adapter");
    assert!(
        status.success(),
        "failed to build ubi-fixture-adapter.wasm (is wasm32-wasip2 installed?)"
    );
    assert!(
        wasm.is_file(),
        "expected fixture wasm at {}",
        wasm.display()
    );
    wasm
}

fn read_fixture_json(name: &str) -> String {
    let path = workspace_agent_dir().join("fixtures/ubi").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn secret_blob() -> HostCredentialBlob {
    HostCredentialBlob::hmac(
        "TEST_API_KEY_SHOULD_NEVER_REACH_COMPONENT",
        "TEST_API_SECRET_SHOULD_NEVER_REACH_COMPONENT",
    )
}

fn com_host_state() -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        "/api/v3/myTrades".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_fixture_json("binance_com_my_trades.json"),
            headers: vec![("x-mbx-used-weight-1m".into(), "1".into())],
            error_class: None,
        },
    );
    UbiHostState::new(
        UbiHostConfig {
            connection_id: "conn-com-001".into(),
            broker_slug: "binance_com".into(),
            book_id: BINANCE_COM_SPOT_BOOK_ID.into(),
            asset_class: "crypto_spot".into(),
            credentials: secret_blob(),
        },
        fixtures,
    )
}

fn kotak_host_state() -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        "/quick/user/trades".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_fixture_json("kotak_neo_trades.json"),
            headers: vec![],
            error_class: None,
        },
    );
    UbiHostState::new(
        UbiHostConfig {
            connection_id: "conn-kotak-001".into(),
            broker_slug: "kotak_neo".into(),
            book_id: KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
            asset_class: "equities".into(),
            credentials: secret_blob(),
        },
        fixtures,
    )
}

#[test]
fn com_shaped_json_fixture_maps_to_fill_event() {
    let wasm = ensure_fixture_wasm();
    let (fills, state) = run_fetch_fills(
        &wasm,
        com_host_state(),
        FillCursor {
            since_unix_ms: None,
            from_id: Some("binance_com".into()),
            symbol: Some("BTCUSDT".into()),
        },
    )
    .expect("fetch_fills com");

    assert_eq!(fills.len(), 1);
    let f = &fills[0];
    assert_eq!(f.fill_id, "28457");
    assert_eq!(f.broker_slug, "binance_com");
    assert_eq!(f.connection_id, "conn-com-001");
    assert_eq!(f.asset_class, "crypto_spot");
    assert_eq!(f.symbol, "BTCUSDT");
    assert_eq!(f.side, "BUY");
    assert!((f.qty - 12.0).abs() < 1e-9);
    assert!((f.price - 4.000001).abs() < 1e-9);
    assert_eq!(f.currency, "USDT");
    assert_eq!(f.filled_at_unix_ms, 1_499_865_549_590);
    assert!((f.fee_amount.unwrap() - 0.012).abs() < 1e-9);
    assert_eq!(f.fee_currency.as_deref(), Some("BNB"));
    assert!(f.exchange_segment.is_none());
    assert!(f.product.is_none());

    assert_eq!(state.calls.len(), 1);
    assert_eq!(state.calls[0].host, "api.binance.com");
    assert_eq!(state.calls[0].path, "/api/v3/myTrades");
    assert!(host_allowed(&state.calls[0].host));
    assert_secrets_never_reached_component(&state, &wasm);
}

#[test]
fn kotak_shaped_json_fixture_maps_to_fill_event() {
    let wasm = ensure_fixture_wasm();
    let (fills, state) = run_fetch_fills(
        &wasm,
        kotak_host_state(),
        FillCursor {
            since_unix_ms: None,
            from_id: Some("kotak_neo".into()),
            symbol: None,
        },
    )
    .expect("fetch_fills kotak");

    assert_eq!(fills.len(), 1);
    let f = &fills[0];
    assert_eq!(f.broker_slug, "kotak_neo");
    assert_eq!(f.connection_id, "conn-kotak-001");
    assert_eq!(f.asset_class, "equities");
    assert_eq!(f.symbol, "ITBEES");
    assert_eq!(f.side, "BUY");
    assert!((f.qty - 100.0).abs() < 1e-9);
    assert!((f.price - 25.50).abs() < 1e-9);
    assert_eq!(f.currency, "INR");
    assert_eq!(f.exchange_segment.as_deref(), Some("nse_cm"));
    assert_eq!(f.product.as_deref(), Some("CNC"));
    assert_eq!(f.trade_id.as_deref(), Some("NSE123456"));

    assert_eq!(state.calls.len(), 1);
    assert_eq!(state.calls[0].host, "cis.kotaksecurities.com");
    assert_eq!(state.calls[0].path, "/quick/user/trades");
    assert_secrets_never_reached_component(&state, &wasm);
}

#[test]
fn host_allowlist_and_forbidden_headers_are_enforced_constants() {
    assert!(!host_allowed("evil.example.com"));
    assert!(host_allowed("api.binance.com"));
    assert!(host_allowed("cis.kotaksecurities.com"));
    assert!(FORBIDDEN_COMPONENT_HEADERS.contains(&"x-mbx-apikey"));
    assert!(FORBIDDEN_COMPONENT_HEADERS.contains(&"auth"));
    assert!(FORBIDDEN_COMPONENT_HEADERS.contains(&"sid"));
    // Ensure fixture wasm is buildable in this environment.
    let _ = ensure_fixture_wasm();
}

/// Proof: raw credentials never appear on the component data channel (R6).
fn assert_secrets_never_reached_component(state: &UbiHostState, component_path: &Path) {
    let secrets = state.config.credentials.secret_values();
    let key = secrets[0];
    let secret = secrets[1];
    assert!(!state.credential_leak_attempted);

    // Responses recorded indirectly: fixture bodies on disk must not embed test secrets.
    let com = read_fixture_json("binance_com_my_trades.json");
    let kotak = read_fixture_json("kotak_neo_trades.json");
    assert!(!com.contains(key) && !com.contains(secret));
    assert!(!kotak.contains(key) && !kotak.contains(secret));

    // Wasm binary must not contain the raw credential strings (never linked in).
    let bytes = std::fs::read(component_path).expect("read wasm");
    let as_latin1 = String::from_utf8_lossy(&bytes);
    assert!(
        !as_latin1.contains(key),
        "api_key found inside component wasm bytes"
    );
    assert!(
        !as_latin1.contains(secret),
        "api_secret found inside component wasm bytes"
    );

    // Host call log from component must not include forbidden auth headers.
    for call in &state.calls {
        for h in &call.headers {
            assert!(
                !FORBIDDEN_COMPONENT_HEADERS
                    .iter()
                    .any(|f| h.name.eq_ignore_ascii_case(f)),
                "component sent forbidden header {}",
                h.name
            );
            assert!(!h.value.contains(key) && !h.value.contains(secret));
        }
        for q in &call.query {
            assert!(!q.value.contains(key) && !q.value.contains(secret));
        }
    }
}
