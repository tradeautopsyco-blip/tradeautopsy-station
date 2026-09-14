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
    BrokerAdapter, BrokerBalancesSnapshot, BrokerCredentialVault, BrokerError, BrokerFill,
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

/// Always-Err fills poll — slot must stay `None` (obtain unavailable, not stale success).
#[derive(Clone)]
struct ErrFillAdapter {
    adapter_name: &'static str,
}

impl ErrFillAdapter {
    fn kotak() -> Arc<Self> {
        Arc::new(Self {
            adapter_name: "kotak_neo",
        })
    }
}

#[async_trait]
impl BrokerAdapter for ErrFillAdapter {
    fn name(&self) -> &'static str {
        self.adapter_name
    }

    async fn poll_fills(
        &self,
        _since: Option<chrono::DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        Err(BrokerError::Http("poll failed".into()))
    }

    async fn poll_balances_holdings(&self) -> Result<BrokerBalancesSnapshot, BrokerError> {
        Ok(BrokerBalancesSnapshot::default())
    }

    async fn poll_open_orders(&self) -> Result<BrokerOpenOrdersSnapshot, BrokerError> {
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

fn kotak_nfo_fill_sym(symbol: &str, product: &str, qty: f64, exchange_segment: &str) -> BrokerFill {
    BrokerFill {
        fill_id: format!("F-NFO-{symbol}-{product}"),
        trade_id: format!("T-NFO-{symbol}"),
        symbol: symbol.into(),
        side: "BUY".into(),
        qty,
        price: 10.0,
        filled_at: Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap(),
        broker: "kotak_neo".into(),
        currency: Some("INR".into()),
        product: Some(product.into()),
        exchange_segment: Some(exchange_segment.into()),
        ..BrokerFill::default()
    }
}

/// Lock golden `NIFTY2692221000PE` — lot 65 comes from master stamp when planted.
fn kotak_nfo_fill() -> BrokerFill {
    kotak_nfo_fill_sym("NIFTY2692221000PE", "NRML", 50.0, "nse_fo")
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
    .expect("stop")
}

async fn fetch_recent_trades(port: u16) -> Value {
    let path = "/api/daemon/toolbar/recent-trades";
    let url = format!("http://127.0.0.1:{port}{path}");
    apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("recent-trades")
    .json()
    .await
    .expect("recent-trades json")
}

fn assert_dual_no_blend_tradebook_wire(envelope: &Value) {
    let wire = serde_json::to_string(envelope).expect("wire json");
    for forbidden in ["exchange_rate", "eapi", "usd", "USD"] {
        assert!(
            !wire.contains(forbidden),
            "NFO tradebook envelope must not blend COM/eapi fields: found {forbidden:?} in {wire}"
        );
    }
    assert_eq!(envelope["data"]["rows"][0]["currency"], "INR");
}

fn start_opts(adapter: Arc<dyn BrokerAdapter>, slug: &str) -> TestAgentOptions {
    kotak_start_opts(adapter, slug, false)
}

fn kotak_start_opts(
    adapter: Arc<dyn BrokerAdapter>,
    slug: &str,
    plant_kotak_nfo_contracts: bool,
) -> TestAgentOptions {
    let vault = seeded_hmac_vault(slug, "TA_TEST_SYNC");
    TestAgentOptions {
        runtime_poll_adapter: Some(adapter),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        plant_kotak_nfo_contracts,
        ..TestAgentOptions::default()
    }
}

async fn start_kotak_sync(port: u16) {
    assert_eq!(
        post_broker_sync_start(port, identity_start_body("kotak_neo"))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;
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
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

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

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 1);
    assert_eq!(nfo["data"]["rows"][0]["symbol"], "NIFTY2692221000PE");
    assert_eq!(nfo["data"]["rows"][0]["product"], "NRML");
    assert_eq!(nfo["provenance_path"], "/quick/user/trades");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn nfo_tradebook_unavailable_before_sync() {
    const PORT: u16 = 19_610;
    let named = NamedFillAdapter::kotak(vec![kotak_nfo_fill()]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "unavailable");
    assert!(nfo["data"].is_null());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn kotak_empty_nfo_poll_success_empty_rows() {
    const PORT: u16 = 19_611;
    let named = NamedFillAdapter::kotak(vec![]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 0);
    assert!(nfo["data"]["rows"].as_array().unwrap().is_empty());
    assert_eq!(nfo["provenance_path"], "/quick/user/trades");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn optstk_mis_dropped_from_nfo_obtain() {
    const PORT: u16 = 19_613;
    let fill = kotak_nfo_fill_sym("RELIANCE26SEP2400CE", "MIS", 1.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 0);
    assert!(nfo["data"]["rows"].as_array().unwrap().is_empty());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn optidx_mis_kept_in_nfo_obtain() {
    const PORT: u16 = 19_614;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "MIS", 2.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 1);
    assert_eq!(nfo["data"]["rows"][0]["symbol"], "NIFTY2692221000PE");
    assert_eq!(nfo["data"]["rows"][0]["product"], "MIS");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn cnc_on_nfo_dropped() {
    const PORT: u16 = 19_615;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "CNC", 1.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 0);

    let cash = obtain_tradebook(PORT, "kotak-nse-bse-cash").await;
    assert_eq!(cash["status"], "success");
    assert_eq!(cash["data"]["fill_count"], 0);

    handle.abort();
}

#[tokio::test]
#[serial]
async fn nfo_lot_65_exact_symbol() {
    const PORT: u16 = 19_616;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "NRML", 2.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 1);
    let row = &nfo["data"]["rows"][0];
    assert_eq!(row["symbol"], "NIFTY2692221000PE");
    assert_eq!(row["qty"], 2.0);
    assert_eq!(row["lot"], 65);
    assert_eq!(row["instrument_type"], "PE");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn nfo_lot_omitted_on_symbol_miss() {
    const PORT: u16 = 19_617;
    let fill = kotak_nfo_fill_sym("NIFTY25JUL24000CE", "NRML", 1.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 1);
    let row = &nfo["data"]["rows"][0];
    assert_eq!(row["symbol"], "NIFTY25JUL24000CE");
    assert!(row.get("lot").is_none(), "unknown symbol must omit lot key");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn nfo_exseg_nse_fo_normalized() {
    const PORT: u16 = 19_618;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "NRML", 1.0, "NSE_FO");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 1);
    assert_eq!(nfo["data"]["rows"][0]["segment"], "NSE_FO");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_does_not_start_sync() {
    const PORT: u16 = 19_619;
    let named = NamedFillAdapter::kotak(vec![kotak_nfo_fill()]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "unavailable");
    assert!(nfo["data"].is_null());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn bo_dropped_from_nfo_obtain() {
    const PORT: u16 = 19_620;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "BO", 1.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 0);
    assert!(nfo["data"]["rows"].as_array().unwrap().is_empty());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn nfo_tradebook_dual_no_blend_envelope() {
    const PORT: u16 = 19_621;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "NRML", 2.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_dual_no_blend_tradebook_wire(&nfo);
    let row = &nfo["data"]["rows"][0];
    assert!(row.get("fees").is_none(), "chrgs absent must omit fees key");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn err_poll_never_ok_stays_unavailable() {
    const PORT: u16 = 19_622;
    let err = ErrFillAdapter::kotak();
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(err as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;
    tokio::time::sleep(Duration::from_millis(350)).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(
        nfo["status"], "unavailable",
        "Err-only poll must not create a slot"
    );
    assert!(nfo["data"].is_null());

    let cash = obtain_tradebook(PORT, "kotak-nse-bse-cash").await;
    assert_eq!(cash["status"], "unavailable");
    assert!(cash["data"].is_null());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn stop_kotak_clears_cash_and_nfo_obtain() {
    const PORT: u16 = 19_623;
    let named = NamedFillAdapter::kotak(vec![kotak_cash_fill(), kotak_nfo_fill()]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["fill_count"], 1);

    assert_eq!(post_broker_sync_stop(PORT).await.status(), 200);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let nfo_after = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo_after["status"], "unavailable");
    assert!(nfo_after["data"].is_null());

    let cash_after = obtain_tradebook(PORT, "kotak-nse-bse-cash").await;
    assert_eq!(cash_after["status"], "unavailable");
    assert!(cash_after["data"].is_null());

    handle.abort();
}

#[tokio::test]
#[serial]
async fn master_pe_wins_over_ce_suffix_on_obtain() {
    const PORT: u16 = 19_624;
    let mut fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "NRML", 2.0, "nse_fo");
    fill.instrument_type = Some("CE".into());
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    assert_eq!(nfo["status"], "success");
    assert_eq!(nfo["data"]["rows"][0]["instrument_type"], "PE");

    handle.abort();
}

#[tokio::test]
#[serial]
async fn instrument_type_never_nrml_or_optidx_on_obtain() {
    const PORT: u16 = 19_625;
    let fill = kotak_nfo_fill_sym("NIFTY2692221000PE", "NRML", 2.0, "nse_fo");
    let named = NamedFillAdapter::kotak(vec![fill]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;

    let nfo = obtain_tradebook(PORT, "kotak-nse-nfo").await;
    for row in nfo["data"]["rows"].as_array().unwrap() {
        let it = row
            .get("instrument_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !matches!(
                it,
                "NRML" | "OPTIDX" | "NSE" | "OPTSTK" | "FUTIDX" | "FUTSTK"
            ),
            "instrument_type must be desk CE/PE/FUT only, got {it:?}"
        );
    }

    handle.abort();
}

#[tokio::test]
#[serial]
async fn merge_poll_excludes_nfo_fills() {
    const PORT: u16 = 19_626;
    let named = NamedFillAdapter::kotak(vec![kotak_cash_fill(), kotak_nfo_fill()]);
    let handle = spawn_test_agent_with_options(
        PORT,
        kotak_start_opts(named as Arc<dyn BrokerAdapter>, "kotak_neo", true),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;

    start_kotak_sync(PORT).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let recent = fetch_recent_trades(PORT).await;
    let trades = recent["trades"].as_array().expect("trades array");
    assert_eq!(trades.len(), 1, "merge_poll must ship cash only");
    assert_eq!(trades[0]["symbol"], "nse_cm|2885");
    assert!(
        trades.iter().all(|t| t["symbol"] != "NIFTY2692221000PE"),
        "NFO fills must not enter merge_poll / toolbar"
    );

    handle.abort();
}

#[tokio::test]
#[serial]
async fn stop_binance_clears_spot_obtain_only() {
    const PORT: u16 = 19_627;
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

    let spot = obtain_tradebook(PORT, "binance-com-spot").await;
    assert_eq!(spot["status"], "success");
    assert_eq!(spot["data"]["fill_count"], 1);

    assert_eq!(post_broker_sync_stop(PORT).await.status(), 200);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let spot_after = obtain_tradebook(PORT, "binance-com-spot").await;
    assert_eq!(spot_after["status"], "unavailable");
    assert!(spot_after["data"].is_null());

    let kotak_cash = obtain_tradebook(PORT, "kotak-nse-bse-cash").await;
    assert_eq!(
        kotak_cash["status"], "unavailable",
        "stop binance must not touch kotak books"
    );

    handle.abort();
}
