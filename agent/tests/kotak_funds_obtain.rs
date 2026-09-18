//! S6-F2-Kotak — cash `obtain(funds)` from RMS `POST /quick/user/limits`.
//!
//! Seam: `GET /api/station/obtain?adapter=kotak_neo&operation=funds` after
//! identity-only Start. Kick is host-mediated `POST /quick/user/limits`
//! (SDK default `seg=ALL&exch=ALL&prod=ALL`) pointed at wiremock serving the
//! Limits.md sample keys — not a planted AccountBook slot, not live Kotak.
//! Ref: `docs/reference/india/kotak-neo/FUNDS-LIMITS.md`.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, spawn_test_agent_with_options, wait_ready,
    TestAgentOptions, WireHeaderOverrides, TEST_BROKER_CONNECTION_ID,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    kotak_cash_limits_jdata_body, kotak_nfo_limits_jdata_body, BrokerAdapter,
    BrokerCredentialVault, CountingPollAdapter, CredentialBlob, MemoryBrokerCredentialVault,
};
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PORT: u16 = 19_654;
const CONSUMER_KEY: &str = "TA_KOTAK_FUNDS_CONSUMER_NEVER_IN_OBTAIN";
const TRADE_TOKEN: &str = "TA_KOTAK_FUNDS_TOKEN_NEVER_IN_OBTAIN";
const SID: &str = "TA_KOTAK_FUNDS_SID_NEVER_IN_OBTAIN";
const LIMITS_FIXTURE: &str = include_str!("../fixtures/kotak/quick_user_limits.json");

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

async fn obtain_funds(port: u16) -> serde_json::Value {
    client()
        .get(format!(
            "http://127.0.0.1:{port}/api/station/obtain?adapter=kotak_neo&operation=funds"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("obtain funds")
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
                hs_server_id: "server4".into(),
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

#[tokio::test]
#[serial]
async fn obtain_kotak_funds_via_limits_fixture_egress() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/quick/user/limits"))
        .and(header("Auth", TRADE_TOKEN))
        .and(header("Sid", SID))
        .and(body_string(kotak_nfo_limits_jdata_body()))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIMITS_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/quick/user/limits"))
        .and(header("Auth", TRADE_TOKEN))
        .and(header("Sid", SID))
        .and(body_string(kotak_cash_limits_jdata_body()))
        .respond_with(ResponseTemplate::new(200).set_body_string(LIMITS_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;

    let dark = obtain_funds(PORT).await;
    assert_eq!(
        dark["status"], "unavailable",
        "funds stay dark until identity-only Start attaches the session locator"
    );
    assert!(dark["data"].is_null());

    post_kotak_start(PORT).await;

    let funds = obtain_funds(PORT).await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["adapter_id"], "kotak_neo");
    assert_eq!(funds["book_id"], "kotak-nse-bse-cash");
    assert_eq!(funds["data"]["identity"]["family"], "account");
    assert_eq!(funds["data"]["identity"]["capability_id"], "funds");
    assert_eq!(funds["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(funds["provenance_path"], "/quick/user/limits");
    assert!(funds["data"]["unrealized_pnl"].is_null());
    assert_eq!(funds["data"]["holdings"].as_array().map(Vec::len), Some(1));
    assert_eq!(funds["data"]["holdings"][0]["asset"], "INR");
    assert_eq!(funds["data"]["holdings"][0]["free"], 19.409999999999997);
    assert_eq!(funds["data"]["holdings"][0]["locked"], 18.78);

    let wire = funds.to_string();
    assert!(!wire.contains(CONSUMER_KEY));
    assert!(!wire.contains(TRADE_TOKEN));
    assert!(!wire.contains(SID));
    assert!(
        !wire.contains("SpanMarginPrsnt"),
        "SPAN sample keys must not leak onto obtain(funds)"
    );

    let nfo = client()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=funds"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("nfo funds")
        .json::<serde_json::Value>()
        .await
        .expect("json");
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo["data"]["holdings"][0]["asset"], "INR");
    assert_eq!(nfo["data"]["holdings"][0]["free"], 19.409999999999997);
    assert_eq!(nfo["data"]["holdings"][0]["locked"], 18.78);
    assert!(nfo["data"]["unrealized_pnl"].is_null());
    assert_eq!(nfo["provenance_path"], "/quick/user/limits");
    let nfo_wire = nfo.to_string();
    assert!(
        !nfo_wire.contains("SpanMarginPrsnt"),
        "SPAN sample keys must not leak onto NFO obtain(funds)"
    );

    let again = obtain_funds(PORT).await;
    assert_eq!(again["status"], "success");
    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        2,
        "cash ALL and NFO FO are two POSTs; do not reuse the cash snapshot"
    );

    handle.abort();
}
