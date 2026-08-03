//! Runtime start selects Wasm components for UBI brokers (Phase 3 · ADR 0001 · B5).
//!
//! B5 lock: Enforcer Start factory (`build_runtime_adapter`) owns live poll/fills for the
//! first pair. Native `binance_com_spot_*.rs` is reference only — never selected for live
//! Start. `binance_com` / `kotak_neo` load Wasm; test/fake prefixes stay CountingPoll.

mod ubi_support;

use serial_test::serial;
use tradeautopsy_agent::{
    build_runtime_adapter, build_wasm_runtime_adapter, component_path_for_slug, uses_wasm_component,
    BinanceComSpotBrokerAdapter, BrokerAdapter, CredentialBlob,
};
use ubi_support::component_wasm;

#[test]
fn enabled_first_party_brokers_run_as_components() {
    assert!(uses_wasm_component("binance_com"));
    assert!(uses_wasm_component("kotak_neo"));
    assert!(!uses_wasm_component("binance_us"));
}

#[test]
fn native_com_adapter_name_is_not_the_live_wasm_name() {
    // Reference module still exists for rebuild notes — must not collide with live SoT name.
    let native = BinanceComSpotBrokerAdapter::new("ref-key", "ref-secret");
    assert_eq!(native.name(), "binance_com_spot");
    assert_ne!(native.name(), "binance_com_wasm");
}

#[test]
#[serial]
fn binance_com_start_loads_the_wasm_adapter() {
    let _ = component_wasm("binance_com");
    let adapter = build_wasm_runtime_adapter(
        "binance_com",
        "conn-com-001",
        &CredentialBlob::hmac("key", "secret"),
    )
    .expect("binance_com component adapter");
    assert_eq!(adapter.name(), "binance_com_wasm");
}

#[test]
#[serial]
fn b5_live_start_factory_selects_wasm_not_native_com() {
    let _ = component_wasm("binance_com");
    let adapter = build_runtime_adapter(
        "binance_com",
        "conn-com-live-001",
        &CredentialBlob::hmac("live-key", "live-secret"),
    )
    .expect("Start factory binance_com");
    assert_eq!(adapter.name(), "binance_com_wasm");
    assert_ne!(adapter.name(), "binance_com_spot");
}

#[test]
#[serial]
fn kotak_neo_start_no_longer_bails() {
    let _ = component_wasm("kotak_neo");
    let blob = CredentialBlob::KotakNeoTotpSession {
        consumer_key: "ck".into(),
        trade_token: "tt".into(),
        sid: "sid".into(),
        base_url: "https://cis.kotaksecurities.com".into(),
        hs_server_id: "server4".into(),
        expires_at: None,
    };
    let adapter = build_wasm_runtime_adapter("kotak_neo", "conn-kotak-001", &blob)
        .expect("kotak_neo component adapter");
    assert_eq!(adapter.name(), "kotak_neo_wasm");
}

#[test]
#[serial]
fn b5_live_start_factory_selects_wasm_for_kotak_neo() {
    let _ = component_wasm("kotak_neo");
    let blob = CredentialBlob::KotakNeoTotpSession {
        consumer_key: "ck".into(),
        trade_token: "tt".into(),
        sid: "sid".into(),
        base_url: "https://cis.kotaksecurities.com".into(),
        hs_server_id: "server4".into(),
        expires_at: None,
    };
    let adapter = build_runtime_adapter("kotak_neo", "conn-kotak-live-001", &blob)
        .expect("Start factory kotak_neo");
    assert_eq!(adapter.name(), "kotak_neo_wasm");
}

#[test]
fn b5_test_prefix_com_stays_counting_poll_not_native() {
    let adapter = build_runtime_adapter(
        "binance_com",
        "conn-com-test-001",
        &CredentialBlob::hmac("TA_TEST_SYNC_key", "secret"),
    )
    .expect("test prefix");
    assert_eq!(adapter.name(), "counting_poll");
    assert_ne!(adapter.name(), "binance_com_spot");
}

#[test]
#[serial]
fn component_lookup_prefers_the_configured_directory() {
    let source = component_wasm("binance_com");
    let dir = std::env::temp_dir().join("ubi-component-lookup-test");
    std::fs::create_dir_all(&dir).expect("temp component dir");
    let target = dir.join("ubi_binance_com_adapter.wasm");
    std::fs::copy(&source, &target).expect("copy component");

    std::env::set_var(tradeautopsy_agent::COMPONENT_DIR_ENV, &dir);
    let resolved = component_path_for_slug("binance_com").expect("resolved component");
    std::env::remove_var(tradeautopsy_agent::COMPONENT_DIR_ENV);

    assert_eq!(resolved, target);
    assert!(component_path_for_slug("binance_us").is_none());
}
