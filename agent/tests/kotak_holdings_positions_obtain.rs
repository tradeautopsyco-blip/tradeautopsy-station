//! S6-F3 — Kotak cash `obtain(holdings)` and cash/NFO `obtain(positionbook)`
//! via committed fixtures on mocked session egress.
//!
//! Seam: `GET /api/station/obtain?adapter=kotak_neo&operation=holdings`
//! and `…&book=kotak-nse-nfo&operation=positionbook` after identity-only Start.
//! Kick is host-mediated `GET /portfolio/v1/holdings` and `GET /quick/user/positions`
//! pointed at wiremock serving `fixtures/kotak/*.json` — not a planted AccountBook
//! slot, not live Kotak.

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

const PORT: u16 = 19_651;
const CONSUMER_KEY: &str = "TA_KOTAK_HOLDINGS_CONSUMER_NEVER_IN_OBTAIN";
const TRADE_TOKEN: &str = "TA_KOTAK_HOLDINGS_TOKEN_NEVER_IN_OBTAIN";
const SID: &str = "TA_KOTAK_HOLDINGS_SID_NEVER_IN_OBTAIN";
const HS_SERVER_ID: &str = "server4";

const HOLDINGS_FIXTURE: &str = include_str!("../fixtures/kotak/portfolio_holdings.json");
const POSITIONS_FIXTURE: &str = include_str!("../fixtures/kotak/quick_user_positions.json");

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
    assert_eq!(resp.status(), 200, "identity-only Start must succeed");
}

async fn obtain(port: u16, query: &str) -> serde_json::Value {
    client()
        .get(format!("http://127.0.0.1:{port}/api/station/obtain?{query}"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("obtain")
        .json()
        .await
        .expect("obtain json")
}

fn start_opts(base_url: String) -> TestAgentOptions {
    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    vault
        .save(
            "prod",
            "kotak_neo",
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::KotakNeoTotpSession {
                consumer_key: CONSUMER_KEY.into(),
                trade_token: TRADE_TOKEN.into(),
                sid: SID.into(),
                base_url: "https://cis.kotaksecurities.com".into(),
                hs_server_id: HS_SERVER_ID.into(),
                expires_at: None,
            },
        )
        .expect("seed vault");
    TestAgentOptions {
        runtime_poll_adapter: Some(Arc::new(CountingPollAdapter::new()) as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        kotak_private_base_url: Some(base_url),
        ..TestAgentOptions::default()
    }
}

fn assert_no_session_secrets(wire: &str) {
    assert!(
        !wire.contains(CONSUMER_KEY),
        "consumer key must not leak on obtain"
    );
    assert!(
        !wire.contains(TRADE_TOKEN),
        "trade token must not leak on obtain"
    );
    assert!(!wire.contains(SID), "Sid must not leak on obtain");
}

#[tokio::test]
#[serial]
async fn obtain_kotak_holdings_and_positions_via_fixture_egress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/portfolio/v1/holdings"))
        .and(header("Auth", TRADE_TOKEN))
        .and(header("Sid", SID))
        .respond_with(ResponseTemplate::new(200).set_body_string(HOLDINGS_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/quick/user/positions"))
        .and(header("Auth", TRADE_TOKEN))
        .and(header("Sid", SID))
        .respond_with(ResponseTemplate::new(200).set_body_string(POSITIONS_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;

    let dark_holdings = obtain(PORT, "adapter=kotak_neo&operation=holdings").await;
    assert_eq!(
        dark_holdings["status"], "unavailable",
        "holdings stay dark until identity-only Start attaches the session locator"
    );
    assert!(dark_holdings["data"].is_null());

    let dark_positions = obtain(
        PORT,
        "adapter=kotak_neo&book=kotak-nse-bse-cash&operation=positionbook",
    )
    .await;
    assert_eq!(dark_positions["status"], "unavailable");
    assert!(dark_positions["data"].is_null());

    post_kotak_start(PORT).await;

    let holdings = obtain(PORT, "adapter=kotak_neo&operation=holdings").await;
    assert_eq!(holdings["status"], "success");
    assert_eq!(holdings["adapter_id"], "kotak_neo");
    assert_eq!(holdings["book_id"], "kotak-nse-bse-cash");
    assert_eq!(holdings["data"]["identity"]["family"], "account");
    assert_eq!(holdings["data"]["identity"]["capability_id"], "holdings");
    assert_eq!(holdings["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(holdings["provenance_path"], "/portfolio/v1/holdings");
    assert_eq!(holdings["data"]["holding_count"], 3);
    let symbols: Vec<&str> = holdings["data"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|r| r["symbol"].as_str())
        .collect();
    assert_eq!(symbols, ["IDBI", "IDFCFIRSTB", "PNB"]);
    assert!(holdings["data"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .all(|r| r["segment"] == "nse_cm"));
    assert_no_session_secrets(&holdings.to_string());

    let cash_pos = obtain(
        PORT,
        "adapter=kotak_neo&book=kotak-nse-bse-cash&operation=positionbook",
    )
    .await;
    assert_eq!(cash_pos["status"], "success");
    assert_eq!(cash_pos["book_id"], "kotak-nse-bse-cash");
    assert_eq!(cash_pos["data"]["identity"]["capability_id"], "positions");
    assert_eq!(cash_pos["provenance_path"], "/quick/user/positions");
    assert_eq!(cash_pos["data"]["position_count"], 4);
    let nets: Vec<f64> = cash_pos["data"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| r["net_qty"].as_f64().expect("net_qty"))
        .collect();
    assert!(
        nets.iter().all(|n| n.abs() > 0.0),
        "zero-net rows must be dropped: {nets:?}"
    );

    let nfo_pos = obtain(
        PORT,
        "adapter=kotak_neo&book=kotak-nse-nfo&operation=positionbook",
    )
    .await;
    assert_eq!(nfo_pos["status"], "success");
    assert_eq!(nfo_pos["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo_pos["data"]["position_count"], 0);
    assert_eq!(nfo_pos["data"]["rows"].as_array().map(Vec::len), Some(0));
    assert_eq!(nfo_pos["provenance_path"], "/quick/user/positions");
    assert_no_session_secrets(&nfo_pos.to_string());

    let again = obtain(PORT, "adapter=kotak_neo&operation=holdings").await;
    assert_eq!(again["status"], "success");
    assert_eq!(again["data"]["holding_count"], 3);

    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        2,
        "fresh AccountBook slots must skip a second venue GET"
    );

    handle.abort();
}
