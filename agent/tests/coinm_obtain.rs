//! T4 — Binance COM Coin-M `obtain(funds|positionbook|forceorder)` via mock USER_DATA egress.
//!
//! Seam: `GET /api/station/obtain?adapter=binance_com&book=binance-com-coinm&operation=…`
//! after identity-only Start. Kick is host-mediated HMAC GET at wiremock — not a
//! planted AccountBook slot, not live dapi. Lock: `issues/compliance/locks/binance-com-coinm.md`.
//! Fixtures are independent obtain literals (not parser-test imports). Not USDM.

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

const PORT: u16 = 19_660;
const API_KEY: &str = "TA_COINM_OBTAIN_KEY_NEVER_IN_OBTAIN";
const API_SECRET: &str = "TA_COINM_OBTAIN_SECRET_NEVER_IN_OBTAIN";

/// Independent obtain fixtures — different numbers from USDM and from parser goldens.
const COINM_BALANCE_FIXTURE: &str =
    r#"[{"asset":"BTC","availableBalance":"0.5"},{"asset":"ETH","availableBalance":"0"}]"#;
const COINM_POSITIONS_FIXTURE: &str =
    r#"[{"symbol":"ETHUSD_PERP","positionAmt":"0"},{"symbol":"BTCUSD_PERP","positionAmt":"3"}]"#;
const COINM_FORCE_ORDERS_EMPTY: &str = r#"[]"#;
const COINM_FORCE_ORDERS_OBSERVING: &str = r#"[{"symbol":"BTCUSD_PERP","side":"SELL"}]"#;

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

fn btc_free(funds: &serde_json::Value) -> Option<f64> {
    funds["data"]["holdings"].as_array()?.iter().find_map(|h| {
        (h["asset"].as_str() == Some("BTC"))
            .then(|| h["free"].as_f64())
            .flatten()
    })
}

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
        binance_coinm_base_url: Some(base_url),
        ..TestAgentOptions::default()
    }
}

fn assert_no_secrets(wire: &str) {
    assert!(!wire.contains(API_KEY), "apiKey must not leak on obtain");
    assert!(
        !wire.contains(API_SECRET),
        "apiSecret must not leak on obtain"
    );
    assert!(!wire.contains("signature"), "HMAC signature must not leak");
}

fn inner_forceorder_status(body: &serde_json::Value) -> &str {
    body["data"]["status"].as_str().unwrap_or("")
}

fn inner_forceorder_physics(body: &serde_json::Value) -> &str {
    body["data"]["identity"]["physics"]
        .as_str()
        .or_else(|| body["data"]["physics"].as_str())
        .unwrap_or("")
}

fn assert_lossy_forceorder(body: &serde_json::Value, expected_status: &str) {
    assert_eq!(body["status"], "success", "forceorder obtain: {body}");
    assert!(
        !body["data"].is_null(),
        "empty success with data:null is a fail: {body}"
    );
    let wire = body.to_string();
    assert!(
        !wire.contains("complete_event_sequence"),
        "force-order must not claim complete_event_sequence: {wire}"
    );
    let physics = inner_forceorder_physics(body);
    let status = inner_forceorder_status(body);
    assert!(
        physics == "lossy_event_observation" || status == "idle" || status == "observing",
        "expected lossy_event_observation or idle/observing, got physics={physics} status={status} body={body}"
    );
    assert_ne!(status, "complete");
    assert_ne!(status, "synced");
    assert_ne!(status, "fresh");
    assert_ne!(physics, "complete_event_sequence");
    if expected_status == "idle" {
        assert!(
            status == "idle" || physics == "lossy_event_observation",
            "empty window must be idle (lossy), got status={status} physics={physics}"
        );
        if !status.is_empty() {
            assert_eq!(
                status, "idle",
                "empty forceOrders window is Idle, not complete"
            );
        }
    } else {
        assert_eq!(status, expected_status, "forceorder status: {body}");
    }
}

async fn mount_hmac_get(server: &MockServer, venue_path: &str, body: &str) {
    Mock::given(method("GET"))
        .and(path(venue_path))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_string(body.to_string()))
        .mount(server)
        .await;
}

#[tokio::test]
#[serial]
async fn obtain_coinm_funds_positionbook_and_empty_forceorder_via_mock_egress() {
    let server = MockServer::start().await;
    mount_hmac_get(&server, "/dapi/v1/balance", COINM_BALANCE_FIXTURE).await;
    mount_hmac_get(&server, "/dapi/v1/positionRisk", COINM_POSITIONS_FIXTURE).await;
    mount_hmac_get(&server, "/dapi/v1/forceOrders", COINM_FORCE_ORDERS_EMPTY).await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;

    let dark_funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=funds",
    )
    .await;
    assert_eq!(
        dark_funds["status"], "unavailable",
        "obtain must stay dark until identity-only Start attaches the Keychain handle"
    );
    assert!(dark_funds["data"].is_null());

    let dark_pos = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=positionbook",
    )
    .await;
    assert_eq!(dark_pos["status"], "unavailable");
    assert!(dark_pos["data"].is_null());

    let dark_fo = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=forceorder",
    )
    .await;
    assert_eq!(dark_fo["status"], "unavailable");
    assert!(dark_fo["data"].is_null());

    post_binance_com_start(PORT).await;

    let funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=funds",
    )
    .await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["adapter_id"], "binance_com");
    assert_eq!(funds["book_id"], "binance-com-coinm");
    assert_eq!(funds["data"]["identity"]["family"], "account");
    assert_eq!(funds["data"]["identity"]["capability_id"], "funds");
    assert_eq!(funds["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(funds["provenance_path"], "/dapi/v1/balance");
    assert_eq!(funds["provenance_adapter_id"], "binance_com");
    assert!(
        !funds["data"].is_null(),
        "empty success with data:null is a fail"
    );
    assert!(funds["data"]["unrealized_pnl"].is_null());

    let by_asset = holdings_by_asset(&funds);
    assert_eq!(by_asset["BTC"]["free"], 0.5);
    assert_eq!(by_asset["BTC"]["locked"], 0.0);
    assert!(
        !by_asset.contains_key("ETH"),
        "availableBalance <= 0 must not appear on obtain(funds)"
    );
    assert_no_secrets(&funds.to_string());

    let positions = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=positionbook",
    )
    .await;
    assert_eq!(positions["status"], "success");
    assert_eq!(positions["book_id"], "binance-com-coinm");
    assert_eq!(positions["data"]["identity"]["capability_id"], "positions");
    assert_eq!(positions["provenance_path"], "/dapi/v1/positionRisk");
    assert_eq!(positions["data"]["position_count"], 1);
    let rows = positions["data"]["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["symbol"], "BTCUSD_PERP");
    assert_eq!(rows[0]["net_qty"], 3.0);
    assert_ne!(rows[0]["net_qty"], 1.0, "qty is 3 not 1");
    assert_eq!(rows[0]["segment"], "coinm");
    assert_ne!(
        rows[0]["segment"], "usdm",
        "Coin-M segment must not be usdm"
    );
    assert_no_secrets(&positions.to_string());

    let forceorder = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=forceorder",
    )
    .await;
    assert_lossy_forceorder(&forceorder, "idle");
    assert_no_secrets(&forceorder.to_string());

    let spot = obtain(PORT, "adapter=binance_com&operation=funds").await;
    assert!(
        (spot["status"] == "unavailable" && spot["data"].is_null())
            || (spot["book_id"] != "binance-com-coinm" && btc_free(&spot) != Some(0.5)),
        "DualNoBlend: Coin-M BTC 0.5 must not paint shipping spot: {spot}"
    );
    assert_ne!(spot["book_id"], "binance-com-coinm");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_coinm_forceorder_with_event_is_observing_not_complete() {
    let server = MockServer::start().await;
    mount_hmac_get(
        &server,
        "/dapi/v1/forceOrders",
        COINM_FORCE_ORDERS_OBSERVING,
    )
    .await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;
    post_binance_com_start(PORT).await;

    let forceorder = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-coinm&operation=forceorder",
    )
    .await;
    assert_lossy_forceorder(&forceorder, "observing");
    let wire = forceorder.to_string();
    assert!(
        wire.contains("BTCUSD_PERP")
            || forceorder["data"].is_object()
            || forceorder["data"].is_array(),
        "observing force-order must keep data present: {forceorder}"
    );
    assert_no_secrets(&wire);

    handle.abort();
}
