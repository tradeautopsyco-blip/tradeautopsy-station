//! S6-F2 — Binance COM spot `obtain(funds)` via mock USER_DATA egress.
//!
//! Seam: `GET /api/station/obtain?adapter=binance_com&operation=funds`
//! after identity-only Start. The kick is host-mediated `GET /api/v3/account`
//! (USER_DATA) pointed at wiremock — not a planted AccountBook slot, not live
//! Binance. Ref: `docs/reference/crypto/binance-global/spot/REST.md` Account Info.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, spawn_test_agent_with_options, wait_ready,
    TestAgentOptions, WireHeaderOverrides, TEST_BROKER_CONNECTION_ID,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, CredentialBlob,
    MemoryBrokerCredentialVault,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PORT: u16 = 19_650;
const API_KEY: &str = "TA_SPOT_FUNDS_KEY_NEVER_IN_OBTAIN";
const API_SECRET: &str = "TA_SPOT_FUNDS_SECRET_NEVER_IN_OBTAIN";

fn account_body() -> serde_json::Value {
    serde_json::json!({
        "canTrade": true,
        "canWithdraw": false,
        "canDeposit": true,
        "balances": [
            { "asset": "MOCKBTC", "free": "1.25000000", "locked": "0.50000000" },
            { "asset": "USDT", "free": "250.00000000", "locked": "0.00000000" },
            { "asset": "ETH", "free": "0.00000000", "locked": "0.00000000" }
        ]
    })
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

async fn obtain_funds(port: u16) -> serde_json::Value {
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

fn holdings_by_asset<'a>(
    funds: &'a serde_json::Value,
) -> std::collections::HashMap<&'a str, &'a serde_json::Value> {
    funds["data"]["holdings"]
        .as_array()
        .expect("holdings")
        .iter()
        .filter_map(|h| h["asset"].as_str().map(|a| (a, h)))
        .collect()
}

/// CountingPollAdapter is not `binance_com`, so merge_poll cannot plant the
/// funds slot. The only way obtain(funds) lights is the kick → mock GET.
fn start_opts(base_url: String) -> TestAgentOptions {
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
        binance_spot_base_url: Some(base_url),
        ..TestAgentOptions::default()
    }
}

#[tokio::test]
#[serial]
async fn obtain_spot_funds_via_mock_account_egress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/account"))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(account_body()))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;

    let dark = obtain_funds(PORT).await;
    assert_eq!(
        dark["status"], "unavailable",
        "obtain must stay dark until identity-only Start attaches the Keychain handle"
    );
    assert!(dark["data"].is_null());

    post_binance_com_start(PORT).await;

    let funds = obtain_funds(PORT).await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["adapter_id"], "binance_com");
    assert_eq!(funds["book_id"], "binance-com-spot");
    assert_eq!(funds["data"]["identity"]["family"], "account");
    assert_eq!(funds["data"]["identity"]["capability_id"], "funds");
    assert_eq!(funds["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(funds["provenance_path"], "/api/v3/account");
    assert_eq!(funds["provenance_adapter_id"], "binance_com");

    let by_asset = holdings_by_asset(&funds);
    assert_eq!(by_asset["MOCKBTC"]["free"], 1.25);
    assert_eq!(by_asset["MOCKBTC"]["locked"], 0.5);
    assert_eq!(by_asset["USDT"]["free"], 250.0);
    assert!(
        !by_asset.contains_key("ETH"),
        "zero free+locked balances must not appear on obtain(funds)"
    );

    let wire = funds.to_string();
    assert!(!wire.contains(API_KEY), "apiKey must not leak on obtain");
    assert!(
        !wire.contains(API_SECRET),
        "apiSecret must not leak on obtain"
    );
    assert!(!wire.contains("signature"), "HMAC signature must not leak");

    // Fresh slot: a second obtain must not charge the mock again.
    let again = obtain_funds(PORT).await;
    assert_eq!(again["status"], "success");
    assert_eq!(
        again["data"]["holdings"][0]["asset"],
        funds["data"]["holdings"][0]["asset"]
    );

    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        1,
        "fresh AccountBook slot must skip a second GET"
    );
    let url = received[0].url.to_string();
    assert!(url.contains("timestamp="), "{url}");
    assert!(url.contains("signature="), "{url}");
    assert!(
        !url.contains(API_SECRET),
        "signing secret must stay off the query besides HMAC hex"
    );

    handle.abort();
}
