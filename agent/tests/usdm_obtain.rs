//! T4 — Binance COM USDM `obtain(funds|positionbook|forceorder)` via mock USER_DATA egress.
//!
//! Seam: `GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=…`
//! after identity-only Start. Kick is host-mediated HMAC GET at wiremock — not a
//! planted AccountBook slot, not live fapi. Lock: `issues/compliance/locks/binance-com-usdm.md`.
//! Fixtures are independent obtain literals (not parser-test imports).

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

const PORT: u16 = 19_658;
const API_KEY: &str = "TA_USDM_OBTAIN_KEY_NEVER_IN_OBTAIN";
const API_SECRET: &str = "TA_USDM_OBTAIN_SECRET_NEVER_IN_OBTAIN";

/// Independent obtain fixture — not imported from `binance_com_usdm_client` goldens.
const USDM_BALANCE_FIXTURE: &str =
    r#"[{"asset":"USDT","availableBalance":"23.72"},{"asset":"BNB","availableBalance":"0"}]"#;
const USDM_POSITIONS_FIXTURE: &str =
    r#"[{"symbol":"BTCUSDT","positionAmt":"0"},{"symbol":"ETHUSDT","positionAmt":"2"}]"#;
const USDM_FORCE_ORDERS_EMPTY: &str = r#"[]"#;
const USDM_FORCE_ORDERS_OBSERVING: &str = r#"[{"symbol":"BTCUSDT","side":"SELL"}]"#;

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

fn usdt_free(funds: &serde_json::Value) -> Option<f64> {
    funds["data"]["holdings"].as_array()?.iter().find_map(|h| {
        (h["asset"].as_str() == Some("USDT"))
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
        binance_usdm_base_url: Some(base_url),
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
async fn obtain_usdm_funds_positionbook_and_empty_forceorder_via_mock_egress() {
    let server = MockServer::start().await;
    mount_hmac_get(&server, "/fapi/v3/balance", USDM_BALANCE_FIXTURE).await;
    mount_hmac_get(&server, "/fapi/v3/positionRisk", USDM_POSITIONS_FIXTURE).await;
    mount_hmac_get(&server, "/fapi/v1/forceOrders", USDM_FORCE_ORDERS_EMPTY).await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;

    let dark_funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=funds",
    )
    .await;
    assert_eq!(
        dark_funds["status"], "unavailable",
        "obtain must stay dark until identity-only Start attaches the Keychain handle"
    );
    assert!(dark_funds["data"].is_null());

    let dark_pos = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=positionbook",
    )
    .await;
    assert_eq!(dark_pos["status"], "unavailable");
    assert!(dark_pos["data"].is_null());

    let dark_fo = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=forceorder",
    )
    .await;
    assert_eq!(dark_fo["status"], "unavailable");
    assert!(dark_fo["data"].is_null());

    post_binance_com_start(PORT).await;

    let funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=funds",
    )
    .await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["adapter_id"], "binance_com");
    assert_eq!(funds["book_id"], "binance-com-usdm");
    assert_eq!(funds["data"]["identity"]["family"], "account");
    assert_eq!(funds["data"]["identity"]["capability_id"], "funds");
    assert_eq!(funds["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(funds["provenance_path"], "/fapi/v3/balance");
    assert_eq!(funds["provenance_adapter_id"], "binance_com");
    assert!(
        !funds["data"].is_null(),
        "empty success with data:null is a fail"
    );
    assert!(funds["data"]["unrealized_pnl"].is_null());

    let by_asset = holdings_by_asset(&funds);
    assert_eq!(by_asset["USDT"]["free"], 23.72);
    assert_eq!(by_asset["USDT"]["locked"], 0.0);
    assert!(
        !by_asset.contains_key("BNB"),
        "availableBalance <= 0 must not appear on obtain(funds)"
    );
    assert_no_secrets(&funds.to_string());

    let positions = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=positionbook",
    )
    .await;
    assert_eq!(positions["status"], "success");
    assert_eq!(positions["book_id"], "binance-com-usdm");
    assert_eq!(positions["data"]["identity"]["capability_id"], "positions");
    assert_eq!(positions["provenance_path"], "/fapi/v3/positionRisk");
    assert_eq!(positions["data"]["position_count"], 1);
    let rows = positions["data"]["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["symbol"], "ETHUSDT");
    assert_eq!(rows[0]["net_qty"], 2.0);
    assert_ne!(rows[0]["net_qty"], 1.0, "qty is 2 not 1");
    assert_eq!(rows[0]["segment"], "usdm");
    assert_no_secrets(&positions.to_string());

    let forceorder = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=forceorder",
    )
    .await;
    assert_lossy_forceorder(&forceorder, "idle");
    assert_no_secrets(&forceorder.to_string());

    let spot = obtain(PORT, "adapter=binance_com&operation=funds").await;
    assert!(
        (spot["status"] == "unavailable" && spot["data"].is_null())
            || (spot["book_id"] != "binance-com-usdm" && usdt_free(&spot) != Some(23.72)),
        "DualNoBlend: USDM USDT 23.72 must not paint shipping spot: {spot}"
    );
    assert_ne!(spot["book_id"], "binance-com-usdm");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_usdm_empty_balance_is_success_with_empty_holdings() {
    let server = MockServer::start().await;
    mount_hmac_get(&server, "/fapi/v3/balance", "[]").await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;
    post_binance_com_start(PORT).await;

    let funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=funds",
    )
    .await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["book_id"], "binance-com-usdm");
    assert!(
        !funds["data"].is_null(),
        "empty success with data:null is a fail: {funds}"
    );
    let holdings = funds["data"]["holdings"].as_array().expect("holdings");
    assert!(
        holdings.is_empty(),
        "empty balance must be holdings []: {funds}"
    );
    assert!(funds["data"]["unrealized_pnl"].is_null());
    assert_no_secrets(&funds.to_string());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_usdm_quotes_is_unsupported() {
    let server = MockServer::start().await;
    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;
    post_binance_com_start(PORT).await;

    let quotes = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=quotes",
    )
    .await;
    assert_eq!(quotes["status"], "unsupported");
    assert_eq!(quotes["book_id"], "binance-com-usdm");
    assert!(quotes["data"].is_null());
    assert_ne!(quotes["status"], "success");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_usdm_forceorder_with_event_is_observing_not_complete() {
    let server = MockServer::start().await;
    mount_hmac_get(&server, "/fapi/v1/forceOrders", USDM_FORCE_ORDERS_OBSERVING).await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;
    post_binance_com_start(PORT).await;

    let forceorder = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-usdm&operation=forceorder",
    )
    .await;
    assert_lossy_forceorder(&forceorder, "observing");
    let wire = forceorder.to_string();
    assert!(
        wire.contains("BTCUSDT") || forceorder["data"].is_object() || forceorder["data"].is_array(),
        "observing force-order must keep data present: {forceorder}"
    );
    assert_no_secrets(&wire);

    handle.abort();
}
