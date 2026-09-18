//! T4 — Binance COM USDM public last on book `binance-com-usdm`.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md`. REST `GET /fapi/v1/ticker/price`
//! field `price`. Matching is `book_id`, never letters alone. Bookless BTCUSDT stays
//! spot. HMAC never attaches. CI plants the committed fixture — no live fapi.

mod common;

use common::{spawn_test_agent_with_options, wait_ready, TestAgentOptions};
use serial_test::serial;

const BIND_PORT: u16 = 19_670;
const SLUG_PORT: u16 = 19_671;

#[tokio::test]
#[serial]
async fn planted_usdm_last_binds_usdm_book_and_stays_off_spot() {
    let handle = spawn_test_agent_with_options(
        BIND_PORT,
        TestAgentOptions {
            plant_binance_usdm_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_ready(BIND_PORT).await;
    let client = reqwest::Client::new();

    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{BIND_PORT}/api/station/quote?instrument=BTCUSDT&book=binance-com-usdm"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind usdm")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["bind_status"], "bound");
    assert_eq!(bind["book_id"], "binance-com-usdm");
    assert_eq!(bind["instrument_id"], "BTCUSDT");
    assert_ne!(bind["instrument_id"], "btcusdt");
    assert_eq!(bind["data"]["last"], "65000.10");
    assert_ne!(bind["data"]["last"], "0");

    let obtain: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{BIND_PORT}/api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("usdm obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(obtain["status"], "success");
    assert_eq!(obtain["book_id"], "binance-com-usdm");
    assert_eq!(obtain["data"]["last"], "65000.10");
    assert_ne!(obtain["data"]["last"], "0");
    assert_eq!(obtain["data"]["instrument_id"], "BTCUSDT");
    assert_ne!(obtain["data"]["instrument_id"], "btcusdt");
    assert_ne!(obtain["status"], "unsupported");

    let bookless: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{BIND_PORT}/api/station/quote?instrument=BTCUSDT"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bookless quote")
        .json()
        .await
        .expect("json");
    assert_eq!(bookless["bind_status"], "bound");
    assert_eq!(bookless["book_id"], "binance-com-spot");
    assert_ne!(bookless["book_id"], "binance-com-usdm");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn slug_only_obtain_quotes_stays_spot() {
    let handle = spawn_test_agent_with_options(
        SLUG_PORT,
        TestAgentOptions {
            plant_binance_usdm_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_ready(SLUG_PORT).await;
    let client = reqwest::Client::new();

    let slug: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{SLUG_PORT}/api/station/obtain?adapter=binance_com&operation=quotes"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("slug obtain")
        .json()
        .await
        .expect("json");
    assert_eq!(slug["book_id"], "binance-com-spot");
    assert_ne!(slug["book_id"], "binance-com-usdm");
    assert!(slug["data"].is_null() || slug["data"]["instrument_id"] != "BTCUSDT");

    handle.abort();
}
