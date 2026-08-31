//! S1 desk extract is on Station loopback without live Binance (CI).
//!
//! Every `reqwest::Client::new()` here GETs `http://127.0.0.1:{port}/api/station/...`.
//! `spawn_test_agent` uses `test_on_port`: `s1_desk_symbol` unset (no COM WS/klines
//! at boot), `eapi_public_fetch` false, no Start — quote/ltp does not subscribe or
//! dial a venue. Ring 2 lints `src/` only; these tests stay off EXEMPT.

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
    assert_eq!(usdm["status"], "unsupported");

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
async fn planted_nfo_quote_last_is_not_cash_last() {
    const PORT: u16 = 19_490;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_s1k_fixtures: true,
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
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
    assert_eq!(cash_obtain_after_nfo_select["status"], "success");
    assert_eq!(
        cash_obtain_after_nfo_select["book_id"],
        "kotak-nse-bse-cash"
    );
    assert_eq!(cash_obtain_after_nfo_select["data"]["last"], "1400.50");
    assert_ne!(cash_obtain_after_nfo_select["data"]["last"], nfo_last);

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
async fn planted_nfo_quote_without_master_optionchain_unavailable() {
    const PORT: u16 = 19_491;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

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
async fn planted_nfo_instruments_and_optionchain_success_lock_lot() {
    const PORT: u16 = 19_493;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_s1k_fixtures: true,
            plant_kotak_nfo_contracts: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

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
    assert_eq!(nfo["data"]["contract_count"], 1);
    assert_eq!(nfo["data"]["identity"]["family"], "reference");
    assert_eq!(
        nfo["data"]["identity"]["capability_id"],
        "derivative_contracts"
    );
    assert_eq!(nfo["data"]["identity"]["physics"], "bounded_snapshot");
    let lot = &nfo["data"]["rows"][0]["lot"];
    assert!(lot.is_number(), "lot must be a JSON number: {lot}");
    assert_eq!(lot.as_i64(), Some(65));
    assert_eq!(nfo["data"]["rows"][0]["instrument_id"], "nse_fo|56526");
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
    assert_eq!(chain["data"]["rows"][0]["instrument_id"], "nse_fo|56526");
    assert_eq!(chain["data"]["rows"][0]["lot"], 65);
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
/// slice spellings must not appear, and NFO depth stays unsupported.
#[tokio::test]
async fn planted_nfo_open_interest_lights_from_open_int_only() {
    const PORT: u16 = 19_494;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
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
    // Unbound `oi` slice spellings, and the master CSV cell, never reach the wire.
    for unbound in ["oi_las", "oi_high", "oi_low", "dOpenInterest"] {
        assert!(!wire.contains(unbound), "{unbound} must stay unbound");
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

    // NFO depth is a separate, still-unobserved body — it stays unsupported.
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
    assert_eq!(depth["status"], "unsupported");

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
