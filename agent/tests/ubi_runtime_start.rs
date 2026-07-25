//! Runtime start selects Wasm components for UBI brokers (Phase 3 · ADR 0001).
//!
//! `binance_com` no longer routes to the native reference adapter and `kotak_neo` no
//! longer bails with "ships in Phase 3".

mod ubi_support;

use serial_test::serial;
use tradeautopsy_agent::{
    build_wasm_runtime_adapter, component_path_for_slug, uses_wasm_component, CredentialBlob,
};
use ubi_support::component_wasm;

#[test]
fn enabled_first_party_brokers_run_as_components() {
    assert!(uses_wasm_component("binance_com"));
    assert!(uses_wasm_component("kotak_neo"));
    assert!(!uses_wasm_component("binance_us"));
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
fn kotak_neo_start_no_longer_bails() {
    let _ = component_wasm("kotak_neo");
    let blob = CredentialBlob::KotakNeoTotpSession {
        consumer_key: "ck".into(),
        trade_token: "tt".into(),
        sid: "sid".into(),
        base_url: "https://cis.kotaksecurities.com".into(),
        expires_at: None,
    };
    let adapter = build_wasm_runtime_adapter("kotak_neo", "conn-kotak-001", &blob)
        .expect("kotak_neo component adapter");
    assert_eq!(adapter.name(), "kotak_neo_wasm");
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
