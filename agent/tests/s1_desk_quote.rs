//! S1 desk extract is on Station loopback without live Binance (CI). WS is env-gated.

mod common;

use common::{spawn_test_agent, spawn_test_agent_with_options, wait_ready, TestAgentOptions};

async fn wait_for_quote_route(port: u16) {
    wait_ready(port).await;
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
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["manifest_id"], "binance_com.s1.v1");
    assert_eq!(
        list[0]["implemented"],
        serde_json::json!(["quotes", "instruments", "tradebook", "funds", "history"])
    );
    assert_eq!(list[1]["manifest_id"], "kotak_neo.s1k.v1");
    assert_eq!(
        list[1]["implemented"],
        serde_json::json!(["quotes", "instruments", "tradebook", "depth"])
    );
    assert!(list[0]["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "quotes"));
    assert!(list[1]["implemented"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "quotes"));

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
    assert_eq!(quotes["operation"], "quotes");

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
