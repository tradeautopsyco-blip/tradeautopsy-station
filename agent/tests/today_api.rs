//! Today v1 API + persistence integration tests.

mod common;

use chrono::{DateTime, TimeZone, Utc};
use common::{
    apply_wire_v1, client, seeded_hmac_vault, spawn_test_agent_with_options, TestAgentOptions,
    TEST_SECRET,
};
use serial_test::serial;
use std::time::Duration;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tradeautopsy_agent::{
    BrokerCredentialVault, BrokerError, BrokerFill, RecentTradesStore, SeqMockBrokerAdapter,
};

const PORT: u16 = 39650;

fn fill(id: &str, side: &str, qty: f64, price: f64, h: u32, m: u32) -> BrokerFill {
    BrokerFill {
        fill_id: id.to_string(),
        trade_id: format!("t-{id}"),
        symbol: "BTCUSDT".to_string(),
        side: side.to_string(),
        qty,
        price,
        filled_at: Utc.with_ymd_and_hms(2026, 7, 4, h, m, 0).unwrap(),
        broker: "binance_us".to_string(),
        fee_amount: Some(0.5),
        fee_asset: Some("USDT".to_string()),
        ..Default::default()
    }
}

async fn seed_fills(db_path: &std::path::Path) {
    let store = RecentTradesStore::open(db_path).expect("open recent db");
    store.upsert_fill(&fill("b1", "BUY", 0.01, 60_000.0, 10, 0)).unwrap();
    store
        .upsert_fill(&fill("s1", "SELL", 0.01, 61_000.0, 14, 0))
        .unwrap();
}

#[tokio::test]
#[serial]
async fn today_api_returns_json_when_broker_syncing() {
    let db = std::env::temp_dir().join(format!("rta-today-api-{PORT}.db"));
    let _ = std::fs::remove_file(&db);
    seed_fills(&db).await;

    let q = Arc::new(Mutex::new(VecDeque::from([Ok::<Vec<BrokerFill>, BrokerError>(vec![])])));
    let adapter = Arc::new(SeqMockBrokerAdapter { calls: q });
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    // today_api uses a distinct connection id — seed that slot too.
    vault
        .save(
            "prod",
            "binance_us",
            "00000000-0000-4000-8000-000000000099",
            &tradeautopsy_agent::CredentialBlob::hmac("TA_TEST_SYNC", "secret"),
        )
        .unwrap();
    let _agent = spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            recent_trades_db_path: Some(db),
            runtime_poll_adapter: Some(adapter.clone()),
            credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
            ..Default::default()
        },
    );
    tokio::time::sleep(Duration::from_millis(400)).await;

    let start_body = serde_json::json!({
        "brokerSlug": "binance_us",
        "brokerConnectionId": "00000000-0000-4000-8000-000000000099",
        "environment": "prod",
        "assetClass": "crypto"
    });
    let body = serde_json::to_vec(&start_body).unwrap();
    let req = apply_wire_v1(
        client()
            .post(format!("http://127.0.0.1:{PORT}/api/daemon/broker/sync/start"))
            .body(body.clone()),
        "POST",
        "/api/daemon/broker/sync/start",
        &body,
        Default::default(),
    );
    let _ = req.send().await.expect("start sync");

    tokio::time::sleep(Duration::from_millis(2500)).await;

    let req = apply_wire_v1(
        client().get(format!("http://127.0.0.1:{PORT}/api/daemon/today")),
        "GET",
        "/api/daemon/today",
        &[],
        Default::default(),
    );
    let resp = req.send().await.expect("today get");
    assert_eq!(resp.status(), 200);
    let j: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(j["performanceBasisNotTax"], true);
    assert!(j.get("hero").is_some());
    assert!(j.get("topSignals").is_some());
}

#[tokio::test]
#[serial]
async fn today_api_degraded_when_sync_paused() {
    let _agent = spawn_test_agent_with_options(PORT + 1, TestAgentOptions::default());
    tokio::time::sleep(Duration::from_millis(300)).await;

    let req = apply_wire_v1(
        client().get(format!("http://127.0.0.1:{}/api/daemon/today", PORT + 1)),
        "GET",
        "/api/daemon/today",
        &[],
        Default::default(),
    );
    let resp = req.send().await.expect("today get");
    let j: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(j["degradedReason"], "sync_unavailable");
    assert!(j["hero"]["pnlTodayUsd"].is_null());
}
