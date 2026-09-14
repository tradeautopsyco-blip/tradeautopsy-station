//! Binance options last (`binance-com-options`) on Station loopback (CI). No live eapi.
//! Lock: `locks/binance-com-options.md` — Way 3 last-only.
//!
//! Every `reqwest::Client::new()` here GETs `http://127.0.0.1:{port}/api/station/...`.
//! The spawned agent plants fixtures; `eapi_public_fetch` is off; dated-contract
//! quote records a stream key and does not dial eapi/WS. Live last is
//! `GET /eapi/v1/ticker?symbol=` when fetch is on. Ring 2 lints `src/` only;
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

    let history: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("history")
        .json()
        .await
        .expect("json");
    assert_eq!(history["status"], "unavailable");
    assert_ne!(history["status"], "unsupported");
    assert!(history["data"].is_null());
    assert_ne!(history["data"]["candles"][0]["open"], "0");

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
        serde_json::json!([
            "quotes",
            "optionchain",
            "open_interest",
            "optiongreeks",
            "depth",
            "history",
            "tradebook",
            "search"
        ])
    );
    // OI is its own noun on its own binding, not a second chain arm.
    let bound_ops: Vec<&str> = options["bindings"]
        .as_array()
        .expect("options bindings")
        .iter()
        .filter_map(|b| b["operation"].as_str())
        .collect();
    assert_eq!(
        bound_ops,
        vec![
            "quotes",
            "optionchain",
            "open_interest",
            "optiongreeks",
            "search",
            "tradebook",
            "history",
            "depth"
        ]
    );
    // Depth on this book is a REST bounded snapshot. A `stream` transport here
    // would claim spot's `@depth` reconstruction loop on a host that never serves it.
    let depth_bind = options["bindings"]
        .as_array()
        .expect("options bindings")
        .iter()
        .find(|b| b["operation"] == "depth")
        .expect("options depth binding");
    assert_eq!(depth_bind["capability_id"], "order_book");
    assert_eq!(depth_bind["physics"], "bounded_snapshot");
    assert_ne!(depth_bind["physics"], "ordered_state");
    assert_eq!(depth_bind["auth_mode"], "public");
    assert_eq!(depth_bind["transports"], serde_json::json!(["rest"]));
    // S3 product: depth/chain/OI may desk_display. Quotes stay research-only.
    assert_eq!(depth_bind["rights"]["display"], true);
    let quotes_bind = options["bindings"]
        .as_array()
        .expect("options bindings")
        .iter()
        .find(|b| b["operation"] == "quotes")
        .expect("options quotes binding");
    assert_eq!(quotes_bind["rights"]["display"], false);

    // Depth is claimed on both the options book and the named Kotak NFO book.
    let nfo = list
        .iter()
        .find(|m| m["book_id"] == "kotak-nse-nfo")
        .expect("nfo book");
    let nfo_ops: Vec<&str> = nfo["implemented"]
        .as_array()
        .expect("nfo implemented")
        .iter()
        .filter_map(|op| op.as_str())
        .collect();
    assert!(nfo_ops.contains(&"depth"));
    let nfo_depth = nfo["bindings"]
        .as_array()
        .expect("nfo bindings")
        .iter()
        .find(|b| b["operation"] == "depth")
        .expect("nfo depth binding");
    assert_eq!(nfo_depth["capability_id"], "order_book");
    assert_eq!(nfo_depth["physics"], "bounded_snapshot");
    assert_eq!(nfo_depth["auth_mode"], "private_read");
    assert_eq!(nfo_depth["transports"], serde_json::json!(["rest"]));
    assert_eq!(nfo_depth["rights"]["display"], true);

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

    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind options contract")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["bind_status"], "bound");
    assert_eq!(bind["instrument_id"], "BTC-200730-9000-C");

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
    assert_eq!(bind["book_id"], "binance-com-options");
    assert_eq!(bind["bind_status"], "bound");

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
    assert_eq!(body["book_id"], "binance-com-options");
    assert_eq!(body["bind_status"], "bound");
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
    assert_eq!(nfo["bind_status"], "refused");
    assert_eq!(nfo["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo["instrument_id"], "");

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
    assert_eq!(body["book_id"], "binance-com-options");
    assert_eq!(body["bind_status"], "bound");
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

/// Venue-published greeks with no `/eapi/v1/mark` snapshot in hand. The capability
/// is implemented and allowlisted, so this is `unavailable` — never `unsupported`,
/// and never an empty success with a fabricated delta.
#[tokio::test]
async fn greeks_route_without_a_mark_snapshot_is_unavailable_not_unsupported() {
    const PORT: u16 = 19_531;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/greeks?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("greeks route")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unavailable");
    assert_ne!(body["status"], "unsupported");
    assert_ne!(body["status"], "success");
    assert!(body["data"].is_null());
    let ineligible: Vec<&str> = body["ineligible"]
        .as_array()
        .expect("ineligible")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(ineligible.contains(&"mark_snapshot_unavailable"));
    assert!(!body.to_string().contains("delta"));

    handle.abort();
}

/// Planted `/eapi/v1/mark` lights the route. Station copied the venue's strings —
/// it did not price this book, so the origin is `venue_published` and the number
/// carries the one display grant Station ships.
#[tokio::test]
async fn planted_mark_lights_venue_published_greeks_and_nothing_else() {
    const PORT: u16 = 19_532;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_mark: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let body: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/greeks?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("greeks route")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["delta"], "0.55937056");
    assert_eq!(body["data"]["gamma"], "0.00010969");
    assert_eq!(body["data"]["theta"], "3739.82509871");
    assert_eq!(body["data"]["vega"], "978.58874732");
    assert_eq!(body["rights"]["display"], true);
    assert_eq!(body["rights"]["redistribute"], false);
    assert_eq!(body["source"]["path"], "/eapi/v1/mark");
    assert_eq!(body["source"]["kind"], "venue_published");
    assert_eq!(body["provenance"]["model"], "venue_published");
    assert_ne!(body["provenance"]["model"], "black_76");
    assert!(
        body["provenance"]["input_at"].as_str().is_some(),
        "an unstamped number has no freshness and must not render"
    );

    // Four greeks, not five. One currency, not two.
    let wire = body.to_string();
    assert!(!wire.contains("rho"));
    assert!(!wire.contains("INR"));
    assert!(!wire.contains("exchange_rate"));

    // A leftover spot id is not a dated contract: still a hole, never a repaint of
    // the stored contract's delta.
    for instrument in ["BTC", "BTCUSDT"] {
        let spot_id: serde_json::Value = client
            .get(format!(
                "http://127.0.0.1:{PORT}/api/station/greeks?book=binance-com-options&instrument={instrument}"
            ))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .expect("greeks route")
            .json()
            .await
            .expect("json");
        assert_eq!(spot_id["status"], "unavailable", "{instrument}");
        assert!(spot_id["data"].is_null(), "{instrument}");
        assert!(
            !spot_id.to_string().contains("0.55937056"),
            "{instrument} must not pick up the stored contract's delta"
        );
    }

    // Wrong book, or no book at all, is a fence — never the options row.
    for (url_book, reason) in [
        ("book=binance-com-spot&", "spot_is_not_greeks"),
        ("book=kotak-nse-bse-cash&", "cash_is_not_greeks"),
        ("", "greeks_book_required"),
    ] {
        let fenced: serde_json::Value = client
            .get(format!(
                "http://127.0.0.1:{PORT}/api/station/greeks?{url_book}instrument=BTC-200730-9000-C"
            ))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .expect("greeks route")
            .json()
            .await
            .expect("json");
        assert_eq!(fenced["status"], "unavailable", "{reason}");
        assert!(fenced["data"].is_null(), "{reason}");
        assert_eq!(fenced["rights"]["display"], false, "{reason}");
        let ineligible: Vec<&str> = fenced["ineligible"]
            .as_array()
            .expect("ineligible")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(ineligible.contains(&reason), "{reason}: {ineligible:?}");
        assert!(!fenced.to_string().contains("0.55937056"), "{reason}");
    }

    handle.abort();
}

/// The NFO book has no venue-published greeks and no named pricing model. A planted
/// USDT-options mark row must not leak onto it.
#[tokio::test]
async fn nfo_greeks_stay_dark_even_with_a_planted_mark_row() {
    const PORT: u16 = 19_533;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_mark: true,
            plant_kotak_nfo_contracts: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/greeks?book=kotak-nse-nfo&instrument=BANKNIFTY"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo greeks")
        .json()
        .await
        .expect("json");
    assert_ne!(body["status"], "success");
    assert!(body["data"].is_null());
    assert_eq!(body["rights"]["display"], false);
    let wire = body.to_string();
    assert!(!wire.contains("delta"), "no delta anywhere: {wire}");
    assert!(!wire.contains("0.55937056"));
    assert!(!wire.contains("exchange_rate"));

    handle.abort();
}

/// Obtain is the second door onto the same number. It is instrument-scoped off the
/// selected contract, so it stays Unavailable until that contract is bound.
#[tokio::test]
async fn obtain_optiongreeks_is_unavailable_until_planted_and_selected() {
    const PORT: u16 = 19_534;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let dark: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=optiongreeks"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain optiongreeks")
        .json()
        .await
        .expect("json");
    assert_eq!(dark["status"], "unavailable");
    assert_ne!(dark["status"], "unsupported");
    assert_ne!(dark["status"], "success");
    assert!(dark["data"].is_null());

    handle.abort();
}

#[tokio::test]
async fn obtain_optiongreeks_succeeds_on_the_selected_contract() {
    const PORT: u16 = 19_535;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            plant_binance_options_mark: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    // Bind the dated contract the way the desk does.
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
    assert_eq!(bind["book_id"], "binance-com-options");
    assert_eq!(bind["bind_status"], "bound");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=optiongreeks"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain optiongreeks")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["data"]["delta"], "0.55937056");
    assert_eq!(obtain["provenance_adapter_id"], "binance_com");
    assert_eq!(obtain["provenance_path"], "/eapi/v1/mark");
    let wire = obtain.to_string();
    assert!(!wire.contains("rho"));
    assert!(!wire.contains("INR"));
    assert!(!wire.contains("exchange_rate"));

    handle.abort();
}

/// Observed live 2026-08-31: `"bidIV":"-1.0"` alongside a real `askIV`. A negative
/// implied volatility is not a volatility — it is a no-bid sentinel. It must vanish
/// from the wire entirely, without taking the four real greeks with it.
#[tokio::test]
async fn a_no_bid_sentinel_vanishes_without_voiding_the_greeks() {
    const PORT: u16 = 19_536;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_mark_no_bid: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/greeks?book=binance-com-options&instrument=BTC-260925-145000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("greeks route")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "success");
    // Absent, not null: a refused IV must not serialize as a key at all.
    assert!(
        body["data"].get("bid_iv").is_none(),
        "-1.0 must vanish, not serialize as null"
    );
    assert_eq!(body["data"]["ask_iv"], "0.74930251");
    assert_eq!(body["data"]["mark_iv"], "0.709");
    // One dead IV does not void the greeks.
    assert_eq!(body["data"]["delta"], "0.00065535");
    assert_eq!(body["data"]["gamma"], "0.00000012");
    assert_eq!(body["data"]["theta"], "-0.84070424");
    assert_eq!(body["data"]["vega"], "0.46744754");

    let wire = body.to_string();
    assert!(
        !wire.contains("-1.0"),
        "the sentinel must not reach the wire"
    );
    assert!(!wire.contains("rho"));
    assert!(!wire.contains("INR"));
    assert!(!wire.contains("exchange_rate"));

    handle.abort();
}

/// S5 keep: lighting mark greeks must not rewrite last freshness. REST ticker has
/// no exchange timestamp (`age_unknown`) → chip `unknown`, never `fresh`.
#[tokio::test]
async fn lighting_greeks_does_not_flip_last_unknown_to_fresh() {
    const PORT: u16 = 19_547;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            plant_binance_options_mark: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let quote: serde_json::Value = client
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
    assert_eq!(quote["status"], "unknown");
    assert_ne!(quote["status"], "fresh");
    assert_eq!(quote["bind_status"], "bound");

    let greeks: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/greeks?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("greeks glance")
        .json()
        .await
        .expect("json");
    assert_eq!(greeks["status"], "success");
    assert_eq!(greeks["data"]["delta"], "0.55937056");
    assert_eq!(greeks["source"]["kind"], "venue_published");

    let last_after: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("quote after greeks")
        .json()
        .await
        .expect("json");
    assert_eq!(last_after["status"], "unknown");
    assert_ne!(last_after["status"], "fresh");
    assert_eq!(last_after["data"]["last"], "1.23");

    handle.abort();
}

/// Depth on this book is a REST bounded snapshot, and obtain is scoped to the
/// selected contract. With no planted ladder it is Unavailable — never
/// `unsupported`, and never an empty `{bids:[],asks:[]}` success.
#[tokio::test]
async fn options_depth_obtain_is_unavailable_without_a_planted_ladder() {
    const PORT: u16 = 19_537;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let dark: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain depth")
        .json()
        .await
        .expect("json");
    assert_eq!(dark["status"], "unavailable");
    assert_ne!(dark["status"], "unsupported");
    assert_ne!(dark["status"], "success");
    assert!(dark["data"].is_null());
    assert_eq!(dark["book_id"], "binance-com-options");

    // NFO depth is claimed. Without a snapshot it stays unavailable, not unsupported.
    let nfo: serde_json::Value = client
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
    assert_eq!(nfo["status"], "unavailable");
    assert_ne!(nfo["status"], "unsupported");
    assert!(nfo["data"].is_null());

    handle.abort();
}

/// The wire this PR exists for: a planted eapi body serves the options book's own
/// ladder, mixed-case, `bounded_snapshot`, no HMAC, no order count, no `synced`.
#[tokio::test]
async fn planted_options_depth_serves_a_bounded_snapshot_on_the_options_book() {
    const PORT: u16 = 19_538;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_depth: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    // Bind the dated contract the way the desk does.
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
    assert_eq!(bind["book_id"], "binance-com-options");
    assert_eq!(bind["bind_status"], "bound");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain depth")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["book_id"], "binance-com-options");
    assert_eq!(obtain["provenance_adapter_id"], "binance_com");
    let data = &obtain["data"];
    assert_eq!(data["identity"]["family"], "market");
    assert_eq!(data["identity"]["capability_id"], "order_book");
    assert_eq!(data["identity"]["physics"], "bounded_snapshot");
    assert_ne!(data["identity"]["physics"], "ordered_state");
    assert_eq!(data["source"], "rest_snapshot");
    assert_ne!(data["source"], "synced");
    assert!(data.get("synced").is_none());
    // Mixed-case dated contract, never smashed into a spot pair.
    assert_eq!(data["instrument_id"], "BTC-200730-9000-C");
    assert_eq!(data["bids"][0]["price"], "1000.000");
    assert_eq!(data["bids"][0]["quantity"], "0.1000");
    assert_eq!(data["asks"][0]["price"], "1900.000");
    assert_eq!(data["bound_levels"], 1);
    assert_eq!(data["completeness"], true);
    // Eapi levels are `[price, quantity]` — there is no order count to publish.
    assert!(data["bids"][0].get("orders").is_none());
    assert!(data["asks"][0].get("orders").is_none());
    let wire = obtain.to_string();
    assert!(!wire.contains("btcusdt"));
    assert!(!wire.contains("binance-com-spot"));
    assert!(!wire.contains("INR"));

    let glance: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/depth?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("options depth glance")
        .json()
        .await
        .expect("json");
    assert_eq!(glance["status"], "success");
    assert_eq!(glance["identity"]["physics"], "bounded_snapshot");
    assert_eq!(glance["data"]["bids"][0]["price"], "1000.000");
    assert!(glance["data"]["bids"][0].get("orders").is_none());
    assert_eq!(glance["rights"]["display"], true);

    // Same slug, spot book: the options ladder must not be reachable there.
    let spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain spot depth")
        .json()
        .await
        .expect("json");
    assert_ne!(spot["status"], "success");
    assert!(spot["data"].is_null());
    assert!(!spot.to_string().contains("BTC-200730-9000-C"));

    // Slug-only obtain stays on the Start default book, which is spot — depth on
    // `binance_com` with no `book=` is never the options ladder.
    let slug_only: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain slug-only depth")
        .json()
        .await
        .expect("json");
    assert_eq!(slug_only["book_id"], "binance-com-spot");
    assert_ne!(slug_only["book_id"], "binance-com-options");
    assert_ne!(slug_only["status"], "success");
    assert!(!slug_only.to_string().contains("BTC-200730-9000-C"));

    handle.abort();
}

/// A leftover spot id is not a contract. Selecting `BTCUSDT` must leave options
/// depth dark rather than resolving the planted contract's ladder for it.
#[tokio::test]
async fn a_leftover_spot_id_leaves_options_depth_unavailable() {
    const PORT: u16 = 19_539;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_depth: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    // Select a spot pair, the way a leftover desk symbol would.
    let _bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind spot pair")
        .json()
        .await
        .expect("json");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=depth"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain depth")
        .json()
        .await
        .expect("json");
    // The planted contract is still in the book, but `BTCUSDT` is not a dated
    // contract, so options depth is instrument-scoped to nothing and stays dark.
    // Regression for a dogfooding find: this used to Success with whichever
    // ladder happened to be resident, which is not the contract anyone selected.
    assert_eq!(obtain["status"], "unavailable");
    assert_ne!(obtain["status"], "success");
    assert!(obtain["data"].is_null());
    assert!(!obtain.to_string().contains("BTC-200730-9000-C"));

    handle.abort();
}

/// Tradebook is implemented on this book. With no planted fills it stays
/// `unavailable` — never `unsupported`, and never an empty success.
#[tokio::test]
async fn options_obtain_tradebook_is_unavailable_without_planted_fills() {
    const PORT: u16 = 19_540;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let obtain: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=tradebook"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain tradebook")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "unavailable");
    assert_ne!(obtain["status"], "unsupported");
    assert_ne!(obtain["status"], "success");
    assert!(obtain["data"].is_null());
    assert_eq!(obtain["book_id"], "binance-com-options");

    handle.abort();
}

/// Planted `/eapi/v1/userTrades` rows serve the options book tradebook only.
#[tokio::test]
async fn planted_options_fills_serve_tradebook_on_the_options_book() {
    const PORT: u16 = 19_541;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_fills: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=tradebook"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain tradebook")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["book_id"], "binance-com-options");
    assert_eq!(obtain["provenance_path"], "/eapi/v1/userTrades");
    assert_eq!(obtain["data"]["fill_count"], 1);
    assert_eq!(obtain["data"]["rows"][0]["symbol"], "BTC-200730-9000-C");
    assert_eq!(obtain["data"]["rows"][0]["side"], "BUY");
    assert_eq!(obtain["data"]["rows"][0]["price"], 1000.0);
    assert_eq!(obtain["data"]["rows"][0]["qty"], 0.1);
    let wire = obtain.to_string();
    assert!(!wire.contains("btcusdt"));
    assert!(!wire.contains("binance-com-spot"));

    // Spot tradebook must not pick up the options fills slot.
    let spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=tradebook"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain spot tradebook")
        .json()
        .await
        .expect("json");
    assert_ne!(spot["status"], "success");
    assert!(spot["data"].is_null());
    assert!(!spot.to_string().contains("BTC-200730-9000-C"));

    handle.abort();
}

/// Planted eapi klines serve `market/ohlcv` on the options book. Spot
/// `/api/v3/klines` must not light this contract.
#[tokio::test]
async fn planted_options_klines_serve_history_on_the_options_book() {
    const PORT: u16 = 19_542;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_history: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

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
    assert_eq!(bind["book_id"], "binance-com-options");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain history")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["book_id"], "binance-com-options");
    assert_eq!(obtain["provenance_path"], "/eapi/v1/klines");
    assert_eq!(obtain["data"]["source"], "eapi_klines");
    assert_ne!(obtain["data"]["source"], "binance_klines");
    assert_eq!(obtain["data"]["instrument_id"], "BTC-200730-9000-C");
    assert_eq!(obtain["data"]["interval"], "1m");
    assert_eq!(obtain["data"]["candles"][0]["open"], "950");
    assert_eq!(obtain["data"]["candles"][0]["close"], "1000");
    let wire = obtain.to_string();
    assert!(!wire.contains("btcusdt"));
    assert!(!wire.contains("/api/v3/klines"));

    let station: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/history?instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("station history")
        .json()
        .await
        .expect("json");
    assert_eq!(station["status"], "success");
    assert_eq!(station["instrument_id"], "BTC-200730-9000-C");
    assert_eq!(station["data"]["source"], "eapi_klines");

    let spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("spot history")
        .json()
        .await
        .expect("json");
    assert_ne!(spot["status"], "success");
    assert!(spot["data"].is_null());

    handle.abort();
}

/// Spot klines plant must not fill the dated-contract options session.
#[tokio::test]
async fn spot_klines_plant_does_not_fill_options_session() {
    const PORT: u16 = 19_543;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_s2_history: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let _bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTC-200730-9000-C&book=binance-com-options"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind")
        .json()
        .await
        .expect("json");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=history"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain history")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "unavailable");
    assert!(obtain["data"].is_null());

    handle.abort();
}

/// No planted index → hole. Never S=0, never spot last / markPrice as S.
#[tokio::test]
async fn index_route_without_a_snapshot_is_unavailable_not_zero() {
    const PORT: u16 = 19_544;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/index?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("index route")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unavailable");
    assert_ne!(body["status"], "success");
    assert!(body["data"].is_null());
    let wire = body.to_string();
    assert!(!wire.contains("indexPrice"));
    assert!(!wire.contains("lastPrice"));
    assert!(!wire.contains("markPrice"));

    handle.abort();
}

/// Planted index + catalog lights lock-named `indexPrice`. Query underlying is
/// `optionSymbols.underlying` (BTCUSDT), not OI's BTC. Spot book stays dark.
#[tokio::test]
async fn planted_index_lights_lock_named_s_from_catalog_underlying() {
    const PORT: u16 = 19_545;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_chain: true,
            plant_binance_options_index: true,
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;
    let client = reqwest::Client::new();

    let body: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/index?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("index route")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "success");
    assert_eq!(body["data"]["indexPrice"], "27670.21666667");
    assert_eq!(body["data"]["underlying"], "BTCUSDT");
    assert_ne!(body["data"]["underlying"], "BTC");
    assert_eq!(body["provenance"]["path"], "/eapi/v1/index");
    assert_eq!(body["identity"]["physics"], "latest_state");
    assert_eq!(body["identity"]["capability_id"], "index");
    assert_ne!(body["provenance"]["model"], "model_computed");
    let wire = body.to_string();
    assert!(!wire.contains("lastPrice"));
    assert!(!wire.contains("markPrice"));
    assert!(!wire.contains("underlyingAsset"));

    for instrument in ["BTC", "BTCUSDT", "XRP"] {
        let miss: serde_json::Value = client
            .get(format!(
                "http://127.0.0.1:{PORT}/api/station/index?book=binance-com-options&instrument={instrument}"
            ))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .expect("index route")
            .json()
            .await
            .expect("json");
        assert_eq!(miss["status"], "unavailable", "{instrument}");
        assert!(miss["data"].is_null(), "{instrument}");
        assert!(
            !miss.to_string().contains("27670.21666667"),
            "{instrument} must not pick up the planted S"
        );
    }

    let spot: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/index?book=binance-com-spot&instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("spot index")
        .json()
        .await
        .expect("json");
    assert_eq!(spot["status"], "unavailable");
    assert!(spot["data"].is_null());

    let nfo: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/index?book=kotak-nse-nfo&instrument=NIFTY"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("nfo index")
        .json()
        .await
        .expect("json");
    assert_eq!(nfo["status"], "unavailable");
    assert!(nfo["data"].is_null());

    handle.abort();
}

/// Index without a catalog row stays dark — do not invent BTCUSDT from typing BTC.
#[tokio::test]
async fn planted_index_without_catalog_stays_unavailable() {
    const PORT: u16 = 19_546;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_index: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/index?book=binance-com-options&instrument=BTC-200730-9000-C"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("index route")
        .json()
        .await
        .expect("json");
    assert_eq!(body["status"], "unavailable");
    assert!(body["data"].is_null());
    assert!(!body.to_string().contains("27670.21666667"));

    handle.abort();
}
