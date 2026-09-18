//! Slice F — CI fixture gate for Kotak s1k A–F (quotes, instruments, REST depth).
//!
//! Locks honest exits so quotes / instruments / host policy cannot silently
//! regress. Live "Start Kotak → RELIANCE LTP moves" still needs a session and
//! is leftover, not a fake pass.

mod common;

use common::{
    apply_wire_v1, client, spawn_test_agent, spawn_test_agent_with_options, wait_ready,
    TestAgentOptions, WireHeaderOverrides,
};
use tradeautopsy_agent::{
    authorize_book_call, authorize_host_call, authorize_inferred_call, infer_capability,
    kotak_neo_s1k_manifest, obtain, AuthMode, HostRefuse, ObtainStatus, R0_ALLOWED_HOSTS,
};

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn obtain_json(port: u16, adapter: &str, operation: &str) -> serde_json::Value {
    let url = format!(
        "http://127.0.0.1:{port}/api/station/obtain?adapter={adapter}&operation={operation}"
    );
    client()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("obtain")
        .json()
        .await
        .expect("json")
}

#[tokio::test]
async fn empty_agent_kotak_obtain_is_unavailable_history_unsupported() {
    const PORT: u16 = 39_540;
    let _handle = AbortOnDrop(spawn_test_agent(PORT));
    wait_ready(PORT).await;

    let quotes = obtain_json(PORT, "kotak_neo", "quotes").await;
    assert_eq!(quotes["status"], "unavailable");
    assert!(quotes["data"].is_null());

    let history = obtain_json(PORT, "kotak_neo", "history").await;
    assert_eq!(history["status"], "unsupported");
    assert!(history["data"].is_null());

    let depth = obtain_json(PORT, "kotak_neo", "depth").await;
    assert_eq!(depth["status"], "unavailable");
    assert!(depth["data"].is_null());

    let optionchain = obtain_json(PORT, "kotak_neo", "optionchain").await;
    assert_eq!(optionchain["status"], "unsupported");
    assert!(optionchain["data"].is_null());
}

#[tokio::test]
async fn planted_fixtures_lock_search_quote_and_obtain_success() {
    const PORT: u16 = 39_541;
    let _handle = AbortOnDrop(spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_s1k_fixtures: true,
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT).await;
    let http = client();

    let search_path = "/instruments/search";
    let search: serde_json::Value = apply_wire_v1(
        http.get(format!("http://127.0.0.1:{PORT}{search_path}?q=RELIANCE")),
        "GET",
        search_path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(std::time::Duration::from_secs(2))
    .send()
    .await
    .expect("search")
    .json()
    .await
    .expect("search json");
    let hits = search["symbols"].as_array().expect("symbols");
    assert!(!hits.is_empty(), "fixture cash CSV must hit RELIANCE");
    assert!(hits.iter().all(|h| h["trading_symbol"] == "RELIANCE"));
    assert!(hits.iter().any(|h| {
        h["exchange"] == "kotak_neo"
            && (h["segment"] == "nse_cm" || h["segment"] == "bse_cm")
            && h["instrument_token"] == 2885
    }));
    assert!(hits.iter().all(|h| {
        let exchange = h["exchange"].as_str().unwrap_or("");
        let segment = h["segment"].as_str().unwrap_or("");
        (exchange == "kotak_neo" || segment == "nse_cm" || segment == "bse_cm")
            && exchange != "BINANCE"
            && exchange != "NSE"
            && exchange != "binance_com"
            && segment != "nse_fo"
            && segment != "bse_fo"
    }));

    let quote: serde_json::Value = http
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=nse_cm%7C2885"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("quote")
        .json()
        .await
        .expect("quote json");
    assert_ne!(quote["status"], "unavailable");
    let last: f64 = quote["data"]["last"]
        .as_str()
        .expect("last")
        .parse()
        .expect("last f64");
    assert!(last > 0.0);
    assert_eq!(quote["provenance"]["adapter_id"], "kotak_neo");
    assert_ne!(quote["provenance"]["adapter_id"], "binance_com");
    assert!(quote["data"]["session_ohlc"].is_object());
    assert_eq!(quote["instrument_id"], "nse_cm|2885");

    let empty_quote: serde_json::Value = http
        .get(format!("http://127.0.0.1:{PORT}/api/station/quote"))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("empty quote")
        .json()
        .await
        .expect("empty quote json");
    assert_eq!(empty_quote["status"], "unavailable");
    assert_eq!(empty_quote["instrument_id"], "");

    let quotes = obtain_json(PORT, "kotak_neo", "quotes").await;
    assert_eq!(quotes["status"], "success");
    assert!(!quotes["data"].is_null());
    let obtain_last: f64 = quotes["data"]["last"]
        .as_str()
        .expect("obtain last")
        .parse()
        .expect("obtain last f64");
    assert!(obtain_last > 0.0);
    assert_eq!(quotes["provenance_adapter_id"], "kotak_neo");
    assert_eq!(quotes["data"]["source"], "tickbook");
    assert!(quotes["data"]["session_ohlc"].is_object());

    let instruments = obtain_json(PORT, "kotak_neo", "instruments").await;
    assert_eq!(instruments["status"], "success");
    assert!(instruments["data"]["symbol_count"].as_u64().unwrap() >= 1);
    assert_eq!(instruments["data"]["identity"]["family"], "reference");

    let history = obtain_json(PORT, "kotak_neo", "history").await;
    assert_eq!(history["status"], "unsupported");
    assert!(history["data"].is_null());

    let depth = obtain_json(PORT, "kotak_neo", "depth").await;
    assert_eq!(depth["status"], "success");
    assert!(!depth["data"].is_null());
    assert_eq!(depth["data"]["identity"]["family"], "market");
    assert_eq!(depth["data"]["identity"]["capability_id"], "order_book");
    assert_eq!(depth["data"]["identity"]["physics"], "bounded_snapshot");
    assert_ne!(depth["data"]["identity"]["physics"], "ordered_state");
    assert_eq!(depth["data"]["source"], "rest_snapshot");
    assert!(depth["data"].get("synced").is_none());
    assert_ne!(depth["status"], "synced");
    assert!(depth["data"]["bids"].as_array().unwrap().len() >= 1);
    assert!(depth["data"]["asks"].as_array().unwrap().len() >= 1);
    assert_eq!(depth["data"]["completeness"], true);
    assert_eq!(depth["provenance_adapter_id"], "kotak_neo");

    let optionchain = obtain_json(PORT, "kotak_neo", "optionchain").await;
    assert_eq!(optionchain["status"], "unsupported");
    assert!(optionchain["data"].is_null());
}

#[test]
fn obtain_kotak_history_is_unavailable_until_fetch() {
    let envelope = obtain(&kotak_neo_s1k_manifest(), "history");
    assert_eq!(envelope.status, ObtainStatus::Unavailable);
    assert!(envelope.data.is_none());
}

#[test]
fn obtain_kotak_quotes_and_instruments_are_unavailable_until_planted() {
    let manifest = kotak_neo_s1k_manifest();
    let quotes = obtain(&manifest, "quotes");
    assert_eq!(quotes.status, ObtainStatus::Unavailable);
    assert!(quotes.data.is_none());
    let instruments = obtain(&manifest, "instruments");
    assert_eq!(instruments.status, ObtainStatus::Unavailable);
    assert!(instruments.data.is_none());
    let depth = obtain(&manifest, "depth");
    assert_eq!(depth.status, ObtainStatus::Unavailable);
    assert!(depth.data.is_none());
    assert_eq!(
        obtain(&manifest, "optionchain").status,
        ObtainStatus::Unsupported
    );
}

#[test]
fn mutations_quick_quotes_napi_csv_and_klines_stay_refused() {
    for path in [
        "/quick/order/place",
        "/quick/order/modify",
        "/quick/order/cancel",
        "/quick/order/placeOrder",
        "/quick/order/modifyOrder",
        "/quick/order/cancelOrder",
    ] {
        assert_eq!(
            infer_capability("POST", path).unwrap_err(),
            HostRefuse::MutationForbidden,
            "{path}"
        );
    }
    assert_eq!(
        infer_capability("GET", "/quick/quotes").unwrap_err(),
        HostRefuse::PathNotAllowlisted
    );
    assert_eq!(
        infer_capability("GET", "apim/quotes/1.0/quotes/neosymbol/nse_cm%7C2885/ltp").unwrap_err(),
        HostRefuse::PathNotAllowlisted
    );
    assert_eq!(
        authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/quick/quotes",
            "quote",
            AuthMode::PrivateRead,
            true,
        )
        .unwrap_err(),
        HostRefuse::PathNotAllowlisted
    );
    let csv = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv";
    let (csv_cap, csv_mode) = infer_capability("GET", csv).unwrap();
    assert_eq!(csv_cap, "instrument_master");
    assert_eq!(csv_mode, AuthMode::Public);
    authorize_inferred_call("lapi.kotaksecurities.com", "GET", csv, false)
        .expect("cash scrip CSV GET is allowlisted");
    let live_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_cm-v1.csv";
    let (v1_cap, v1_mode) = infer_capability("GET", live_v1).unwrap();
    assert_eq!(v1_cap, "instrument_master");
    assert_eq!(v1_mode, AuthMode::Public);
    authorize_inferred_call("lapi.kotaksecurities.com", "GET", live_v1, false)
        .expect("live v1 cash scrip CSV GET is allowlisted");
    let fo = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv";
    assert_eq!(
        authorize_book_call(
            "kotak-nse-bse-cash",
            "lapi.kotaksecurities.com",
            "GET",
            fo,
            false
        )
        .unwrap_err(),
        HostRefuse::PathNotAllowlisted
    );
    let fo_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_fo-v1.csv";
    assert_eq!(
        authorize_book_call(
            "kotak-nse-bse-cash",
            "lapi.kotaksecurities.com",
            "GET",
            fo_v1,
            false
        )
        .unwrap_err(),
        HostRefuse::PathNotAllowlisted
    );
    assert!(R0_ALLOWED_HOSTS.contains(&"lapi.kotaksecurities.com"));
    assert!(!R0_ALLOWED_HOSTS.contains(&"mlhsm.kotaksecurities.com"));
    let (klines_cap, klines_mode) = infer_capability("GET", "/api/v3/klines").unwrap();
    assert_eq!(klines_cap, "ohlcv");
    assert_eq!(klines_mode, AuthMode::Public);
    assert_eq!(
        authorize_inferred_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/api/v3/klines",
            false
        )
        .unwrap_err(),
        HostRefuse::HostNotAllowed
    );
    assert_eq!(
        authorize_inferred_call("api.binance.com", "GET", "/api/v3/klines", true).unwrap_err(),
        HostRefuse::PrivateCredentialOnPublicCall
    );
}

#[test]
fn quote_type_depth_infers_order_book_ltp_stays_quote() {
    let depth = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/depth";
    let (cap, mode) = infer_capability("GET", depth).unwrap();
    assert_eq!(cap, "order_book");
    assert_eq!(mode, AuthMode::PrivateRead);
    let (ltp_cap, ltp_mode) = infer_capability(
        "GET",
        "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp",
    )
    .unwrap();
    assert_eq!(ltp_cap, "quote");
    assert_eq!(ltp_mode, AuthMode::PrivateRead);
}
