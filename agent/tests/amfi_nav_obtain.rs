//! S7-06 — AMFI NAV obtain (`product_use=labs`). SpecializedNonBroker, not Kotak.
//!
//! Seam: `GET /api/station/obtain?adapter=amfi&operation=amfi_nav`
//! Mock AMFI HTTP at wiremock (egress/system boundary). Fixture snippet is copied
//! from official `NAVAll.txt` (sheet `docs/research/sheets/amfi.md`, fetch 2026-09-17 IST).

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    wait_ready, TestAgentOptions, WireHeaderOverrides,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{BrokerAdapter, BrokerCredentialVault, CountingPollAdapter};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PORT_NAV: u16 = 19_880;
const PORT_FENCE: u16 = 19_881;
const PORT_QUOTE: u16 = 19_882;

/// Official file shape + one published scheme row (not a 100-fund invention).
/// Source: https://www.amfiindia.com/spages/NAVAll.txt fetched 2026-09-17 IST.
const AMFI_NAV_FIXTURE: &str = "\
Scheme Code;ISIN Div Payout/ ISIN Growth;ISIN Div Reinvestment;Scheme Name;Plan;Option;Net Asset Value;Date\n\
\n\
Open Ended Schemes(Children’s Fund - Childrens' Fund)\n\
\n\
Axis Mutual Fund\n\
\n\
135762;INF846K01WO1;-;Axis Children's Fund;Direct Plan;Growth Option;29.5870;16-Sep-2026\n";

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
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

fn amfi_mock_opts(base_url: String) -> TestAgentOptions {
    TestAgentOptions {
        amfi_nav_base_url: Some(base_url),
        ..TestAgentOptions::default()
    }
}

#[tokio::test]
#[serial]
async fn obtain_amfi_nav_fixture_is_labs_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/spages/NAVAll.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(AMFI_NAV_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_NAV,
        amfi_mock_opts(server.uri()),
    ));
    wait_ready(PORT_NAV).await;

    let nav = obtain(PORT_NAV, "adapter=amfi&operation=amfi_nav").await;
    assert_eq!(nav["status"], "success");
    assert_eq!(nav["product_use"], "labs");
    assert_eq!(nav["provenance_adapter_id"], "amfi");
    assert_eq!(nav["adapter_id"], "amfi");
    assert_eq!(nav["book_id"], "amfi-nav");
    assert_ne!(nav["book_id"], "kotak-nse-bse-cash");
    assert_ne!(nav["book_id"], "kotak-nse-nfo");
    assert_ne!(nav["book_id"], "licensed-history");

    let schemes = nav["data"]["schemes"].as_array().expect("schemes");
    assert!(!schemes.is_empty(), "at least one published scheme NAV");
    let row = &schemes[0];
    assert_eq!(row["scheme_code"], "135762");
    assert_eq!(row["scheme_name"], "Axis Children's Fund");
    assert_eq!(row["nav"], "29.5870");
    assert_eq!(row["date"], "16-Sep-2026");
}

#[tokio::test]
#[serial]
async fn amfi_unknown_host_fence_refuses_not_kotak_book() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_FENCE,
        TestAgentOptions {
            amfi_nav_host: Some("www.nseindia.com".into()),
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT_FENCE).await;

    let nav = obtain(PORT_FENCE, "adapter=amfi&operation=amfi_nav").await;
    assert_ne!(nav["status"], "success");
    assert_eq!(nav["product_use"], "labs");
    assert_eq!(nav["adapter_id"], "amfi");
    assert_eq!(nav["book_id"], "amfi-nav");
    assert_ne!(nav["book_id"], "kotak-nse-bse-cash");
    assert_ne!(nav["book_id"], "kotak-nse-nfo");
    assert_ne!(nav["book_id"], "licensed-history");
    assert!(nav["data"].is_null());
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

#[tokio::test]
#[serial]
async fn kotak_quotes_stay_kotak_neo_when_amfi_present() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/spages/NAVAll.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(AMFI_NAV_FIXTURE))
        .mount(&server)
        .await;

    let vault = seeded_hmac_vault("kotak_neo", "TA_S7_AMFI_QUOTE");
    let counter = Arc::new(CountingPollAdapter::new());
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_QUOTE,
        TestAgentOptions {
            runtime_poll_adapter: Some(counter as Arc<dyn BrokerAdapter>),
            broker_base_poll_ms: 80,
            credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
            plant_kotak_s1k_fixtures: true,
            amfi_nav_base_url: Some(server.uri()),
            kotak_quote_budget: 10,
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT_QUOTE).await;
    post_kotak_start(PORT_QUOTE).await;
    bind_planted_cash_quote(PORT_QUOTE).await;

    let quote = obtain(PORT_QUOTE, "adapter=kotak_neo&operation=quotes").await;
    assert_eq!(quote["status"], "success");
    assert_eq!(quote["provenance_adapter_id"], "kotak_neo");
    assert_eq!(quote["adapter_id"], "kotak_neo");
    assert_eq!(quote["book_id"], "kotak-nse-bse-cash");
    assert_ne!(quote["provenance_adapter_id"], "amfi");
    assert_ne!(quote["book_id"], "amfi-nav");
    assert_ne!(quote["product_use"], "labs");

    let nav = obtain(PORT_QUOTE, "adapter=amfi&operation=amfi_nav").await;
    assert_eq!(nav["status"], "success");
    assert_eq!(nav["product_use"], "labs");
    assert_eq!(nav["provenance_adapter_id"], "amfi");
}
