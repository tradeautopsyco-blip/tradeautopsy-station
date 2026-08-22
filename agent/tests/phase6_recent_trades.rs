//! Phase 6: recent trades read model + broker sync + `toolbar_show` (issue #63).

mod common;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use futures::StreamExt;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerError, BrokerFill, RecentTradesStore, SeqMockBrokerAdapter,
};

async fn sse_raw_contains(port: u16, needle: &str, deadline: Duration) -> bool {
    let path = "/api/daemon/events/stream";
    let url = format!("http://127.0.0.1:{port}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    let Ok(resp) = req.timeout(Duration::from_secs(30)).send().await else {
        return false;
    };
    if resp.status() != 200 {
        return false;
    }
    let mut stream = resp.bytes_stream();
    let mut acc = String::new();
    let sleep = tokio::time::sleep(deadline);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => break,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { break };
                acc.push_str(&String::from_utf8_lossy(&chunk));
                if acc.contains(needle) {
                    return true;
                }
            }
        }
    }
    acc.contains(needle)
}

async fn sse_collect_toolbar_events(port: u16, deadline: Duration, min: usize) -> Vec<Value> {
    let path = "/api/daemon/events/stream";
    let url = format!("http://127.0.0.1:{port}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    let resp = req
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("sse");
    assert_eq!(resp.status(), 200, "sse status");
    let mut stream = resp.bytes_stream();
    let mut toolbar = Vec::new();
    let mut backlog = String::new();
    let sleep = tokio::time::sleep(deadline);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => break,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { break };
                backlog.push_str(&String::from_utf8_lossy(&chunk));
                backlog.retain(|c| c != '\r');

                while let Some(pos) = backlog.find("\n\n") {
                    let frame = backlog[..pos].to_string();
                    backlog = backlog[pos + 2..].to_string();

                    let mut data_blob = String::new();
                    for line in frame.lines() {
                        let line = line.trim_end();
                        let Some(rest) = line.strip_prefix("data:") else { continue };
                        if !data_blob.is_empty() {
                            data_blob.push('\n');
                        }
                        data_blob.push_str(rest.trim());
                    }
                    if data_blob.is_empty() {
                        continue;
                    }
                    let Ok(val) = serde_json::from_str::<Value>(&data_blob) else { continue };
                    let t = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if t == "toolbar_show" {
                        toolbar.push(val.clone());
                        if toolbar.len() >= min {
                            return toolbar;
                        }
                    }
                }
            }
        }
    }
    toolbar
}

async fn sse_broker_sync_events(port: u16, deadline: Duration) -> Vec<Value> {
    let path = "/api/daemon/events/stream";
    let url = format!("http://127.0.0.1:{port}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    let resp = req
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("sse");
    assert_eq!(resp.status(), 200);
    let mut stream = resp.bytes_stream();
    let mut out = Vec::new();
    let mut backlog = String::new();
    let sleep = tokio::time::sleep(deadline);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => break,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { break };
                backlog.push_str(&String::from_utf8_lossy(&chunk));
                backlog.retain(|c| c != '\r');

                while let Some(pos) = backlog.find("\n\n") {
                    let frame = backlog[..pos].to_string();
                    backlog = backlog[pos + 2..].to_string();

                    let mut data_blob = String::new();
                    for line in frame.lines() {
                        let line = line.trim_end();
                        let Some(rest) = line.strip_prefix("data:") else { continue };
                        if !data_blob.is_empty() {
                            data_blob.push('\n');
                        }
                        data_blob.push_str(rest.trim());
                    }
                    if data_blob.is_empty() {
                        continue;
                    }
                    let Ok(val) = serde_json::from_str::<Value>(&data_blob) else { continue };
                    if val.get("type").and_then(|v| v.as_str()) == Some("broker_sync_state") {
                        out.push(val);
                    }
                }
            }
        }
    }
    out
}

#[tokio::test]
async fn get_recent_trades_local_not_connected_when_no_broker_adapter() {
    const PORT: u16 = 19_360;
    let path = "/api/daemon/toolbar/recent-trades";
    let handle = spawn_test_agent_with_options(PORT, TestAgentOptions::default());

    tokio::time::sleep(Duration::from_millis(300)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    let resp = req
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("get");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["syncState"], "not_connected");
    assert!(body["trades"].is_array());
    assert_eq!(body["trades"].as_array().unwrap().len(), 0);

    handle.abort();
}

fn sample_fill(suffix: &str) -> BrokerFill {
    BrokerFill {
        fill_id: format!("fill-{suffix}"),
        trade_id: uuid::Uuid::new_v4().to_string(),
        symbol: format!("SYM{suffix}"),
        side: "BUY".to_string(),
        qty: 10.0,
        price: 100.25,
        filled_at: chrono::Utc::now(),
        broker: "mock".to_string(),
        fee_amount: None,
        fee_asset: None,
        ..Default::default()
    }
}

#[derive(Clone)]
struct RecordingSinceAdapter {
    calls: Arc<Mutex<Vec<Option<DateTime<Utc>>>>>,
}

#[async_trait]
impl BrokerAdapter for RecordingSinceAdapter {
    fn name(&self) -> &'static str {
        "recording_since"
    }

    async fn poll_fills(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        self.calls.lock().expect("calls mutex").push(since);
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn first_broker_poll_starts_with_ninety_day_backfill_floor() {
    const PORT: u16 = 19_467;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let adapter = Arc::new(RecordingSinceAdapter {
        calls: calls.clone(),
    });
    let before_start = Utc::now();
    let opts = TestAgentOptions {
        broker_adapter: Some(adapter),
        broker_initial_delay_ms: 100,
        broker_base_poll_ms: 5_000,
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);

    tokio::time::sleep(Duration::from_millis(1_500)).await;

    let first_since = calls
        .lock()
        .expect("calls mutex")
        .first()
        .cloned()
        .flatten()
        .expect("first poll should request a backfill floor");
    let lower_bound = before_start - chrono::Duration::days(91);
    let upper_bound = Utc::now() - chrono::Duration::days(89);
    assert!(
        first_since >= lower_bound && first_since <= upper_bound,
        "expected first since near 90-day floor, got {first_since}"
    );

    handle.abort();
}

#[tokio::test]
async fn existing_recent_fills_resume_from_newest_fill_timestamp() {
    let mut db_path = std::env::temp_dir();
    db_path.push(format!("rta-existing-recent-{}.db", uuid::Uuid::new_v4()));
    let store = RecentTradesStore::open(&db_path).expect("open recent store");
    let newest_at = DateTime::parse_from_rfc3339("2026-06-28T09:00:00.000Z")
        .unwrap()
        .with_timezone(&Utc);
    let older_at = DateTime::parse_from_rfc3339("2026-06-26T09:00:00.000Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut newer = sample_fill("newer");
    newer.filled_at = newest_at;
    let mut older = sample_fill("older");
    older.filled_at = older_at;
    store
        .merge_poll(&[newer, older])
        .expect("insert unsorted fill batch");

    assert_eq!(store.newest_filled_at().unwrap(), Some(newest_at));

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn broker_poll_emits_toolbar_show_for_new_fill() {
    const PORT: u16 = 19_361;
    let fill = sample_fill("1");
    let q = Arc::new(Mutex::new(VecDeque::from([Ok(vec![fill.clone()])])));
    let adapter = Arc::new(SeqMockBrokerAdapter { calls: q });
    let opts = TestAgentOptions {
        broker_adapter: Some(adapter),
        broker_initial_delay_ms: 450,
        broker_base_poll_ms: 200,
        toolbar_coalesce_ms: 0,
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    let saw_toolbar = tokio::spawn(async move {
        sse_raw_contains(PORT, "toolbar_show", Duration::from_secs(8)).await
    });
    tokio::time::sleep(Duration::from_millis(350)).await;
    tokio::time::sleep(Duration::from_millis(1_800)).await;
    let toolbar_ok = saw_toolbar.await.expect("join");

    let rt_url = format!("http://127.0.0.1:{PORT}/api/daemon/toolbar/recent-trades");
    let rt_req = apply_wire_v1(
        client().get(&rt_url),
        "GET",
        "/api/daemon/toolbar/recent-trades",
        b"",
        WireHeaderOverrides::default(),
    );
    let read_model: Value = rt_req
        .send()
        .await
        .expect("recent trades")
        .json()
        .await
        .expect("recent json");
    let has_trade = read_model["trades"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    assert!(
        has_trade,
        "read model missing fill after poll — broker merge path regression: {:?}",
        read_model
    );
    assert!(
        toolbar_ok,
        "expected raw SSE to include toolbar_show, read_model_ok={}",
        has_trade
    );
    let trade = read_model["trades"]
        .get(0)
        .expect("persisted toolbar row must exist once read model asserts");
    assert_eq!(trade["tradeId"].as_str().unwrap(), fill.trade_id);
    assert_eq!(trade["symbol"].as_str().unwrap(), fill.symbol);

    handle.abort();
}

#[tokio::test]
async fn rapid_fills_coalesce_to_single_toolbar_show() {
    const PORT: u16 = 19_362;
    let a = sample_fill("A");
    let b = sample_fill("B");
    let c = sample_fill("C");
    let c_trade_id = c.trade_id.clone();
    let q = Arc::new(Mutex::new(VecDeque::from([Ok(vec![a, b, c])])));
    let adapter = Arc::new(SeqMockBrokerAdapter { calls: q });
    let opts = TestAgentOptions {
        broker_adapter: Some(adapter),
        broker_initial_delay_ms: 450,
        broker_base_poll_ms: 120,
        toolbar_coalesce_ms: 80,
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    let collect =
        tokio::spawn(
            async move { sse_collect_toolbar_events(PORT, Duration::from_secs(8), 3).await },
        );
    tokio::time::sleep(Duration::from_millis(350)).await;
    tokio::time::sleep(Duration::from_millis(1_800)).await;
    let toolbar = collect.await.expect("collector");
    assert!(
        toolbar.len() <= 1,
        "expected at most one toolbar_show for one poll burst, got {}",
        toolbar.len()
    );
    if let Some(ev) = toolbar.first() {
        assert_eq!(ev["payload"]["trade_id"], c_trade_id);
    }

    handle.abort();
}

#[derive(Clone)]
struct AlwaysFailAdapter;

#[async_trait]
impl BrokerAdapter for AlwaysFailAdapter {
    fn name(&self) -> &'static str {
        "always_fail"
    }

    async fn poll_fills(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        Err(BrokerError::Http("unit-fail".to_string()))
    }

    async fn poll_balances_holdings(
        &self,
    ) -> Result<tradeautopsy_agent::BrokerBalancesSnapshot, BrokerError> {
        Err(BrokerError::Http("unit-fail".to_string()))
    }

    async fn poll_open_orders(
        &self,
    ) -> Result<tradeautopsy_agent::BrokerOpenOrdersSnapshot, BrokerError> {
        Err(BrokerError::Http("unit-fail".to_string()))
    }
}

#[tokio::test]
async fn three_consecutive_failures_backoff_then_circuit_and_emits_sync_state() {
    const PORT: u16 = 19_363;
    let adapter = Arc::new(AlwaysFailAdapter);
    let opts = TestAgentOptions {
        broker_adapter: Some(adapter),
        broker_initial_delay_ms: 450,
        broker_base_poll_ms: 40,
        failure_escalate_after: 3,
        broker_backoff_tick_1_ms: 120,
        broker_backoff_tick_2_ms: 200,
        failures_until_open: 5,
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    let collect =
        tokio::spawn(async move { sse_broker_sync_events(PORT, Duration::from_secs(8)).await });
    tokio::time::sleep(Duration::from_millis(350)).await;
    tokio::time::sleep(Duration::from_millis(5_000)).await;
    let events = collect.await.expect("collector");
    assert!(!events.is_empty(), "expected broker_sync_state events");

    let classes: Vec<String> = events
        .iter()
        .filter_map(|e| e["payload"]["class"].as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        classes.iter().any(|c| c == "polling"),
        "expected startup class, got {:?}",
        classes
    );
    assert!(
        classes
            .iter()
            .any(|c| c == "disconnected" || c == "broker_backoff" || c == "circuit_open"),
        "expected failure ladder, got {:?}",
        classes
    );

    handle.abort();
}

#[tokio::test]
async fn stale_sync_state_when_success_is_aged() {
    const PORT: u16 = 19_364;
    let fill = sample_fill("stale");
    // Second poll must not refresh last_success_at_ms (empty Ok would keep "synced" fresh).
    let q = Arc::new(Mutex::new(VecDeque::from([
        Ok(vec![fill.clone()]),
        Err(BrokerError::Http("test_hold".into())),
    ])));
    let adapter = Arc::new(SeqMockBrokerAdapter { calls: q });
    let fresh_secs = 1_u64;
    let opts = TestAgentOptions {
        broker_adapter: Some(adapter),
        broker_initial_delay_ms: 100,
        broker_base_poll_ms: 60_000,
        fresh_secs,
        stale_secs: 60,
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);

    let path = "/api/daemon/toolbar/recent-trades";
    let url = format!("http://127.0.0.1:{PORT}{path}");

    let synced = wait_for_recent_trades_sync_state(&url, "synced", 5_000).await;
    assert_eq!(synced.as_str(), "synced");

    // Whole-second age bucketing: need strictly more than fresh_secs elapsed since last success.
    tokio::time::sleep(Duration::from_millis((fresh_secs + 1) * 1_000)).await;

    let stale = fetch_recent_trades_sync_state(&url)
        .await
        .expect("recent-trades response");
    assert_eq!(stale.as_str(), "stale");

    handle.abort();
}

async fn fetch_recent_trades_sync_state(url: &str) -> Option<String> {
    let req = apply_wire_v1(
        client().get(url),
        "GET",
        "/api/daemon/toolbar/recent-trades",
        b"",
        WireHeaderOverrides::default(),
    );
    let resp = req.timeout(Duration::from_secs(2)).send().await.ok()?;
    let body: Value = resp.json().await.ok()?;
    body["syncState"].as_str().map(str::to_string)
}

async fn wait_for_recent_trades_sync_state(url: &str, want: &str, timeout_ms: u64) -> String {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Some(state) = fetch_recent_trades_sync_state(url).await {
            if state == want {
                return state;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for syncState={want}, last={state}"
            );
        } else {
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for agent at {url}"
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn get_broker_sync_state_returns_200_when_not_connected() {
    const PORT: u16 = 19_365;
    let path = "/api/daemon/broker/sync-state";
    let handle = spawn_test_agent_with_options(PORT, TestAgentOptions::default());
    tokio::time::sleep(Duration::from_millis(300)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );
    let resp = req
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("get");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["syncState"], "not_connected");
    assert_eq!(body["runtimeStatus"], "ready_to_start");
    assert!(body["lastPollAtMs"].is_null());
    assert!(body["lastSuccessAtMs"].is_null());

    handle.abort();
}

#[tokio::test]
async fn broker_sync_state_matches_recent_trades_sync_posture() {
    const PORT: u16 = 19_366;
    let fill = sample_fill("syncalign");
    let q = Arc::new(Mutex::new(VecDeque::from([Ok(vec![fill])])));
    let adapter = Arc::new(SeqMockBrokerAdapter { calls: q });
    let opts = TestAgentOptions {
        broker_adapter: Some(adapter),
        broker_initial_delay_ms: 450,
        broker_base_poll_ms: 200,
        toolbar_coalesce_ms: 0,
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    tokio::time::sleep(Duration::from_millis(950)).await;

    let rt_path = "/api/daemon/toolbar/recent-trades";
    let bs_path = "/api/daemon/broker/sync-state";
    let rt_url = format!("http://127.0.0.1:{PORT}{rt_path}");
    let bs_url = format!("http://127.0.0.1:{PORT}{bs_path}");
    let rt_req = apply_wire_v1(
        client().get(&rt_url),
        "GET",
        rt_path,
        b"",
        WireHeaderOverrides::default(),
    );
    let bs_req = apply_wire_v1(
        client().get(&bs_url),
        "GET",
        bs_path,
        b"",
        WireHeaderOverrides::default(),
    );

    let (rt, bs) = tokio::join!(
        async {
            rt_req
                .send()
                .await
                .expect("recent trades")
                .json::<Value>()
                .await
                .expect("recent json")
        },
        async {
            bs_req
                .send()
                .await
                .expect("sync-state")
                .json::<Value>()
                .await
                .expect("sync-state json")
        }
    );

    assert_eq!(rt["syncState"], bs["syncState"]);
    assert_eq!(rt["lastPollAtMs"], bs["lastPollAtMs"]);
    assert_eq!(rt["broker"], bs["broker"]);

    handle.abort();
}
