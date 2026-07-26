//! Issue #13/#14 — runtime broker sync start/stop without agent restart.
//! T2.2 / B2 — Start is identity-only; credentials seed the host vault, never the wire.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    TestAgentOptions, WireHeaderOverrides, TEST_BROKER_CONNECTION_ID,
};
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, CredentialBlob,
    MemoryBrokerCredentialVault,
};

async fn get_health_boot_id(port: u16) -> String {
    let path = "/api/daemon/health";
    let url = format!("http://127.0.0.1:{port}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    let body: Value = req
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("health")
        .json()
        .await
        .expect("health json");
    body["boot_id"]
        .as_str()
        .expect("boot_id")
        .to_string()
}

async fn post_broker_sync_start(port: u16, body: Value) -> reqwest::Response {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&body).expect("json");
    apply_wire_v1(
        client().post(&url).header("content-type", "application/json"),
        "POST",
        path,
        &payload,
        WireHeaderOverrides::default(),
    )
    .body(payload)
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("start")
}

async fn post_broker_sync_stop(port: u16) -> reqwest::Response {
    let path = "/api/daemon/broker/sync/stop";
    let url = format!("http://127.0.0.1:{port}{path}");
    apply_wire_v1(
        client().post(&url),
        "POST",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("stop")
}

async fn get_broker_sync_state(port: u16) -> Value {
    let path = "/api/daemon/broker/sync-state";
    let url = format!("http://127.0.0.1:{port}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    req.timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("sync-state")
        .json()
        .await
        .expect("sync-state json")
}

fn opts_with_vault(
    vault: Arc<MemoryBrokerCredentialVault>,
    counter: Arc<CountingPollAdapter>,
) -> TestAgentOptions {
    TestAgentOptions {
        runtime_poll_adapter: Some(counter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    }
}

#[tokio::test]
#[serial]
async fn runtime_start_reports_syncing_and_polls_without_preconfigured_adapter() {
    const PORT: u16 = 19_470;
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    let handle = spawn_test_agent_with_options(PORT, opts_with_vault(vault, counter.clone()));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let before = get_broker_sync_state(PORT).await;
    assert_eq!(before["runtimeStatus"], "ready_to_start");

    let resp = post_broker_sync_start(PORT, identity_start_body("binance_us")).await;
    assert_eq!(resp.status(), 200, "start status");

    tokio::time::sleep(Duration::from_millis(450)).await;
    let after = get_broker_sync_state(PORT).await;
    assert_eq!(after["runtimeStatus"], "syncing");
    assert!(counter.poll_count() >= 1, "expected at least one poll after start");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn stop_halts_polling_without_changing_agent_boot_id() {
    const PORT: u16 = 19_471;
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    let opts = TestAgentOptions {
        broker_base_poll_ms: 60,
        ..opts_with_vault(vault, counter.clone())
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;
    let polls_before_stop = counter.poll_count();
    assert!(polls_before_stop >= 2, "expected active polling before stop");

    let boot_before = get_health_boot_id(PORT).await;

    let stop_resp = post_broker_sync_stop(PORT).await;
    assert_eq!(stop_resp.status(), 200);

    tokio::time::sleep(Duration::from_millis(50)).await;
    let polls_at_stop = counter.poll_count();
    tokio::time::sleep(Duration::from_millis(450)).await;
    assert_eq!(
        counter.poll_count(),
        polls_at_stop,
        "poll count should freeze after stop settles"
    );

    let boot_after = get_health_boot_id(PORT).await;
    assert_eq!(boot_before, boot_after, "agent process must not restart on stop");

    let state = get_broker_sync_state(PORT).await;
    assert_eq!(state["runtimeStatus"], "paused");
    assert_eq!(state["syncState"], "not_connected");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn start_after_stop_resumes_polling_with_fresh_vault_credentials() {
    const PORT: u16 = 19_472;
    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    vault
        .save(
            "prod",
            "binance_us",
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::hmac("first-key", "test-secret"),
        )
        .unwrap();
    let start_keys = Arc::new(Mutex::new(Vec::<String>::new()));
    let counter = Arc::new(CountingPollAdapter::new());
    let opts = TestAgentOptions {
        runtime_poll_adapter: Some(counter.clone() as Arc<dyn BrokerAdapter>),
        start_key_log: Some(start_keys.clone()),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault.clone() as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(post_broker_sync_stop(PORT).await.status(), 200);

    // Rotate in the host vault (not on the wire), then Start identity-only again.
    vault
        .save(
            "prod",
            "binance_us",
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::hmac("second-key", "rotated-secret"),
        )
        .unwrap();

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(250)).await;

    let seen = start_keys.lock().expect("keys").clone();
    assert_eq!(seen, vec!["first-key".to_string(), "second-key".to_string()]);
    assert!(counter.poll_count() >= 1, "resume should restart polling");

    let state = get_broker_sync_state(PORT).await;
    assert_eq!(state["runtimeStatus"], "syncing");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn start_without_wire_secrets_loads_host_vault() {
    const PORT: u16 = 19_473;
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC_VAULT");
    let counter = Arc::new(CountingPollAdapter::new());
    let handle = spawn_test_agent_with_options(PORT, opts_with_vault(vault, counter.clone()));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let resp = post_broker_sync_start(PORT, identity_start_body("binance_us")).await;
    assert_eq!(resp.status(), 200, "identity-only start");

    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(counter.poll_count() >= 1);
    handle.abort();
}

#[tokio::test]
#[serial]
async fn b2_wire_secrets_on_start_are_rejected() {
    const PORT: u16 = 19_474;
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    let handle = spawn_test_agent_with_options(PORT, opts_with_vault(vault, counter));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let resp = post_broker_sync_start(
        PORT,
        json!({
            "brokerSlug": "binance_us",
            "brokerConnectionId": TEST_BROKER_CONNECTION_ID,
            "environment": "prod",
            "assetClass": "crypto",
            "apiKey": "leak-key",
            "apiSecret": "leak-secret"
        }),
    )
    .await;
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.expect("json");
    let err = body["error"].as_str().unwrap_or("");
    assert!(err.contains("wire credentials refused"), "{err}");
    assert!(!err.contains("leak-key"), "{err}");
    assert!(!err.contains("leak-secret"), "{err}");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn b2_empty_vault_start_fails() {
    const PORT: u16 = 19_475;
    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    let counter = Arc::new(CountingPollAdapter::new());
    let handle = spawn_test_agent_with_options(PORT, opts_with_vault(vault, counter));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let resp = post_broker_sync_start(PORT, identity_start_body("binance_us")).await;
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.expect("json");
    let err = body["error"].as_str().unwrap_or("");
    assert!(err.contains("missing credentials"), "{err}");

    handle.abort();
}
