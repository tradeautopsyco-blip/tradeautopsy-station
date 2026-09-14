//! Book-scoped quote selection on Station loopback (CI). No live venues.
//!
//! Assumes `QuoteSelections`, `validate_quote_binding`, `bind_quote_selection`, and
//! `selected_quote_for` on AppState. Each obtain path reads only its own book slot.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent,
    spawn_test_agent_with_options, wait_ready, TestAgentOptions, WireHeaderOverrides,
};
use serde_json::Value;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{BrokerAdapter, BrokerCredentialVault, CountingPollAdapter};

const FIXTURE_CONTRACT: &str = "BTC-200730-9000-C";
const MIXED_CASE_CONTRACT: &str = "BTC-260925-145000-C";
const NFO_INSTRUMENT: &str = "nse_fo|12345";

async fn start_kotak_sync(port: u16) {
    assert_eq!(
        post_broker_sync_start(port, identity_start_body("kotak_neo"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;
}

fn multibroker_opts(adapter: Arc<dyn BrokerAdapter>) -> TestAgentOptions {
    kotak_runtime_opts(
        adapter,
        TestAgentOptions {
            plant_kotak_nfo_quote: true,
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    )
}

async fn wait_for_quote_route(port: u16) {
    wait_ready(port).await;
}

fn quote_url(port: u16, instrument: &str, book: Option<&str>) -> String {
    match book {
        Some(book) => {
            format!("http://127.0.0.1:{port}/api/station/quote?instrument={instrument}&book={book}")
        }
        None => format!("http://127.0.0.1:{port}/api/station/quote?instrument={instrument}"),
    }
}

async fn get_quote(port: u16, instrument: &str, book: Option<&str>) -> Value {
    reqwest::Client::new()
        .get(quote_url(port, instrument, book))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("quote GET")
        .json()
        .await
        .expect("quote json")
}

async fn get_obtain(port: u16, book: &str, operation: &str) -> Value {
    reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{port}/api/station/obtain?book={book}&operation={operation}"
        ))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("obtain GET")
        .json()
        .await
        .expect("obtain json")
}

async fn post_broker_sync_start(port: u16, body: Value) -> reqwest::Response {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&body).expect("json");
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
    .expect("broker start")
}

async fn post_broker_sync_stop(port: u16) -> reqwest::Response {
    let path = "/api/daemon/broker/sync/stop";
    let url = format!("http://127.0.0.1:{port}{path}");
    apply_wire_v1(
        client().post(&url),
        "POST",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("broker stop")
}

fn assert_bound(body: &Value, book_id: &str, instrument_id: &str) {
    assert_eq!(body["bind_status"], "bound", "expected bound: {body}");
    assert_eq!(body["book_id"], book_id);
    assert_eq!(body["instrument_id"], instrument_id);
}

fn assert_refused(body: &Value, book_id: &str, ineligible: &str) {
    assert_eq!(body["bind_status"], "refused", "expected refused: {body}");
    assert_eq!(body["book_id"], book_id);
    assert_eq!(body["instrument_id"], "");
    assert_eq!(body["status"], "unavailable");
    assert!(body["data"].is_null());
    let empty: Vec<serde_json::Value> = vec![];
    let classes: Vec<&str> = body["ineligible"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        classes.contains(&ineligible),
        "expected ineligible {ineligible:?}, got {classes:?}"
    );
}

fn kotak_runtime_opts(adapter: Arc<dyn BrokerAdapter>, opts: TestAgentOptions) -> TestAgentOptions {
    let vault = seeded_hmac_vault("kotak_neo", "TA_TEST_SYNC");
    TestAgentOptions {
        runtime_poll_adapter: Some(adapter),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..opts
    }
}

/// Kotak is the execution desk; a dated contract still binds the public options book.
#[tokio::test]
async fn kotak_execution_active_dated_contract_resolves_to_options_book() {
    const PORT: u16 = 19_640;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_quote: true,
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let body = get_quote(PORT, FIXTURE_CONTRACT, None).await;
    assert_bound(&body, "binance-com-options", FIXTURE_CONTRACT);
    assert_ne!(body["status"], "unavailable");

    let obtain = get_obtain(PORT, "binance-com-options", "quotes").await;
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["book_id"], "binance-com-options");
    assert_eq!(obtain["data"]["instrument_id"], FIXTURE_CONTRACT);

    handle.abort();
}

/// Mixed-case dated symbols survive bind and extract verbatim.
#[tokio::test]
async fn dated_contract_preserves_verbatim_case() {
    const PORT: u16 = 19_641;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let body = get_quote(PORT, MIXED_CASE_CONTRACT, Some("binance-com-options")).await;
    assert_bound(&body, "binance-com-options", MIXED_CASE_CONTRACT);
    assert_ne!(body["instrument_id"], "btc-260925-145000-c");

    let lower = get_quote(PORT, "btc-260925-145000-c", None).await;
    assert_eq!(lower["status"], "unavailable");
    assert!(lower["data"].is_null());

    handle.abort();
}

/// Selecting Binance options must not clobber an independent Kotak NFO selection.
#[tokio::test]
#[serial]
async fn options_selection_does_not_overwrite_nfo_selection() {
    const PORT: u16 = 19_642;
    let counter = Arc::new(CountingPollAdapter::new());
    let handle = spawn_test_agent_with_options(PORT, multibroker_opts(counter));
    wait_for_quote_route(PORT).await;
    start_kotak_sync(PORT).await;

    let nfo_bind = get_quote(PORT, NFO_INSTRUMENT, None).await;
    assert_bound(&nfo_bind, "kotak-nse-nfo", NFO_INSTRUMENT);
    assert_eq!(nfo_bind["data"]["last"], "10.00");

    let options_bind = get_quote(PORT, FIXTURE_CONTRACT, Some("binance-com-options")).await;
    assert_bound(&options_bind, "binance-com-options", FIXTURE_CONTRACT);

    let nfo_obtain = get_obtain(PORT, "kotak-nse-nfo", "quotes").await;
    assert_eq!(nfo_obtain["status"], "success");
    assert_eq!(nfo_obtain["book_id"], "kotak-nse-nfo");
    assert_eq!(nfo_obtain["data"]["instrument_id"], NFO_INSTRUMENT);
    assert_eq!(nfo_obtain["data"]["last"], "10.00");
    assert_ne!(nfo_obtain["data"]["last"], options_bind["data"]["last"]);

    let options_obtain = get_obtain(PORT, "binance-com-options", "quotes").await;
    assert_eq!(options_obtain["status"], "success");
    assert_eq!(options_obtain["data"]["instrument_id"], FIXTURE_CONTRACT);
    assert_eq!(options_obtain["data"]["last"], "1.23");

    handle.abort();
}

/// Optionchain obtain is scoped to the options book selection, not NFO or spot leftovers.
#[tokio::test]
async fn optionchain_reads_only_options_selection() {
    const PORT: u16 = 19_643;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_kotak_nfo_quote: true,
            plant_binance_options_quote: true,
            plant_binance_options_chain: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    // NFO alone must not light the options chain.
    let _ = get_quote(PORT, NFO_INSTRUMENT, None).await;
    let dark = get_obtain(PORT, "binance-com-options", "optionchain").await;
    assert_eq!(dark["status"], "unavailable");
    assert!(dark["data"].is_null());

    let bind = get_quote(PORT, FIXTURE_CONTRACT, Some("binance-com-options")).await;
    assert_bound(&bind, "binance-com-options", FIXTURE_CONTRACT);

    let chain = get_obtain(PORT, "binance-com-options", "optionchain").await;
    assert_eq!(chain["status"], "success");
    assert_eq!(chain["data"]["row_count"], 3);
    assert_eq!(chain["data"]["rows"][0]["instrument_id"], FIXTURE_CONTRACT);

    // Re-bind NFO: options chain must stay on the dated contract.
    let _ = get_quote(PORT, NFO_INSTRUMENT, None).await;
    let still_options = get_obtain(PORT, "binance-com-options", "optionchain").await;
    assert_eq!(still_options["status"], "success");
    assert_eq!(
        still_options["data"]["rows"][0]["instrument_id"],
        FIXTURE_CONTRACT
    );

    handle.abort();
}

/// Options depth serves only the contract bound on the options book.
#[tokio::test]
async fn depth_reads_only_the_same_selected_contract() {
    const PORT: u16 = 19_644;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_depth: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let bind = get_quote(PORT, FIXTURE_CONTRACT, Some("binance-com-options")).await;
    assert_bound(&bind, "binance-com-options", FIXTURE_CONTRACT);

    let depth = get_obtain(PORT, "binance-com-options", "depth").await;
    assert_eq!(depth["status"], "success");
    assert_eq!(depth["data"]["instrument_id"], FIXTURE_CONTRACT);

    // A different dated contract on the same book has no planted ladder.
    let _ = get_quote(PORT, "ETH-200730-400-C", Some("binance-com-options")).await;
    let dark = get_obtain(PORT, "binance-com-options", "depth").await;
    assert_eq!(dark["status"], "unavailable");
    assert!(dark["data"].is_null());
    assert!(!dark.to_string().contains(FIXTURE_CONTRACT));

    handle.abort();
}

/// Unknown or mismatched `book=` is refused — never silently rerouted.
#[tokio::test]
async fn mismatched_book_query_is_refused() {
    const PORT: u16 = 19_645;
    let handle = spawn_test_agent(PORT);
    wait_for_quote_route(PORT).await;

    for (instrument, book, book_id, class) in [
        (
            FIXTURE_CONTRACT,
            "kotak-nse-nfo",
            "binance-com-options",
            "instrument_book_mismatch",
        ),
        (
            FIXTURE_CONTRACT,
            "binance-com-spot",
            "binance-com-options",
            "instrument_book_mismatch",
        ),
        (
            NFO_INSTRUMENT,
            "binance-com-options",
            "kotak-nse-nfo",
            "instrument_book_mismatch",
        ),
        ("BTCUSDT", "nonsense", "nonsense", "unknown_book"),
    ] {
        let body = get_quote(PORT, instrument, Some(book)).await;
        assert_refused(&body, book_id, class);
    }

    handle.abort();
}

/// HTTP seam: binding a dated contract must not subscribe the spot slot.
/// Stream-key proof: `api::quote::tests::dated_contract_records_options_key_and_no_spot_key`.
#[tokio::test]
async fn dated_contract_never_opens_binance_spot_streams() {
    const PORT: u16 = 19_646;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let bind = get_quote(PORT, FIXTURE_CONTRACT, None).await;
    assert_bound(&bind, "binance-com-options", FIXTURE_CONTRACT);

    for spot_id in ["BTCUSDT", "btcusdt"] {
        let spot = get_quote(PORT, spot_id, None).await;
        assert_eq!(spot["status"], "unavailable", "{spot_id}");
        assert!(spot["data"].is_null(), "{spot_id}");
        assert_ne!(spot["data"]["last"], bind["data"]["last"]);
    }

    let spot_obtain = get_obtain(PORT, "binance-com-spot", "quotes").await;
    assert_eq!(spot_obtain["status"], "unavailable");
    assert!(spot_obtain["data"].is_null());

    let spot_depth = get_obtain(PORT, "binance-com-spot", "depth").await;
    assert_ne!(spot_depth["status"], "success");
    assert!(spot_depth["data"].is_null());

    handle.abort();
}

/// Scoped stop: Kotak books clear; public Binance options selection survives.
#[tokio::test]
#[serial]
async fn stopping_kotak_does_not_clear_public_options_selection() {
    const PORT: u16 = 19_647;
    let counter = Arc::new(CountingPollAdapter::new());
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_runtime_opts(
            counter.clone(),
            TestAgentOptions {
                plant_kotak_nfo_quote: true,
                plant_binance_options_quote: true,
                ..TestAgentOptions::default()
            },
        ),
    );
    wait_for_quote_route(PORT).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("kotak_neo"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;

    let options_bind = get_quote(PORT, FIXTURE_CONTRACT, Some("binance-com-options")).await;
    assert_bound(&options_bind, "binance-com-options", FIXTURE_CONTRACT);
    let _ = get_quote(PORT, NFO_INSTRUMENT, None).await;

    assert_eq!(post_broker_sync_stop(PORT).await.status(), 200);
    tokio::time::sleep(Duration::from_millis(150)).await;

    let options_obtain = get_obtain(PORT, "binance-com-options", "quotes").await;
    assert_eq!(options_obtain["status"], "success");
    assert_eq!(options_obtain["data"]["instrument_id"], FIXTURE_CONTRACT);
    assert_eq!(options_obtain["data"]["last"], "1.23");

    let nfo_obtain = get_obtain(PORT, "kotak-nse-nfo", "quotes").await;
    assert_eq!(nfo_obtain["status"], "unavailable");

    handle.abort();
}

/// Refused binds must not echo the raw instrument as though binding succeeded.
#[tokio::test]
async fn invalid_binds_do_not_echo_successful_looking_instrument() {
    const PORT: u16 = 19_648;
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_options_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_for_quote_route(PORT).await;

    let nfo_on_options = get_quote(PORT, NFO_INSTRUMENT, Some("binance-com-options")).await;
    assert_refused(&nfo_on_options, "kotak-nse-nfo", "instrument_book_mismatch");
    assert_ne!(nfo_on_options["instrument_id"], NFO_INSTRUMENT);

    let dated_on_spot = get_quote(PORT, FIXTURE_CONTRACT, Some("binance-com-spot")).await;
    assert_refused(
        &dated_on_spot,
        "binance-com-options",
        "instrument_book_mismatch",
    );
    assert_ne!(dated_on_spot["instrument_id"], FIXTURE_CONTRACT);

    let garbage = get_quote(PORT, "NOT_A_REAL_ID", None).await;
    assert_eq!(garbage["bind_status"], "refused");
    assert_eq!(garbage["instrument_id"], "");
    assert_eq!(garbage["status"], "unavailable");
    assert!(garbage["data"].is_null());

    handle.abort();
}
