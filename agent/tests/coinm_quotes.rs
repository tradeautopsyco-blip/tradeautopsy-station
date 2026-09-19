//! Coin-M public last on book `binance-com-coinm`.
//!
//! Lock: `issues/compliance/locks/binance-com-coinm.md`. REST `GET /dapi/v1/ticker/price`
//! field `price`. Matching is `book_id`. `BTCUSD_PERP` never shares TickBook with
//! `BTCUSDT` USDM. TRADE stays MutationForbidden.

mod common;

use common::{spawn_test_agent_with_options, wait_ready, TestAgentOptions};
use serial_test::serial;

const PORT: u16 = 19_680;

#[tokio::test]
#[serial]
async fn planted_coinm_last_binds_coinm_book_not_usdm() {
    let handle = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            plant_binance_coinm_quote: true,
            plant_binance_coinm_exchange_info: true,
            plant_binance_usdm_quote: true,
            ..TestAgentOptions::default()
        },
    );
    wait_ready(PORT).await;
    let client = reqwest::Client::new();

    let bind: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSD_PERP&book=binance-com-coinm"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind coinm")
        .json()
        .await
        .expect("json");
    assert_eq!(bind["bind_status"], "bound");
    assert_eq!(bind["book_id"], "binance-com-coinm");
    assert_ne!(bind["book_id"], "binance-com-usdm");
    assert_eq!(bind["instrument_id"], "BTCUSD_PERP");
    assert_eq!(bind["data"]["last"], "65000.10");
    assert_eq!(bind["tick_size"], "0.10");
    assert_ne!(bind["tick_size"], "8");
    assert_eq!(bind["step_size"], "1");

    let usdm: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/quote?instrument=BTCUSDT&book=binance-com-usdm"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("bind usdm")
        .json()
        .await
        .expect("json");
    assert_eq!(usdm["book_id"], "binance-com-usdm");
    assert_ne!(usdm["instrument_id"], "BTCUSD_PERP");

    let search: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{PORT}/api/station/obtain?book=binance-com-coinm&operation=search&q=btcusd"
        ))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .expect("coinm search")
        .json()
        .await
        .expect("json");
    assert_eq!(search["status"], "success");
    assert_eq!(search["book_id"], "binance-com-coinm");
    let first = &search["data"]["rows"][0];
    assert_eq!(first["segment"], "COINM");
    assert_ne!(first["segment"], "USDM");
    assert_eq!(first["trading_symbol"], "BTCUSD_PERP");

    handle.abort();
}
