//! Issue #17 — partial sync, retry/backoff, and rate-limit handling.

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
    BrokerAdapter, BrokerBalancesSnapshot, BrokerError, BrokerOpenOrdersSnapshot,
    BrokerCredentialVault, ConfigurableDataClassAdapter, DataClassPollRound,
};

async fn post_broker_sync_start(port: u16) -> reqwest::Response {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let body = identity_start_body("binance_us");
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

async fn post_broker_sync_retry(port: u16) -> reqwest::Response {
    let path = "/api/daemon/broker/sync/retry";
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
    .expect("retry")
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

#[tokio::test]
#[serial]
async fn rate_limited_class_reports_rate_limited_status() {
    const PORT: u16 = 19_490;
    let retry_after = chrono::Utc::now().timestamp_millis() + 120_000;
    let adapter = Arc::new(ConfigurableDataClassAdapter::new(vec![DataClassPollRound {
        fills: Ok(vec![]),
        balances: Err(BrokerError::RateLimited {
            retry_after_ms: Some(retry_after),
        }),
        open_orders: Ok(BrokerOpenOrdersSnapshot::empty()),
    }]));
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let opts = TestAgentOptions {
        runtime_poll_adapter: Some(adapter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(post_broker_sync_start(PORT).await.status(), 200);
    tokio::time::sleep(Duration::from_millis(450)).await;

    let state = get_broker_sync_state(PORT).await;
    assert_eq!(state["runtimeStatus"], "rate_limited");
    assert!(state["rateLimitRetryAtMs"].as_i64().is_some());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn manual_retry_clears_requires_manual_retry_flag() {
    const PORT: u16 = 19_491;
    let failing = DataClassPollRound {
        fills: Err(BrokerError::Http("fills down".into())),
        balances: Ok(BrokerBalancesSnapshot::default()),
        open_orders: Ok(BrokerOpenOrdersSnapshot::empty()),
    };
    let adapter = Arc::new(ConfigurableDataClassAdapter::new(vec![
        failing.clone(),
        failing.clone(),
        failing.clone(),
        failing.clone(),
        failing.clone(),
        failing,
        DataClassPollRound {
            fills: Ok(vec![]),
            balances: Ok(BrokerBalancesSnapshot::default()),
            open_orders: Ok(BrokerOpenOrdersSnapshot::empty()),
        },
    ]));
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let opts = TestAgentOptions {
        runtime_poll_adapter: Some(adapter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 40,
        failures_until_open: 3,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(post_broker_sync_start(PORT).await.status(), 200);
    tokio::time::sleep(Duration::from_millis(800)).await;

    let before = get_broker_sync_state(PORT).await;
    assert_eq!(before["requiresManualRetry"], true);

    assert_eq!(post_broker_sync_retry(PORT).await.status(), 200);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let after = get_broker_sync_state(PORT).await;
    assert_eq!(after["requiresManualRetry"], false);

    handle.abort();
}
