//! Binance options last (`binance-com-options`) on Station loopback (CI). No live eapi.
//! Lock: `locks/binance-com-options.md` — Way 3 last-only.
//!
//! Every `reqwest::Client::new()` here GETs `http://127.0.0.1:{port}/api/station/...`.
//! The spawned agent plants fixtures; `eapi_public_fetch` is off; dated-contract
//! quote records a stream key and does not dial eapi/WS. Ring 2 lints `src/` only;
//! these tests stay off EXEMPT.

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

    // optionchain is implemented on this book now. With no planted master it is
    // `unavailable` — never `unsupported`, and never an empty success.
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
    assert_eq!(chain["status"], "unavailable");
    assert_ne!(chain["status"], "unsupported");
    assert_ne!(chain["status"], "success");
    assert!(chain["data"].is_null());

    let oi: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=open_interest"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("open_interest")
        .json()
        .await
        .expect("json");
    assert_eq!(oi["status"], "unavailable");
    assert_ne!(oi["status"], "unsupported");
    assert!(oi["data"].is_null());

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
    assert_eq!(
        options["implemented"],
        serde_json::json!(["quotes", "optionchain", "open_interest"])
    );
    // OI is its own noun on its own binding, not a second chain arm.
    let bound_ops: Vec<&str> = options["bindings"]
        .as_array()
        .expect("options bindings")
        .iter()
        .filter_map(|b| b["operation"].as_str())
        .collect();
    assert_eq!(bound_ops, vec!["quotes", "optionchain", "open_interest"]);

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
    // A last-only book with no chain master stays dark. Never an empty success.
    assert_eq!(obtain_chain["status"], "unavailable");
    assert_ne!(obtain_chain["status"], "success");
    assert!(obtain_chain["data"].is_null());

    handle.abort();
}

#[tokio::test]
async fn planted_options_master_lights_chain_and_oi_on_glance_and_obtain() {
    const PORT: u16 = 19_521;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            plant_binance_options_chain: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let chain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options chain")
        .json()
        .await
        .expect("json");
    assert_eq!(chain["status"], "success");
    assert_eq!(chain["data"]["row_count"], 3);
    assert_eq!(
        chain["data"]["rows"][0]["instrument_id"],
        "BTC-200730-9000-C"
    );
    assert_eq!(chain["data"]["rows"][0]["last"], "1.23");
    assert!(
        chain["data"]["rows"][0].get("lot").is_none() || chain["data"]["rows"][0]["lot"].is_null()
    );

    let eth: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/chain?book=binance-com-options&instrument=ETH-200730-400-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("eth chain")
        .json()
        .await
        .expect("json");
    assert_eq!(eth["status"], "success");
    assert_eq!(eth["data"]["row_count"], 1);

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
    assert_eq!(oi["status"], "success");
    assert_eq!(oi["data"]["sumOpenInterest"], "12.5");

    // Obtain is instrument-scoped off the selected id. Bind the dated contract
    // the way the desk does — a leftover BTC / BTCUSDT must not smash a chain in.
    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind dated contract")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["instrument_id"], "BTC-200730-9000-C");

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
    assert_eq!(obtain_chain["status"], "success");
    assert_ne!(obtain_chain["status"], "unsupported");
    // Same rows through either door.
    assert_eq!(
        obtain_chain["data"]["row_count"],
        chain["data"]["row_count"]
    );
    assert_eq!(obtain_chain["data"]["row_count"], 3);
    assert_eq!(
        obtain_chain["data"]["rows"][0]["instrument_id"],
        "BTC-200730-9000-C"
    );
    assert_eq!(obtain_chain["provenance_adapter_id"], "binance_com");
    // Success must name the path Binance actually has.
    assert_eq!(obtain_chain["provenance_path"], "/eapi/v1/exchangeInfo");

    // Nothing on the obtain wire may name a path the official MarketDataApi
    // does not list. `/eapi/v1/optionChain` stays NOT SPECIFIED.
    let chain_wire = obtain_chain.to_string();
    assert!(!chain_wire.contains("optionChain"));
    assert!(!chain_wire.contains("/eapi/v1/optionChain"));

    let obtain_oi: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=open_interest"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain open_interest")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain_oi["status"], "success");
    assert_eq!(obtain_oi["data"]["sumOpenInterest"], "12.5");
    assert_ne!(obtain_oi["data"]["sumOpenInterest"], "0");
    assert_eq!(obtain_oi["provenance_path"], "/eapi/v1/openInterest");
    assert!(!obtain_oi.to_string().contains("optionChain"));

    handle.abort();
}

/// B1 standing fact 3: a leftover spot id on the options book yields no rows.
/// Obtain stays dark rather than smashing a spot chain in.
#[tokio::test]
async fn options_obtain_optionchain_refuses_a_spot_instrument() {
    const PORT: u16 = 19_523;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            plant_binance_options_chain: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    // Select a spot id. The options master has no `BTCUSDT` symbol.
    client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind spot id");

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
    assert_eq!(obtain_chain["status"], "unavailable");
    assert_ne!(obtain_chain["status"], "success");
    assert!(obtain_chain["data"].is_null());

    handle.abort();
}

/// options book. Mixed case survives; the spot slot is never consulted.
#[tokio::test]
async fn quote_route_named_options_book_serves_mixed_case_last() {
    const PORT: u16 = 19_512;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let body: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options quote")
        .json()
        .await
        .expect("json");
    // REST ticker carries no server time: `age_unknown` → `unknown`, never invented freshness.
    assert_eq!(body["status"], "unknown");
    assert_ne!(body["status"], "unavailable");
    assert_eq!(body["data"]["last"], "1.23");
    assert_ne!(body["data"]["last"], "0");
    assert_eq!(body["instrument_id"], "BTC-200730-9000-C");
    assert_ne!(body["instrument_id"], "btc-200730-9000-c");
    assert_eq!(body["provenance"]["adapter_id"], "binance_com");
    assert_eq!(body["provenance"]["transport"], "rest");
    assert_eq!(body["identity"]["capability_id"], "quote");
    assert!(body["ineligible"].as_array().unwrap().is_empty());

    // Options book requested with an NFO id must not serve NFO data.
    let nfo: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_fo%7C12345&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo on options book")
        .json()
        .await
        .expect("json");
    assert_eq!(nfo["status"], "unavailable");
    assert!(nfo["data"].is_null());

    handle.abort();
}

/// Chosen semantics: a dated contract routes to the options book by shape, with no
/// `book=` — the desk binds these with `bookId == nil`. Never spot, never NFO.
#[tokio::test]
async fn quote_route_without_book_routes_dated_contract_by_shape() {
    const PORT: u16 = 19_513;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bare options id")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unknown");
    assert_eq!(body["data"]["last"], "1.23");
    // Case is preserved — the spot resolver never rewrote the id.
    assert_eq!(body["instrument_id"], "BTC-200730-9000-C");
    assert_ne!(body["instrument_id"], "btc-200730-9000-c");
    assert_eq!(body["provenance"]["adapter_id"], "binance_com");

    // The lowercased spot form is a different id and stays a hole: the options
    // row was read from `binance-com-options`, not from the spot slot.
    let spot_form: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=btc-200730-9000-c"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("lowercased form")
        .json()
        .await
        .expect("json");
    assert_eq!(spot_form["status"], "unavailable");
    assert!(spot_form["data"].is_null());

    handle.abort();
}

/// A spot instrument can never pick up the options last, with or without `book=`.
#[tokio::test]
async fn quote_route_spot_instrument_never_serves_options_last() {
    const PORT: u16 = 19_514;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    for url in [
        format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSDT&book=binance-com-spot"
        ),
        format!("http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSDT"),
    ] {
        let body: serde_json::Value = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .expect("spot quote")
            .json()
            .await
            .expect("json");
        assert_eq!(body["status"], "unavailable", "{url}");
        assert!(body["data"].is_null(), "{url}");
        assert_ne!(body["data"]["last"], "1.23", "{url}");
        assert_eq!(body["instrument_id"], "btcusdt", "{url}");
        assert_ne!(body["instrument_id"], "BTC-200730-9000-C", "{url}");
    }

    handle.abort();
}

/// Unplanted options book is `unavailable` and opens no spot stream. The stream-set
/// seam itself is asserted in `api::quote::tests::dated_contract_records_options_key_and_no_spot_key`
/// (`quote_streams` is not reachable over HTTP); here we prove the route stays a hole
/// and never lights spot last / depth / history for the contract.
#[tokio::test]
async fn quote_route_options_book_unplanted_is_unavailable_and_opens_no_spot_stream() {
    const PORT: u16 = 19_515;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let body: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options quote")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unavailable");
    assert!(body["data"].is_null());
    assert_eq!(body["instrument_id"], "BTC-200730-9000-C");

    // The spot slot was not subscribed on the contract's account: a bare spot read
    // stays a hole, and the lowercased contract id never became a spot instrument.
    let spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=btc-200730-9000-c"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("spot read")
        .json()
        .await
        .expect("json");
    assert_eq!(spot["status"], "unavailable");
    assert!(spot["data"].is_null());

    handle.abort();
}
