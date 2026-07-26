//! Issue #16 — balances/holdings and open-orders data classes + completeness.
//! T2.2 / B2 — Start is identity-only; credentials seed the host vault.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    TestAgentOptions, WireHeaderOverrides,
};
use serde_json::Value;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerBalancesSnapshot, BrokerCredentialVault, BrokerError,
    ConfigurableDataClassAdapter, DataClassPollRound,
};

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

fn start_with_vault(adapter: Arc<dyn BrokerAdapter>, poll_ms: u64) -> TestAgentOptions {
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    TestAgentOptions {
        runtime_poll_adapter: Some(adapter),
        broker_base_poll_ms: poll_ms,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    }
}

#[tokio::test]
#[serial]
async fn all_data_classes_current_reports_syncing() {
    const PORT: u16 = 19_480;
    let adapter = Arc::new(ConfigurableDataClassAdapter::all_ok());
    let handle = spawn_test_agent_with_options(
        PORT,
        start_with_vault(adapter as Arc<dyn BrokerAdapter>, 80),
    );
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(450)).await;

    let state = get_broker_sync_state(PORT).await;
    assert_eq!(state["runtimeStatus"], "syncing");
    assert_eq!(state["dataClasses"]["fills_trade_history"]["current"], true);
    assert_eq!(state["dataClasses"]["balances_holdings"]["current"], true);
    assert_eq!(state["dataClasses"]["open_orders"]["current"], true);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn partial_data_class_failure_reports_degraded() {
    const PORT: u16 = 19_481;
    let failing = DataClassPollRound {
        fills: Ok(vec![]),
        balances: Ok(BrokerBalancesSnapshot::default()),
        open_orders: Err(BrokerError::Http("open orders unavailable".into())),
    };
    let adapter = Arc::new(ConfigurableDataClassAdapter::new(vec![
        failing.clone(),
        failing.clone(),
        failing.clone(),
        failing,
    ]));
    let handle = spawn_test_agent_with_options(
        PORT,
        start_with_vault(adapter as Arc<dyn BrokerAdapter>, 80),
    );
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(450)).await;

    let state = get_broker_sync_state(PORT).await;
    assert_eq!(state["runtimeStatus"], "degraded");
    assert_eq!(state["dataClasses"]["fills_trade_history"]["current"], true);
    assert_eq!(state["dataClasses"]["open_orders"]["current"], false);
    assert!(state["failingDataClasses"]
        .as_array()
        .expect("failing")
        .iter()
        .any(|v| v == "open_orders"));

    handle.abort();
}

#[tokio::test]
#[serial]
async fn empty_open_orders_snapshot_is_current() {
    const PORT: u16 = 19_482;
    let adapter = Arc::new(ConfigurableDataClassAdapter::all_ok());
    let handle = spawn_test_agent_with_options(
        PORT,
        start_with_vault(adapter as Arc<dyn BrokerAdapter>, 80),
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(450)).await;

    let state = get_broker_sync_state(PORT).await;
    assert_eq!(state["dataClasses"]["open_orders"]["current"], true);

    handle.abort();
}
