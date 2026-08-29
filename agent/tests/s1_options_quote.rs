//! Binance options last (`binance-com-options`) on Station loopback (CI). No live eapi.
//! Lock: `locks/binance-com-options.md` — Way 3 last-only.

mod common;

use common::{spawn_test_agent, spawn_test_agent_with_options, wait_ready, TestAgentOptions};

async fn wait_for_quote_route(port: u16) {
    wait_ready(port).await;
}

#[tokio::test]
async fn options_obtain_empty_tickbook_is_unavailable_not_last_zero() {
    const PORT: u16 = 19_500;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "unavailable");
    assert!(obtain["data"].is_null());
    assert_ne!(obtain["data"]["last"], "0");
    assert_eq!(obtain["adapter_id"], "binance_com");
    assert_eq!(obtain["book_id"], "binance-com-options");
    assert_eq!(obtain["operation"], "quotes");

    // Last-only lock: optionchain is not implemented on this book.
    let chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=optionchain"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("optionchain")
        .json()
        .await
        .expect("json");
    assert_eq!(chain["status"], "unsupported");
    assert!(chain["data"].is_null());

    let slug_only: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("spot obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(slug_only["book_id"], "binance-com-spot");

    let nfo: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&book=kotak-nse-nfo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(nfo["book_id"], "kotak-nse-nfo");

    let cash: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=kotak_neo&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("cash obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(cash["book_id"], "kotak-nse-bse-cash");

    let manifests: serde_json::Value = client
        .get(format!("http://127.0.0.1:{PORT}/api/station/manifest"))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("manifest")
        .json()
        .await
        .expect("json");
    let list = manifests["manifests"].as_array().unwrap();
    let ids: Vec<&str> = list
        .iter()
        .filter_map(|m| m["manifest_id"].as_str())
        .collect();
    assert!(ids.contains(&"binance_com.s1.v1"));
    assert!(ids.contains(&"binance_com.options.v1"));
    let first_binance = list
        .iter()
        .find(|m| m["adapter_id"] == "binance_com")
        .expect("binance_com");
    assert_eq!(first_binance["book_id"], "binance-com-spot");
    let options = list
        .iter()
        .find(|m| m["book_id"] == "binance-com-options")
        .expect("options book");
    assert_eq!(options["manifest_id"], "binance_com.options.v1");
    assert_eq!(options["implemented"], serde_json::json!(["quotes"]));
    assert_ne!(
        options["implemented"],
        serde_json::json!(["quotes", "optionchain"])
    );

    handle.abort();
}

#[tokio::test]
async fn planted_options_ticker_last_stays_mixed_case() {
    const PORT: u16 = 19_501;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["adapter_id"], "binance_com");
    assert_eq!(obtain["book_id"], "binance-com-options");
    assert_eq!(obtain["data"]["last"], "1.23");
    assert_ne!(obtain["data"]["last"], "0");
    assert_eq!(obtain["data"]["instrument_id"], "BTC-200730-9000-C");
    assert_ne!(obtain["data"]["instrument_id"], "btc-200730-9000-c");
    assert_eq!(obtain["data"]["source"], "tickbook");
    assert_eq!(obtain["provenance_adapter_id"], "binance_com");

    let spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("spot obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(spot["status"], "unavailable");
    assert!(spot["data"].is_null());

    let slug_spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("slug obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(slug_spot["book_id"], "binance-com-spot");
    assert_ne!(slug_spot["book_id"], "binance-com-options");
    assert!(
        slug_spot["data"].is_null() || slug_spot["data"]["instrument_id"] != "BTC-200730-9000-C"
    );

    handle.abort();
}

#[tokio::test]
async fn planted_options_last_does_not_light_chain_oi_or_spot() {
    const PORT: u16 = 19_511;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?book=binance-com-options&instrument=BTC"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options chain")
        .json()
        .await
        .expect("json");
    assert_eq!(chain["status"], "unavailable");
    assert!(chain["data"].is_null());

    let spot_chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?book=binance-com-spot&instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("spot chain")
        .json()
        .await
        .expect("json");
    assert_eq!(spot_chain["status"], "unavailable");
    assert!(spot_chain["data"].is_null());

    let oi: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/oi?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options oi")
        .json()
        .await
        .expect("json");
    assert_eq!(oi["status"], "unavailable");
    assert!(oi["data"].is_null());

    let obtain_chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=optionchain"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain optionchain")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain_chain["status"], "unsupported");
    assert!(obtain_chain["data"].is_null());

    handle.abort();
}
