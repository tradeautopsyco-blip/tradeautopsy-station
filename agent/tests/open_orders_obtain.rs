//! S6-F4 — `obtain(orderbook)` for Binance COM spot and Kotak cash/NFO books.
//!
//! Seams after identity-only Start:
//! - `GET /api/station/obtain?adapter=binance_com&operation=orderbook`
//!   kick: host-mediated `GET /api/v3/openOrders` (USER_DATA) via wiremock
//! - `GET /api/station/obtain?book=kotak-nse-bse-cash&operation=orderbook`
//!   and `…&book=kotak-nse-nfo&operation=orderbook`
//!   kick: host-mediated `GET /quick/user/orders` via wiremock
//!
//! Not a planted AccountBook slot. Not live venues.
//! Ref: `docs/reference/crypto/binance-global/spot/REST.md` Current Open Orders.

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

const BINANCE_PORT: u16 = 19_652;
const KOTAK_PORT: u16 = 19_653;
const API_KEY: &str = "TA_SPOT_ORDERS_KEY_NEVER_IN_OBTAIN";
const API_SECRET: &str = "TA_SPOT_ORDERS_SECRET_NEVER_IN_OBTAIN";
const CONSUMER_KEY: &str = "TA_KOTAK_ORDERS_CONSUMER_NEVER_IN_OBTAIN";
const TRADE_TOKEN: &str = "TA_KOTAK_ORDERS_TOKEN_NEVER_IN_OBTAIN";
const SID: &str = "TA_KOTAK_ORDERS_SID_NEVER_IN_OBTAIN";

fn open_orders_body() -> serde_json::Value {
    serde_json::json!([
        {
            "orderId": 88001,
            "symbol": "MOCKBTCUSDT",
            "side": "BUY",
            "price": "65000.00",
            "origQty": "0.01000000"
        }
    ])
}

fn kotak_orders_body() -> serde_json::Value {
    serde_json::json!({
        "stat": "Ok",
        "stCode": 200,
        "data": [
            {
                "nOrdNo": "OPEN-CASH-1",
                "trdSym": "IDBI-EQ",
                "sym": "IDBI",
                "exSeg": "nse_cm",
                "prod": "CNC",
                "trnsTp": "B",
                "qty": 1,
                "prc": "90.00",
                "unFldSz": 1,
                "ordSt": "open"
            },
            {
                "nOrdNo": "OPEN-NFO-1",
                "trdSym": "NIFTY26SEPFUT",
                "sym": "NIFTY",
                "exSeg": "nse_fo",
                "prod": "NRML",
                "trnsTp": "S",
                "qty": 65,
                "prc": "24200.00",
                "unFldSz": 65,
                "ordSt": "open"
            },
            {
                "nOrdNo": "DONE-CASH-1",
                "trdSym": "PNB-EQ",
                "sym": "PNB",
                "exSeg": "nse_cm",
                "prod": "CNC",
                "trnsTp": "S",
                "qty": 2,
                "prc": "0.00",
                "unFldSz": 0,
                "ordSt": "complete"
            }
        ]
    })
}

async fn post_start(port: u16, slug: &str) {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&identity_start_body(slug)).expect("json");
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

fn binance_opts(base_url: String) -> TestAgentOptions {
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

fn kotak_opts(base_url: String) -> TestAgentOptions {
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
async fn obtain_spot_open_orders_via_mock_egress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/openOrders"))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(open_orders_body()))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(BINANCE_PORT, binance_opts(server.uri()));
    wait_ready(BINANCE_PORT).await;

    let dark = obtain(BINANCE_PORT, "adapter=binance_com&operation=orderbook").await;
    assert_eq!(dark["status"], "unavailable");
    assert!(dark["data"].is_null());

    post_start(BINANCE_PORT, "binance_com").await;

    let orders = obtain(BINANCE_PORT, "adapter=binance_com&operation=orderbook").await;
    assert_eq!(orders["status"], "success");
    assert_eq!(orders["adapter_id"], "binance_com");
    assert_eq!(orders["book_id"], "binance-com-spot");
    assert_eq!(orders["data"]["identity"]["family"], "account");
    assert_eq!(orders["data"]["identity"]["capability_id"], "orders");
    assert_eq!(orders["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(orders["provenance_path"], "/api/v3/openOrders");
    assert_eq!(orders["data"]["order_count"], 1);
    assert_eq!(orders["data"]["rows"][0]["order_id"], "88001");
    assert_eq!(orders["data"]["rows"][0]["symbol"], "MOCKBTCUSDT");
    assert_eq!(orders["data"]["rows"][0]["side"], "BUY");
    assert_eq!(orders["data"]["rows"][0]["qty"], 0.01);
    assert_eq!(orders["data"]["rows"][0]["price"], 65000.0);

    let wire = orders.to_string();
    assert!(!wire.contains(API_KEY), "apiKey must not leak on obtain");
    assert!(!wire.contains(API_SECRET), "apiSecret must not leak");
    assert!(!wire.contains("signature"), "HMAC signature must not leak");

    let again = obtain(BINANCE_PORT, "adapter=binance_com&operation=orderbook").await;
    assert_eq!(again["status"], "success");
    assert_eq!(again["data"]["rows"][0]["order_id"], "88001");
    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        1,
        "fresh orders slot must skip a second GET"
    );

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_kotak_open_orders_via_mock_egress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/quick/user/orders"))
        .and(header("Auth", TRADE_TOKEN))
        .and(header("Sid", SID))
        .respond_with(ResponseTemplate::new(200).set_body_json(kotak_orders_body()))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(KOTAK_PORT, kotak_opts(server.uri()));
    wait_ready(KOTAK_PORT).await;

    let dark = obtain(
        KOTAK_PORT,
        "adapter=kotak_neo&book=kotak-nse-bse-cash&operation=orderbook",
    )
    .await;
    assert_eq!(dark["status"], "unavailable");

    post_start(KOTAK_PORT, "kotak_neo").await;

    let cash = obtain(
        KOTAK_PORT,
        "adapter=kotak_neo&book=kotak-nse-bse-cash&operation=orderbook",
    )
    .await;
    assert_eq!(cash["status"], "success");
    assert_eq!(cash["book_id"], "kotak-nse-bse-cash");
    assert_eq!(cash["data"]["identity"]["capability_id"], "orders");
    assert_eq!(cash["provenance_path"], "/quick/user/orders");
    assert_eq!(cash["data"]["order_count"], 1);
    assert_eq!(cash["data"]["rows"][0]["order_id"], "OPEN-CASH-1");
    assert_eq!(cash["data"]["rows"][0]["symbol"], "IDBI");
    assert_eq!(cash["data"]["rows"][0]["segment"], "nse_cm");
    assert_eq!(cash["data"]["rows"][0]["unfilled_qty"], 1.0);
    let cash_ids: Vec<&str> = cash["data"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|r| r["order_id"].as_str())
        .collect();
    assert!(
        !cash_ids.contains(&"DONE-CASH-1"),
        "complete unFldSz=0 rows must not appear"
    );
    assert!(
        !cash_ids.contains(&"OPEN-NFO-1"),
        "nse_fo rows must not leak onto the cash book"
    );

    let nfo = obtain(
        KOTAK_PORT,
        "adapter=kotak_neo&book=kotak-nse-nfo&operation=orderbook",
    )
    .await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo["data"]["order_count"], 1);
    assert_eq!(nfo["data"]["rows"][0]["order_id"], "OPEN-NFO-1");
    assert_eq!(nfo["data"]["rows"][0]["symbol"], "NIFTY26SEPFUT");
    assert_eq!(nfo["data"]["rows"][0]["segment"], "nse_fo");
    assert_eq!(nfo["data"]["rows"][0]["product"], "NRML");

    let wire = format!("{cash}{nfo}");
    assert!(!wire.contains(CONSUMER_KEY));
    assert!(!wire.contains(TRADE_TOKEN));
    assert!(!wire.contains(SID));

    let again = obtain(
        KOTAK_PORT,
        "adapter=kotak_neo&book=kotak-nse-bse-cash&operation=orderbook",
    )
    .await;
    assert_eq!(again["status"], "success");
    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        1,
        "fresh orders slot must skip a second GET"
    );

    handle.abort();
}
