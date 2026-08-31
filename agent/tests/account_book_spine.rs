//! Step 0 spine — book-keyed AccountBook obtain, merge_poll fork, no COM-count-on-cash leak.

mod common;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    TestAgentOptions, WireHeaderOverrides,
};
use serde_json::Value;
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerBalancesSnapshot, BrokerCredentialVault, BrokerFill,
    BrokerOpenOrdersSnapshot, ConfigurableDataClassAdapter, DataClassPollRound,
};

/// Adapter with a fixed `name()` so split/merge uses the real venue slug rules.
#[derive(Clone)]
struct NamedFillAdapter {
    adapter_name: &'static str,
    fills: Arc<Mutex<Vec<BrokerFill>>>,
}

impl NamedFillAdapter {
    fn binance(fills: Vec<BrokerFill>) -> Arc<Self> {
        Arc::new(Self {
            adapter_name: "binance_com",
            fills: Arc::new(Mutex::new(fills)),
        })
    }

    fn kotak(fills: Vec<BrokerFill>) -> Arc<Self> {
        Arc::new(Self {
            adapter_name: "kotak_neo",
            fills: Arc::new(Mutex::new(fills)),
        })
    }
}

#[async_trait]
impl BrokerAdapter for NamedFillAdapter {
    fn name(&self) -> &'static str {
        self.adapter_name
    }

    async fn poll_fills(
        &self,
        _since: Option<chrono::DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, tradeautopsy_agent::BrokerError> {
        Ok(self.fills.lock().expect("fills mutex").clone())
    }

    async fn poll_balances_holdings(
        &self,
    ) -> Result<BrokerBalancesSnapshot, tradeautopsy_agent::BrokerError> {
        Ok(BrokerBalancesSnapshot::default())
    }

    async fn poll_open_orders(
        &self,
    ) -> Result<BrokerOpenOrdersSnapshot, tradeautopsy_agent::BrokerError> {
        Ok(BrokerOpenOrdersSnapshot::empty())
    }
}

fn com_fill() -> BrokerFill {
    BrokerFill {
        fill_id: "F-BTC".into(),
        trade_id: "T-BTC".into(),
        symbol: "BTCUSDT".into(),
        side: "BUY".into(),
        qty: 0.01,
        price: 65_000.0,
        filled_at: Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap(),
        broker: "binance_com".into(),
        currency: Some("USDT".into()),
        ..BrokerFill::default()
    }
}

fn kotak_cash_fill() -> BrokerFill {
    BrokerFill {
        fill_id: "F-CASH".into(),
        trade_id: "T-CASH".into(),
        symbol: "nse_cm|2885".into(),
        side: "BUY".into(),
        qty: 1.0,
        price: 1400.0,
        filled_at: Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap(),
        broker: "kotak_neo".into(),
        currency: Some("INR".into()),
        product: Some("CNC".into()),
        exchange_segment: Some("nse_cm".into()),
        ..BrokerFill::default()
    }
}

fn kotak_nfo_fill() -> BrokerFill {
    BrokerFill {
        fill_id: "F-NFO".into(),
        trade_id: "T-NFO".into(),
        symbol: "nse_fo|12345".into(),
        side: "BUY".into(),
        qty: 50.0,
        price: 10.0,
        filled_at: Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap(),
        broker: "kotak_neo".into(),
        currency: Some("INR".into()),
        product: Some("NRML".into()),
        exchange_segment: Some("nse_fo".into()),
        lot: Some(50),
        ..BrokerFill::default()
    }
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
    .expect("start")
}

async fn obtain_tradebook(port: u16, book: &str) -> Value {
    let path = format!("/api/station/obtain?book={book}&operation=tradebook");
    let url = format!("http://127.0.0.1:{port}{path}");
    client()
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("obtain")
        .json()
        .await
        .expect("obtain json")
}

fn start_opts(adapter: Arc<dyn BrokerAdapter>, slug: &str) -> TestAgentOptions {
    let vault = seeded_hmac_vault(slug, "TA_TEST_SYNC");
    TestAgentOptions {
        runtime_poll_adapter: Some(adapter),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    }
}

#[tokio::test]
#[serial]
async fn empty_binance_poll_obtain_returns_rows_empty_not_unavailable() {
    const PORT: u16 = 19_600;
    let adapter = Arc::new(ConfigurableDataClassAdapter::all_ok()) as Arc<dyn BrokerAdapter>;
    // ConfigurableDataClassAdapter name won't split — use NamedFillAdapter with empty fills.
    let named = NamedFillAdapter::binance(vec![]);
    let handle = spawn_test_agent_with_options(
        PORT,
        start_opts(named as Arc<dyn BrokerAdapter>, "binance_us"),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;

    let envelope = obtain_tradebook(PORT, "binance-com-spot").await;
    assert_eq!(envelope["status"], "success");
    assert_eq!(envelope["data"]["fill_count"], 0);
    assert!(envelope["data"]["rows"].as_array().unwrap().is_empty());
    assert_eq!(envelope["provenance_path"], "/api/v3/myTrades");

    let _ = adapter;
    handle.abort();
}

#[tokio::test]
#[serial]
async fn binance_fills_do_not_light_kotak_cash_tradebook() {
    const PORT: u16 = 19_601;
    let named = NamedFillAdapter::binance(vec![com_fill()]);
    let handle = spawn_test_agent_with_options(
        PORT,
        start_opts(named as Arc<dyn BrokerAdapter>, "binance_us"),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("binance_us"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;

    let cash = obtain_tradebook(PORT, "kotak-nse-bse-cash").await;
    assert_eq!(
        cash["status"], "unavailable",
        "cash tradebook must not inherit COM fill_count"
    );

    let spot = obtain_tradebook(PORT, "binance-com-spot").await;
    assert_eq!(spot["status"], "success");
    assert_eq!(spot["data"]["fill_count"], 1);
    assert_eq!(spot["data"]["rows"][0]["symbol"], "BTCUSDT");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn kotak_split_writes_cash_and_nfo_books_separately() {
    const PORT: u16 = 19_602;
    let named = NamedFillAdapter::kotak(vec![kotak_cash_fill(), kotak_nfo_fill()]);
    let handle = spawn_test_agent_with_options(
        PORT,
        start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo"),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        post_broker_sync_start(PORT, identity_start_body("kotak_neo"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;

    let cash = obtain_tradebook(PORT, "kotak-nse-bse-cash").await;
    assert_eq!(cash["status"], "success");
    assert_eq!(cash["data"]["fill_count"], 1);
    assert_eq!(cash["data"]["rows"][0]["segment"], "nse_cm");
    assert!(
        cash["data"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["segment"] != "nse_fo"),
        "cash obtain must not contain nse_fo rows"
    );

    // NFO book slot is written but tradebook manifest lights in F1.2 — obtain may stay unavailable
    // until manifest adds tradebook on kotak-nse-nfo. Cash separation is the Step 0 invariant.

    handle.abort();
}
