//! S1 desk extract is on Station loopback without live Binance (CI).
//!
//! Every `reqwest::Client::new()` here GETs `http://127.0.0.1:{port}/api/station/...`.
//! `spawn_test_agent` uses `test_on_port`: `s1_desk_symbol` unset (no COM WS/klines
//! at boot), `eapi_public_fetch` false, no Start — quote/ltp does not subscribe or
//! dial a venue. Ring 2 lints `src/` only; these tests stay off EXEMPT.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent,
    spawn_test_agent_with_options, wait_ready, TestAgentOptions, WireHeaderOverrides,
};
use serde_json::Value;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, MemoryBrokerCredentialVault,
};

async fn wait_for_quote_route(port: u16) {
    wait_ready(port).await;
}

async fn post_kotak_sync_start(port: u16) {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&identity_start_body("kotak_neo")).expect("json");
    apply_wire_v1(
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
    .expect("kotak start");
    tokio::time::sleep(Duration::from_millis(350)).await;
}

fn kotak_sync_opts(extra: TestAgentOptions) -> TestAgentOptions {
    let vault = seeded_hmac_vault("kotak_neo", "TA_TEST_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    TestAgentOptions {
        runtime_poll_adapter: Some(counter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..extra
    }
}

#[tokio::test]
async fn get_station_quote_unavailable_without_ticks() {
    const PORT: u16 = 19_485;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let url = format!("http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSDT");
    let resp = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("quote extract should reach agent");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], "unavailable");
    assert!(body["data"].is_null());
    assert_eq!(body["identity"]["family"], "market");
    assert_eq!(body["identity"]["capability_id"], "quote");
    assert_eq!(body["identity"]["physics"], "latest_state");
    assert_eq!(body["instrument_id"], "btcusdt");
    assert_eq!(body["book_id"], "binance-com-spot");
    assert_eq!(body["bind_status"], "bound");
    assert_eq!(body["provenance"]["adapter_id"], "binance_com");
    assert_eq!(body["canonical"], false);
    assert_eq!(body["persist_canonical"], false);

    handle.abort();
}

#[tokio::test]
async fn yahoo_history_forbids_canonical() {
    const PORT: u16 = 19_486;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let url =
        format!("http://127.0.0.1:{PORT}/api/station/history?instrument=BTCUSDT&source=yahoo");
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("history")
        .json()
        .await
        .expect("json");

    assert_eq!(body["identity"]["capability_id"], "ohlcv");
    assert_eq!(body["identity"]["physics"], "historical_series");
    assert_eq!(body["status"], "research_segment");
    assert!(body["data"].is_null());
    assert!(body["ineligible"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "rights_forbid_canonical"));
    assert_eq!(body["canonical"], false);
    assert_eq!(body["persist_canonical"], false);

    handle.abort();
}

#[tokio::test]
async fn chain_and_oi_are_unavailable_holes() {
    const PORT: u16 = 19_487;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(chain["status"], "unavailable");
    assert!(chain["data"].is_null());
    assert_eq!(chain["identity"]["capability_id"], "option_chain");
    assert_eq!(chain["identity"]["physics"], "bounded_snapshot");

    let oi: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/oi?instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(oi["status"], "unavailable");
    assert_eq!(oi["identity"]["capability_id"], "open_interest");

    handle.abort();
}

#[tokio::test]
async fn s0_manifest_and_obtain_are_host_owned() {
    const PORT: u16 = 19_488;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let manifests: serde_json::Value = client
        .get(format!("http://127.0.0.1:{PORT}/api/station/manifest"))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let list = manifests["manifests"].as_array().unwrap();
    let ids: Vec<&str> = list
        .iter()
        .filter_map(|m| m["manifest_id"].as_str())
        .collect();
    assert!(ids.contains(&"binance_com.s1.v1"));
    assert!(ids.contains(&"kotak_neo.s1k.v1"));
    assert!(ids.contains(&"kotak_neo.nfo.v1"));

    let first_binance = list
        .iter()
        .find(|m| m["adapter_id"] == "binance_com")
        .expect("binance_com manifest");
    assert_eq!(first_binance["manifest_id"], "binance_com.s1.v1");
    assert_eq!(first_binance["book_id"], "binance-com-spot");
    assert!(first_binance["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "quotes"));

    let kotak_cash = list
        .iter()
        .find(|m| m["manifest_id"] == "kotak_neo.s1k.v1")
        .expect("kotak cash manifest");
    assert_eq!(kotak_cash["book_id"], "kotak-nse-bse-cash");
    assert!(kotak_cash["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "quotes"));

    let kotak_nfo = list
        .iter()
        .find(|m| m["manifest_id"] == "kotak_neo.nfo.v1")
        .expect("kotak nfo manifest");
    assert_eq!(kotak_nfo["book_id"], "kotak-nse-nfo");
    assert!(kotak_nfo["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "quotes"));
    assert!(kotak_nfo["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "instruments"));
    assert!(kotak_nfo["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "optionchain"));

    let quotes: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(quotes["status"], "unavailable");
    assert!(quotes["data"].is_null());
    assert_eq!(quotes["adapter_id"], "binance_com");
    assert_eq!(quotes["book_id"], "binance-com-spot");
    assert_eq!(quotes["operation"], "quotes");

    let quotes_by_book: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=binance-com-spot&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(quotes_by_book["status"], quotes["status"]);
    assert_eq!(quotes_by_book["adapter_id"], "binance_com");
    assert_eq!(quotes_by_book["book_id"], "binance-com-spot");

    let quotes_both: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(quotes_both["status"], quotes["status"]);
    assert_eq!(quotes_both["adapter_id"], "binance_com");
    assert_eq!(quotes_both["book_id"], "binance-com-spot");

    let usdm: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=binance-com-usdm&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(usdm["status"], "unavailable");
    assert_eq!(usdm["book_id"], "binance-com-usdm");
    assert!(usdm["data"].is_null());
    assert_ne!(usdm["status"], "unsupported");

    let disagree: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=kotak-nse-bse-cash&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(disagree["status"], "unsupported");

    let funds: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=funds"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(funds["status"], "unavailable");
    assert!(funds["data"].is_null());
    assert_eq!(funds["provenance_adapter_id"], "binance_com");

    let kotak_quotes: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(kotak_quotes["status"], "unavailable");
    assert!(kotak_quotes["data"].is_null());
    assert_eq!(kotak_quotes["adapter_id"], "kotak_neo");
    assert_eq!(kotak_quotes["book_id"], "kotak-nse-bse-cash");

    let kotak_instruments: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=instruments"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(kotak_instruments["status"], "unavailable");
    assert!(kotak_instruments["data"].is_null());

    let kotak_history: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(kotak_history["status"], "unsupported");
    assert!(kotak_history["data"].is_null());

    let kotak_depth: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(kotak_depth["status"], "unavailable");
    assert!(kotak_depth["data"].is_null());

    let kotak_optionchain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=optionchain"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(kotak_optionchain["status"], "unsupported");
    assert!(kotak_optionchain["data"].is_null());

    let nfo_quotes_empty: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(nfo_quotes_empty["status"], "unavailable");
    assert!(nfo_quotes_empty["data"].is_null());
    assert_ne!(nfo_quotes_empty["data"]["last"], "0");
    assert_eq!(nfo_quotes_empty["book_id"], "kotak-nse-nfo");

    let nfo_optionchain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=optionchain"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(nfo_optionchain["status"], "unavailable");
    assert!(nfo_optionchain["data"].is_null());

    let nfo_instruments_empty: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=instruments"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(nfo_instruments_empty["status"], "unavailable");
    assert!(nfo_instruments_empty["data"].is_null());
    assert_eq!(nfo_instruments_empty["book_id"], "kotak-nse-nfo");

    let binance_history: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(binance_history["status"], "unavailable");
    assert!(binance_history["data"].is_null());
    assert_eq!(binance_history["adapter_id"], "binance_com");
    assert_eq!(binance_history["operation"], "history");

    handle.abort();
}

#[tokio::test]
async fn planted_binance_history_is_licensed_series_not_yahoo() {
    const PORT: u16 = 19_489;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_s2_history: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["adapter_id"], "binance_com");
    assert_eq!(obtain["data"]["last_close"], "0.01590000");
    assert_eq!(obtain["data"]["identity"]["physics"], "historical_series");
    assert_eq!(obtain["data"]["source"], "binance_klines");
    assert_eq!(obtain["provenance_adapter_id"], "binance_com");

    let licensed: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/history?instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(licensed["status"], "success");
    assert_eq!(licensed["provenance"]["adapter_id"], "binance_com");
    assert_eq!(licensed["data"]["last_close"], "0.01590000");
    assert!(licensed["ineligible"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v != "insufficient_retention"));

    let yahoo: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/history?instrument=BTCUSDT&source=yahoo"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(yahoo["status"], "research_segment");
    assert!(yahoo["data"].is_null());
    assert!(yahoo["ineligible"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "rights_forbid_canonical"));

    let bad_interval: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/history?instrument=BTCUSDT&interval=2m"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(bad_interval["data"].is_null());
    assert!(bad_interval["ineligible"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "unsupported_interval"));

    let bad_limit: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/history?instrument=BTCUSDT&limit=1001"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(bad_limit["data"].is_null());
    assert!(bad_limit["ineligible"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "unsupported_range"));

    let other: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/history?instrument=ETHUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(other["status"], "unavailable");
    assert!(other["data"].is_null());
    assert_eq!(other["instrument_id"], "ethusdt");
    assert_ne!(other["data"]["last_close"], "0.01590000");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn planted_nfo_quote_last_is_not_cash_last() {
    const PORT: u16 = 19_490;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_s1k_fixtures: true,
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();

    let nfo: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C12345"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo quote")
        .json()
        .await
        .expect("nfo json");
    assert_ne!(nfo["status"], "unavailable");
    assert!(!nfo["data"].is_null());
    assert_eq!(nfo["instrument_id"], "nse_fo|12345");
    assert_eq!(nfo["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo["bind_status"], "bound");
    assert_eq!(nfo["provenance"]["adapter_id"], "kotak_neo");
    let nfo_last = nfo["data"]["last"].as_str().expect("nfo last");
    assert_ne!(nfo_last, "0");
    assert!(nfo_last.parse::<f64>().unwrap() > 0.0);

    let cash_obtain_after_nfo_select: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // Book-scoped: NFO selection must not paint cash obtain.
    assert_eq!(cash_obtain_after_nfo_select["status"], "unavailable");
    assert_eq!(
        cash_obtain_after_nfo_select["book_id"],
        "kotak-nse-bse-cash"
    );
    assert!(cash_obtain_after_nfo_select["data"].is_null());

    let cash: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_cm%7C2885"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("cash quote")
        .json()
        .await
        .expect("cash json");
    assert_ne!(cash["status"], "unavailable");
    assert_eq!(cash["book_id"], "kotak-nse-bse-cash");
    assert_eq!(cash["bind_status"], "bound");
    let cash_last = cash["data"]["last"].as_str().expect("cash last");
    assert_ne!(nfo_last, cash_last);
    assert_eq!(nfo_last, "10.00");
    assert_eq!(cash_last, "1400.50");

    let nfo_obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(nfo_obtain["status"], "success");
    assert_eq!(nfo_obtain["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo_obtain["data"]["last"], "10.00");
    assert_eq!(nfo_obtain["provenance_adapter_id"], "kotak_neo");

    let cash_obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cash_obtain["status"], "success");
    assert_eq!(cash_obtain["book_id"], "kotak-nse-bse-cash");
    assert_eq!(cash_obtain["data"]["last"], "1400.50");
    assert_ne!(cash_obtain["data"]["last"], nfo_obtain["data"]["last"]);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn planted_nfo_quote_without_master_optionchain_unavailable() {
    const PORT: u16 = 19_491;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();

    client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C12345"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind nfo");

    let quotes: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(quotes["status"], "success");
    assert!(!quotes["data"].is_null());
    assert_eq!(quotes["data"]["last"], "10.00");
    assert_eq!(quotes["book_id"], "kotak-nse-nfo");

    let chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=optionchain"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(chain["status"], "unavailable");
    assert!(chain["data"].is_null());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn planted_nfo_instruments_and_optionchain_success_lock_lot() {
    const PORT: u16 = 19_493;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_s1k_fixtures: true,
            plant_kotak_nfo_contracts: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();

    client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C56526"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind nfo contract");

    let nfo: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=instruments"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo["adapter_id"], "kotak_neo");
    assert_eq!(nfo["data"]["contract_count"], 2);
    assert_eq!(nfo["data"]["identity"]["family"], "reference");
    assert_eq!(
        nfo["data"]["identity"]["capability_id"],
        "derivative_contracts"
    );
    assert_eq!(nfo["data"]["identity"]["physics"], "bounded_snapshot");
    let rows = nfo["data"]["rows"].as_array().expect("rows");
    let nifty = rows
        .iter()
        .find(|row| row["instrument_id"] == "nse_fo|56526")
        .expect("nse_fo|56526 row");
    let lot = &nifty["lot"];
    assert!(lot.is_number(), "lot must be a JSON number: {lot}");
    assert_eq!(lot.as_i64(), Some(65));
    let dumped = nfo.to_string();
    assert!(!dumped.contains("57500"), "no ghost strike: {dumped}");

    let chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=optionchain"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(chain["status"], "success");
    assert_eq!(chain["data"]["identity"]["capability_id"], "option_chain");
    let chain_rows = chain["data"]["rows"].as_array().expect("chain rows");
    let chain_nifty = chain_rows
        .iter()
        .find(|row| row["instrument_id"] == "nse_fo|56526")
        .expect("NIFTY chain row");
    assert_eq!(chain_nifty["lot"], 65);
    let dumped_chain = chain.to_string();
    assert!(
        !dumped_chain.contains("57500"),
        "no ghost strike: {dumped_chain}"
    );

    let glance: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?book=kotak-nse-nfo&instrument=NIFTY"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(glance["status"], "success");
    assert_eq!(glance["identity"]["capability_id"], "option_chain");
    assert_eq!(glance["identity"]["physics"], "bounded_snapshot");
    assert_eq!(glance["data"]["rows"][0]["instrument_id"], "nse_fo|56526");

    let cash_chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?book=kotak-nse-bse-cash&instrument=NIFTY"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cash_chain["status"], "unavailable");
    assert!(cash_chain["data"].is_null());

    let cash: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=instruments"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cash["book_id"], "kotak-nse-bse-cash");
    assert_eq!(cash["status"], "success");
    assert_eq!(cash["data"]["symbol_count"], 2);
    assert!(cash["data"]["rows"].is_null());

    handle.abort();
}

#[tokio::test]
async fn nfo_plant_does_not_steal_cash_adapter_obtain() {
    const PORT: u16 = 19_492;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let cash: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cash["book_id"], "kotak-nse-bse-cash");
    assert_eq!(cash["status"], "unavailable");
    assert!(cash["data"].is_null());
    assert_ne!(cash["data"]["last"], "10.00");
    assert_ne!(cash["data"]["last"], "0");

    handle.abort();
}

/// O1 on the wire: `obtain?book=kotak-nse-nfo&operation=open_interest` Successes
/// from `open_int` on the same `quote_type=all` body that feeds last. The `oi`
/// slice spellings must not appear. Depth without a planted snapshot stays dark.
#[tokio::test]
#[serial]
async fn planted_nfo_open_interest_lights_from_open_int_only() {
    const PORT: u16 = 19_494;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();

    // Select the contract the way the desk does.
    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C12345"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind nfo")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["instrument_id"], "nse_fo|12345");

    let oi: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=open_interest"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo oi obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(oi["status"], "success");
    assert_eq!(oi["book_id"], "kotak-nse-nfo");
    assert_eq!(oi["data"]["identity"]["capability_id"], "open_interest");
    assert_eq!(oi["data"]["identity"]["physics"], "latest_state");
    assert_ne!(oi["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(oi["data"]["field"], "open_int");
    assert_eq!(oi["data"]["instrument_id"], "nse_fo|12345");
    let value = oi["data"]["open_interest"].as_str().expect("open interest");
    assert!(value.parse::<f64>().unwrap() >= 0.0);
    assert_eq!(oi["provenance_adapter_id"], "kotak_neo");
    let wire = oi.to_string();
    // Raw `oi` slice spellings stay unbound; mapped session fields use `oi_session_*`.
    for unbound in ["oi_las", "oi_high", "oi_low", "dOpenInterest"] {
        assert!(!wire.contains(unbound), "{unbound} must stay unbound");
    }
    for session_key in ["oi_session_las", "oi_session_high", "oi_session_low"] {
        assert!(
            oi["data"].get(session_key).is_none(),
            "{session_key} requires plant_kotak_nfo_oi_session"
        );
    }
    // OI is not a price: it must not be published as last.
    assert!(oi["data"].get("last").is_none());

    // The glance route lights the same reading.
    let glance: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/oi?book=kotak-nse-nfo&instrument=nse_fo%7C12345"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo oi glance")
        .json()
        .await
        .expect("json");
    assert_eq!(glance["status"], "success");
    assert_eq!(glance["data"]["field"], "open_int");

    // NFO depth is implemented but dark until a snapshot lands.
    let depth: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo depth obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(depth["status"], "unavailable");
    assert_ne!(depth["status"], "unsupported");
    assert!(depth["data"].is_null());

    handle.abort();
}

/// When the `quote_type=oi` slice is planted, obtain adds supplementary
/// `oi_session_*` fields without overwriting `open_interest` from `open_int`.
#[tokio::test]
#[serial]
async fn planted_nfo_oi_session_adds_supplementary_fields_without_overwriting_open_int() {
    const PORT: u16 = 19_540;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_nfo_quote: true,
            plant_kotak_nfo_oi_session: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();

    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C12345"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind nfo")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["instrument_id"], "nse_fo|12345");

    let oi: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=open_interest"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo oi obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(oi["status"], "success");
    assert_eq!(oi["data"]["field"], "open_int");
    assert_eq!(oi["data"]["open_interest"], "480750");
    assert_eq!(oi["data"]["oi_session_las"], "475000");
    assert_eq!(oi["data"]["oi_session_high"], "500000");
    assert_eq!(oi["data"]["oi_session_low"], "450000");
    assert_eq!(oi["data"]["oi_session_provenance"]["quote_type"], "oi");
    assert!(oi["data"]["oi_session_provenance"]["path"]
        .as_str()
        .expect("path")
        .ends_with("/oi"));
    let wire = oi.to_string();
    for unbound in ["oi_las", "oi_high", "oi_low", "dOpenInterest"] {
        assert!(
            !wire.contains(unbound),
            "{unbound} must stay unbound on the wire"
        );
    }

    handle.abort();
}

/// Planted NFO depth serves a REST bounded snapshot on the named book.
#[tokio::test]
#[serial]
async fn planted_nfo_depth_serves_a_bounded_snapshot_on_the_nfo_book() {
    const PORT: u16 = 19_539;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_nfo_depth: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();

    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C56526&book=kotak-nse-nfo"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind nfo depth contract")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["instrument_id"], "nse_fo|56526");
    assert_eq!(bind["book_id"], "kotak-nse-nfo");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain nfo depth")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["book_id"], "kotak-nse-nfo");
    assert_eq!(obtain["provenance_adapter_id"], "kotak_neo");
    let data = &obtain["data"];
    assert_eq!(data["identity"]["capability_id"], "order_book");
    assert_eq!(data["identity"]["physics"], "bounded_snapshot");
    assert_ne!(data["identity"]["physics"], "ordered_state");
    assert_eq!(data["source"], "rest_snapshot");
    assert_eq!(data["instrument_id"], "nse_fo|56526");
    assert!(!data["bids"].as_array().unwrap().is_empty());
    assert!(!data["asks"].as_array().unwrap().is_empty());
    assert_eq!(data["bids"][0]["orders"], "3");

    handle.abort();
}

/// Without the plant there is no reading, so OI is Unavailable — never
/// `unsupported` (it is implemented) and never an `open_interest: 0` Success.
#[tokio::test]
async fn nfo_open_interest_without_a_reading_is_unavailable_not_zero() {
    const PORT: u16 = 19_495;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let oi: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=open_interest"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo oi obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(oi["status"], "unavailable");
    assert_ne!(oi["status"], "unsupported");
    assert_ne!(oi["status"], "success");
    assert!(oi["data"].is_null());
    assert!(!oi.to_string().contains("\"open_interest\":\"0\""));

    handle.abort();
}

#[tokio::test]
async fn planted_binance_spot_funds_obtain_success() {
    const PORT: u16 = 19_496;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_spot_funds: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let funds: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=funds"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["adapter_id"], "binance_com");
    assert_eq!(funds["book_id"], "binance-com-spot");
    assert_eq!(funds["data"]["identity"]["capability_id"], "funds");
    assert_eq!(funds["data"]["holdings"][0]["asset"], "BTC");
    assert_eq!(funds["provenance_path"], "/api/v3/account");

    handle.abort();
}

/// obtain search: short q → empty success; unknown book → unsupported.
#[tokio::test]
#[serial]
async fn obtain_search_short_query_returns_empty_rows() {
    const PORT: u16 = 19_520;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_contracts: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let short: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=search&q=n"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("search short q")
        .json()
        .await
        .expect("json");
    assert_eq!(short["status"], "success");
    assert_eq!(short["data"]["row_count"], 0);
    assert!(short["data"]["rows"].as_array().unwrap().is_empty());

    let unknown: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=binance-com-not-a-book&operation=search&q=nifty"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("unknown book search")
        .json()
        .await
        .expect("json");
    assert_eq!(unknown["status"], "unsupported");
    assert!(unknown["data"].is_null());

    handle.abort();
}

/// Planted NFO master → obtain search returns prefix hits on the NFO book only.
#[tokio::test]
#[serial]
async fn obtain_search_nfo_master_returns_prefix_hits() {
    const PORT: u16 = 19_521;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_contracts: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let hits: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=kotak-nse-nfo&operation=search&q=NIFTY"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("nfo search")
        .json()
        .await
        .expect("json");
    assert_eq!(hits["status"], "success");
    assert_eq!(
        hits["data"]["identity"]["capability_id"],
        "instrument_search"
    );
    assert_eq!(hits["data"]["identity"]["physics"], "bounded_snapshot");
    assert!(hits["data"]["row_count"].as_u64().unwrap_or(0) > 0);
    let first = &hits["data"]["rows"][0];
    assert_eq!(first["segment"], "nse_fo");

    handle.abort();
}

/// Glance depth with an empty DepthBook is Unavailable — never empty Success.
#[tokio::test]
async fn depth_glance_empty_book_is_unavailable() {
    const PORT: u16 = 19_560;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();
    let body: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/depth?book=binance-com-spot&instrument=BTCUSDT"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("depth glance")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unavailable");
    assert!(body["data"].is_null());
    assert_eq!(body["rights"]["display"], true);
    assert_ne!(body["status"], "success");
    handle.abort();
}

/// Planted NFO depth is Success on the named book via glance.
#[tokio::test]
#[serial]
async fn planted_nfo_depth_glance_is_a_bounded_snapshot() {
    const PORT: u16 = 19_561;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_nfo_depth: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();
    let _bind: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C56526&book=kotak-nse-nfo"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("bind")
        .json()
        .await
        .expect("json");
    let body: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/depth?book=kotak-nse-nfo&instrument=nse_fo%7C56526"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("nfo depth glance")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "success");
    assert_eq!(body["identity"]["physics"], "bounded_snapshot");
    assert_ne!(body["identity"]["physics"], "ordered_state");
    assert!(!body["data"]["bids"].as_array().unwrap().is_empty());
    assert_eq!(body["data"]["bids"][0]["orders"], "3");
    assert_eq!(body["rights"]["display"], true);
    handle.abort();
}

/// Cash s1k fixture plants REST depth — glance must not say synced.
#[tokio::test]
#[serial]
async fn planted_cash_depth_glance_is_rest_not_synced() {
    const PORT: u16 = 19_562;
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_sync_opts(TestAgentOptions {
            plant_kotak_s1k_fixtures: true,
            ..TestAgentOptions::default()
        }),
    );
    wait_for_quote_route(PORT).await;
    post_kotak_sync_start(PORT).await;
    let client = reqwest::Client::new();
    let body: Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/depth?book=kotak-nse-bse-cash&instrument=nse_cm%7C2885"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("cash depth glance")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "success");
    assert_eq!(body["identity"]["physics"], "bounded_snapshot");
    assert_ne!(body["identity"]["physics"], "synced");
    assert_eq!(body["data"]["bids"][0]["price"], "1400.00");
    handle.abort();
}

/// COM gap stamps Unusable, not Unavailable and not last-good Success.
#[tokio::test]
async fn spot_depth_glance_after_gap_is_unusable_not_unavailable() {
    const PORT: u16 = 19_563;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_spot_depth_unusable: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let body: Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/depth?book=binance-com-spot&instrument=BTCUSDT"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("spot gap glance")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unusable");
    assert_ne!(body["status"], "unavailable");
    assert_ne!(body["status"], "success");
    assert!(body["data"].is_null());
    handle.abort();
}

/// Options slot must not serve a spot ladder.
#[tokio::test]
async fn options_depth_glance_does_not_leak_spot() {
    const PORT: u16 = 19_564;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_spot_depth_unusable: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let body: Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/depth?book=binance-com-options&instrument=BTCUSDT"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("leak glance")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unavailable");
    assert!(body["data"].is_null());
    handle.abort();
}
