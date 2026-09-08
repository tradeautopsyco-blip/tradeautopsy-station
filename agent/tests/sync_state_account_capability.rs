//! S6 — sync-state account capability honesty after Start.
//!
//! Seam: `GET /api/daemon/broker/sync-state` after identity-only Start.
//! Account `capabilities.*` must follow AccountBook + the shipping SourceManifest,
//! not poller `dataClasses` freshness. A CountingPollAdapter success is not a
//! funds slot. Notch UI and options `userTrades` are out of scope.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, spawn_test_agent_with_options, wait_ready,
    TestAgentOptions, WireHeaderOverrides, TEST_BROKER_CONNECTION_ID,
};
use serde_json::Value;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, CredentialBlob,
    MemoryBrokerCredentialVault,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PORT_POLLER: u16 = 19_655;
const PORT_OBTAIN: u16 = 19_656;
const API_KEY: &str = "TA_SYNC_STATE_KEY_NEVER_ON_WIRE";
const API_SECRET: &str = "TA_SYNC_STATE_SECRET_NEVER_ON_WIRE";

async fn get_broker_sync_state(port: u16) -> Value {
    let path = "/api/daemon/broker/sync-state";
    let url = format!("http://127.0.0.1:{port}{path}");
    apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("sync-state")
    .json()
    .await
    .expect("sync-state json")
}

async fn post_binance_com_start(port: u16) {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&identity_start_body("binance_com")).expect("json");
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
    assert_eq!(resp.status(), 200, "identity-only Start must succeed");
}

fn counting_opts(binance_spot_base_url: Option<String>) -> TestAgentOptions {
    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    vault
        .save(
            "prod",
            "binance_com",
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::hmac(API_KEY, API_SECRET),
        )
        .expect("seed vault");
    TestAgentOptions {
        runtime_poll_adapter: Some(Arc::new(CountingPollAdapter::new()) as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        binance_spot_base_url,
        ..TestAgentOptions::default()
    }
}

fn account_body() -> serde_json::Value {
    serde_json::json!({
        "canTrade": true,
        "canWithdraw": false,
        "canDeposit": true,
        "balances": [
            { "asset": "MOCKBTC", "free": "1.00000000", "locked": "0.00000000" }
        ]
    })
}

async fn obtain_funds(port: u16) -> Value {
    client()
        .get(format!(
            "http://127.0.0.1:{port}/api/station/obtain?adapter=binance_com&operation=funds"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("obtain funds")
        .json()
        .await
        .expect("obtain json")
}

fn assert_no_secrets(body: &Value) {
    let wire = body.to_string();
    assert!(!wire.contains(API_KEY), "apiKey must not leak on sync-state");
    assert!(
        !wire.contains(API_SECRET),
        "apiSecret must not leak on sync-state"
    );
    assert!(
        !wire.contains("signature"),
        "HMAC signature must not leak on sync-state"
    );
}

#[tokio::test]
#[serial]
async fn account_caps_unavailable_before_start() {
    let handle = spawn_test_agent_with_options(PORT_POLLER, counting_opts(None));
    wait_ready(PORT_POLLER).await;

    let before = get_broker_sync_state(PORT_POLLER).await;
    assert_eq!(before["runtimeStatus"], "ready_to_start");
    assert_eq!(before["capabilities"]["funds"], "unavailable");
    assert_eq!(before["capabilities"]["fills"], "unavailable");
    assert_eq!(before["capabilities"]["holdings"], "unavailable");
    assert_eq!(before["capabilities"]["positions"], "unavailable");
    assert_eq!(before["capabilities"]["orders"], "unavailable");
    assert_no_secrets(&before);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn poller_success_does_not_paint_account_caps_fresh() {
    let handle = spawn_test_agent_with_options(PORT_POLLER, counting_opts(None));
    wait_ready(PORT_POLLER).await;
    post_binance_com_start(PORT_POLLER).await;
    tokio::time::sleep(Duration::from_millis(450)).await;

    let after = get_broker_sync_state(PORT_POLLER).await;
    assert_eq!(after["runtimeStatus"], "syncing");
    assert_eq!(after["brokerSlug"], "binance_com");
    assert_eq!(
        after["dataClasses"]["balances_holdings"]["current"], true,
        "poller dataClasses stay a separate completeness axis"
    );
    assert_eq!(after["dataClasses"]["fills_trade_history"]["current"], true);
    assert_eq!(
        after["capabilities"]["funds"], "unavailable",
        "CountingPollAdapter must not mint a funds slot"
    );
    assert_eq!(after["capabilities"]["fills"], "unavailable");
    assert_eq!(after["capabilities"]["orders"], "unavailable");
    assert_eq!(
        after["capabilities"]["holdings"], "unsupported",
        "spot manifest does not declare holdings"
    );
    assert_eq!(after["capabilities"]["positions"], "unsupported");
    assert_no_secrets(&after);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_funds_slot_lights_sync_state_funds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/account"))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(account_body()))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(PORT_OBTAIN, counting_opts(Some(server.uri())));
    wait_ready(PORT_OBTAIN).await;
    post_binance_com_start(PORT_OBTAIN).await;

    let dark = get_broker_sync_state(PORT_OBTAIN).await;
    assert_eq!(dark["capabilities"]["funds"], "unavailable");

    let funds = obtain_funds(PORT_OBTAIN).await;
    assert_eq!(funds["status"], "success");

    let lit = get_broker_sync_state(PORT_OBTAIN).await;
    assert_eq!(lit["capabilities"]["funds"], "fresh");
    assert_eq!(lit["capabilities"]["holdings"], "unsupported");
    assert_eq!(lit["capabilities"]["orders"], "unavailable");
    assert_no_secrets(&lit);

    handle.abort();
}
