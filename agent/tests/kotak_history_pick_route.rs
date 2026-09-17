//! S7 — Kotak obtain history/quote through pick_route; fixture licensed_history.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    wait_ready, TestAgentOptions, WireHeaderOverrides,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, CredentialBlob,
    MemoryBrokerCredentialVault,
};

const PORT_UNSUP: u16 = 19_670;
const PORT_VENDOR: u16 = 19_671;
const PORT_BUDGET: u16 = 19_672;
const PORT_URL: u16 = 19_673;
const PORT_VAULT: u16 = 19_780;
const PORT_VAULT_QUOTA: u16 = 19_781;
const PORT_VAULT_URL: u16 = 19_782;

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn post_kotak_start(port: u16) {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&identity_start_body("kotak_neo")).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header("content-type", "application/json"),
        "POST",
        path,
        &payload,
        WireHeaderOverrides::default(),
    )
    .body(payload)
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("start");
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    assert_eq!(status, 200, "identity-only Start must succeed: {body}");
}

async fn obtain(port: u16, query: &str) -> serde_json::Value {
    client()
        .get(format!(
            "http://127.0.0.1:{port}/api/station/obtain?{query}"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("obtain")
        .json()
        .await
        .expect("json")
}

/// Planted S1k last is scoped to the bound cash id (same as `s1k_fixture_gate`).
async fn bind_planted_cash_quote(port: u16) {
    let quote: serde_json::Value = client()
        .get(format!(
            "http://127.0.0.1:{port}/api/station/quote?instrument=nse_cm%7C2885"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("bind quote")
        .json()
        .await
        .expect("quote json");
    assert_ne!(
        quote["status"], "unavailable",
        "planted cash last must bind before obtain(quotes)"
    );
}

fn kotak_start_opts(extra: TestAgentOptions) -> TestAgentOptions {
    let vault = seeded_hmac_vault("kotak_neo", "TA_S7_HISTORY_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    TestAgentOptions {
        runtime_poll_adapter: Some(counter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        plant_kotak_s1k_fixtures: true,
        ..extra
    }
}

fn vendor_on() -> TestAgentOptions {
    kotak_start_opts(TestAgentOptions {
        plant_licensed_history_gap: true,
        gap_vendor_enabled: true,
        gap_vendor_key: Some("lh-fixture-key".into()),
        gap_vendor_history_budget: 5,
        kotak_quote_budget: 10,
        ..TestAgentOptions::default()
    })
}

fn vault_with_kotak_and_vendor(vendor_key: &str) -> Arc<MemoryBrokerCredentialVault> {
    let vault = seeded_hmac_vault("kotak_neo", "TA_S7_HISTORY_SYNC");
    vault
        .save(
            "prod",
            "licensed_history",
            "licensed_history",
            &CredentialBlob::hmac(vendor_key, "unused"),
        )
        .expect("plant vendor vault key");
    vault
}

fn kotak_start_opts_vault(
    vault: Arc<MemoryBrokerCredentialVault>,
    extra: TestAgentOptions,
) -> TestAgentOptions {
    let counter = Arc::new(CountingPollAdapter::new());
    TestAgentOptions {
        runtime_poll_adapter: Some(counter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        plant_kotak_s1k_fixtures: true,
        ..extra
    }
}

#[tokio::test]
#[serial]
async fn kotak_history_without_vendor_is_unsupported() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_UNSUP,
        kotak_start_opts(TestAgentOptions::default()),
    ));
    wait_ready(PORT_UNSUP).await;
    post_kotak_start(PORT_UNSUP).await;
    let history = obtain(PORT_UNSUP, "adapter=kotak_neo&operation=history").await;
    assert_eq!(history["status"], "unsupported");
    assert!(history["data"].is_null());
    assert_ne!(history["book_id"], "licensed-history");
}

#[tokio::test]
#[serial]
async fn fixture_vendor_fills_history_with_vendor_book_and_provenance() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(PORT_VENDOR, vendor_on()));
    wait_ready(PORT_VENDOR).await;
    post_kotak_start(PORT_VENDOR).await;

    let history = obtain(PORT_VENDOR, "adapter=kotak_neo&operation=history").await;
    assert_eq!(history["status"], "success");
    assert_eq!(history["provenance_adapter_id"], "licensed_history");
    assert_eq!(history["adapter_id"], "licensed_history");
    assert_eq!(history["book_id"], "licensed-history");
    assert_ne!(history["book_id"], "kotak-nse-nfo");
    assert_ne!(history["book_id"], "kotak-nse-bse-cash");
    let candles = history["data"]["candles"].as_array().expect("candles");
    assert!(!candles.is_empty());

    bind_planted_cash_quote(PORT_VENDOR).await;
    let quote = obtain(PORT_VENDOR, "adapter=kotak_neo&operation=quotes").await;
    assert_eq!(quote["status"], "success");
    assert_eq!(quote["provenance_adapter_id"], "kotak_neo");
    assert_eq!(quote["book_id"], "kotak-nse-bse-cash");
}

#[tokio::test]
#[serial]
async fn vendor_budget_zero_history_unavailable_quote_still_kotak() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_BUDGET,
        kotak_start_opts(TestAgentOptions {
            plant_licensed_history_gap: true,
            gap_vendor_enabled: true,
            gap_vendor_key: Some("lh-fixture-key".into()),
            gap_vendor_history_budget: 0,
            kotak_quote_budget: 10,
            ..TestAgentOptions::default()
        }),
    ));
    wait_ready(PORT_BUDGET).await;
    post_kotak_start(PORT_BUDGET).await;
    let history = obtain(PORT_BUDGET, "adapter=kotak_neo&operation=history").await;
    assert_eq!(history["status"], "unavailable");
    assert!(history["data"].is_null());
    bind_planted_cash_quote(PORT_BUDGET).await;
    let quote = obtain(PORT_BUDGET, "adapter=kotak_neo&operation=quotes").await;
    assert_eq!(quote["status"], "success");
    assert_eq!(quote["provenance_adapter_id"], "kotak_neo");
}

#[tokio::test]
#[serial]
async fn url_shaped_secret_does_not_light_vendor() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_URL,
        kotak_start_opts(TestAgentOptions {
            plant_licensed_history_gap: true,
            gap_vendor_enabled: true,
            gap_vendor_key: Some("https://evil.example/klines".into()),
            gap_vendor_history_budget: 9,
            kotak_quote_budget: 10,
            ..TestAgentOptions::default()
        }),
    ));
    wait_ready(PORT_URL).await;
    post_kotak_start(PORT_URL).await;
    let history = obtain(PORT_URL, "adapter=kotak_neo&operation=history").await;
    assert_eq!(history["status"], "unsupported");
    assert_ne!(history["book_id"], "licensed-history");
    assert_ne!(history["provenance_adapter_id"], "licensed_history");
}

/// Production path: vendor key lives in the host vault (not TestAgentOptions.gap_vendor_key).
/// Fixture licensed_history; no lock says desk, so product_use is labs.
#[tokio::test]
#[serial]
async fn vault_vendor_key_history_success_is_labs_licensed_history() {
    let vault = vault_with_kotak_and_vendor("lh-fixture-key");
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_VAULT,
        kotak_start_opts_vault(
            vault,
            TestAgentOptions {
                plant_licensed_history_gap: true,
                gap_vendor_enabled: true,
                gap_vendor_history_budget: 1,
                kotak_quote_budget: 10,
                ..TestAgentOptions::default()
            },
        ),
    ));
    wait_ready(PORT_VAULT).await;
    post_kotak_start(PORT_VAULT).await;
    let history = obtain(PORT_VAULT, "adapter=kotak_neo&operation=history").await;
    assert_eq!(history["status"], "success");
    assert_eq!(history["provenance_adapter_id"], "licensed_history");
    assert_eq!(history["book_id"], "licensed-history");
    assert_eq!(history["product_use"], "labs");
}

/// Live quota: budget 1 is consumed by the first vendor history obtain.
/// Quote meter is independent — Kotak last still succeeds.
#[tokio::test]
#[serial]
async fn vault_vendor_budget_one_second_history_unavailable_quote_still_kotak() {
    let vault = vault_with_kotak_and_vendor("lh-fixture-key");
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_VAULT_QUOTA,
        kotak_start_opts_vault(
            vault,
            TestAgentOptions {
                plant_licensed_history_gap: true,
                gap_vendor_enabled: true,
                gap_vendor_history_budget: 1,
                kotak_quote_budget: 10,
                ..TestAgentOptions::default()
            },
        ),
    ));
    wait_ready(PORT_VAULT_QUOTA).await;
    post_kotak_start(PORT_VAULT_QUOTA).await;

    let first = obtain(PORT_VAULT_QUOTA, "adapter=kotak_neo&operation=history").await;
    assert_eq!(first["status"], "success");
    assert_eq!(first["provenance_adapter_id"], "licensed_history");

    let second = obtain(PORT_VAULT_QUOTA, "adapter=kotak_neo&operation=history").await;
    assert_eq!(second["status"], "unavailable");
    assert!(second["data"].is_null());

    bind_planted_cash_quote(PORT_VAULT_QUOTA).await;
    let quote = obtain(PORT_VAULT_QUOTA, "adapter=kotak_neo&operation=quotes").await;
    assert_eq!(quote["status"], "success");
    assert_eq!(quote["provenance_adapter_id"], "kotak_neo");
}

#[tokio::test]
#[serial]
async fn vault_url_shaped_secret_does_not_light_vendor() {
    let vault = vault_with_kotak_and_vendor("https://evil.example/klines");
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_VAULT_URL,
        kotak_start_opts_vault(
            vault,
            TestAgentOptions {
                plant_licensed_history_gap: true,
                gap_vendor_enabled: true,
                gap_vendor_history_budget: 9,
                kotak_quote_budget: 10,
                ..TestAgentOptions::default()
            },
        ),
    ));
    wait_ready(PORT_VAULT_URL).await;
    post_kotak_start(PORT_VAULT_URL).await;
    let history = obtain(PORT_VAULT_URL, "adapter=kotak_neo&operation=history").await;
    assert_eq!(history["status"], "unsupported");
    assert_ne!(history["book_id"], "licensed-history");
}
