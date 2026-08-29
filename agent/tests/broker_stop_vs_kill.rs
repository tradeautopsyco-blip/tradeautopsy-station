//! T2.3 / B3 — Stop ≠ Kill (R10).
//!
//! Stop pauses broker poll only. Kill (DNS L3 + audit + SSE) stays independent:
//! after Stop, L3 still arms, audit still appends, SSE still publishes, and
//! dismiss does **not** resume sync.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    wait_ready, TestAgentOptions, WireHeaderOverrides,
};
use futures::StreamExt;
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, BLOCK_MARKER};

async fn post_json(port: u16, path: &str, body: Value) -> reqwest::Response {
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
    .timeout(Duration::from_secs(5))
    .send()
    .await
    .expect("post")
}

async fn get_json(port: u16, path: &str) -> Value {
    let url = format!("http://127.0.0.1:{port}{path}");
    apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(5))
    .send()
    .await
    .expect("get")
    .json()
    .await
    .expect("json")
}

/// Wire-v1 signs the path without the query string (same as other kill audit tests).
async fn get_kill_audit_tail(port: u16, tail: u32) -> Value {
    let path = "/api/daemon/kill-switch/audit";
    let url = format!("http://127.0.0.1:{port}{path}?tail={tail}");
    apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(5))
    .send()
    .await
    .expect("audit get")
    .json()
    .await
    .expect("audit json")
}

async fn get_health_boot_id(port: u16) -> String {
    get_json(port, "/api/daemon/health").await["boot_id"]
        .as_str()
        .expect("boot_id")
        .to_string()
}

/// Wait until SSE stream contains `needle` (or deadline).
async fn sse_sees(port: u16, needle: &str, deadline: Duration) -> bool {
    let path = "/api/daemon/events/stream";
    let url = format!("http://127.0.0.1:{port}{path}");
    let Ok(resp) = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(30))
    .send()
    .await
    else {
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

#[tokio::test]
#[serial]
async fn b3_stop_then_l1_kill_still_publishes_sse_sync_stays_paused() {
    const PORT: u16 = 19_476;
    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    let opts = TestAgentOptions {
        runtime_poll_adapter: Some(counter.clone() as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 60,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    wait_ready(PORT).await;

    assert_eq!(
        post_json(
            PORT,
            "/api/daemon/broker/sync/start",
            identity_start_body("binance_us")
        )
        .await
        .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(counter.poll_count() >= 2, "need active polling before stop");

    let boot_before = get_health_boot_id(PORT).await;
    assert_eq!(
        post_json(PORT, "/api/daemon/broker/sync/stop", json!({}))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(80)).await;
    let polls_at_stop = counter.poll_count();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(counter.poll_count(), polls_at_stop, "Stop must freeze poll");
    assert_eq!(
        get_json(PORT, "/api/daemon/broker/sync-state").await["runtimeStatus"],
        "paused"
    );

    // Subscribe before fire so we catch kill_switch_state (B3: SSE stays live after Stop).
    let sse =
        tokio::spawn(
            async move { sse_sees(PORT, "kill_switch_state", Duration::from_secs(6)).await },
        );
    tokio::time::sleep(Duration::from_millis(150)).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 1, "broker": "binance_com", "reason": "b3-stop-vs-kill" }),
    )
    .await;
    assert!(fire.status().is_success(), "Kill must work after Stop");
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(fire_json["received"], true);
    assert_eq!(fire_json["level"], 1);

    assert!(
        sse.await.expect("sse join"),
        "Kill SSE must still publish after Stop"
    );

    // Stop is not undone by Kill.
    assert_eq!(
        get_json(PORT, "/api/daemon/broker/sync-state").await["runtimeStatus"],
        "paused"
    );
    assert_eq!(
        counter.poll_count(),
        polls_at_stop,
        "Kill must not resume poll"
    );
    assert_eq!(get_health_boot_id(PORT).await, boot_before);

    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial]
async fn b3_stop_then_l3_dns_audit_dismiss_leaves_sync_paused() {
    const PORT: u16 = 19_477;
    let hosts_path =
        std::env::temp_dir().join(format!("rta-b3-stop-kill-{}.hosts", uuid::Uuid::new_v4()));
    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        hosts_path.to_string_lossy().to_string(),
    );

    let vault = seeded_hmac_vault("binance_us", "TA_TEST_SYNC");
    let counter = Arc::new(CountingPollAdapter::new());
    let opts = TestAgentOptions {
        runtime_poll_adapter: Some(counter.clone() as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 60,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    wait_ready(PORT).await;

    assert_eq!(
        post_json(
            PORT,
            "/api/daemon/broker/sync/start",
            identity_start_body("binance_us")
        )
        .await
        .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(counter.poll_count() >= 2);

    let boot_before = get_health_boot_id(PORT).await;
    assert_eq!(
        post_json(PORT, "/api/daemon/broker/sync/stop", json!({}))
            .await
            .status(),
        200
    );
    tokio::time::sleep(Duration::from_millis(80)).await;
    let polls_at_stop = counter.poll_count();
    assert_eq!(
        get_json(PORT, "/api/daemon/broker/sync-state").await["runtimeStatus"],
        "paused"
    );

    // L3 Kill while paused — COM hosts (R8) + audit (R10 combined scenario).
    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({
            "level": 3,
            "broker": "binance_com",
            "reason": "b3-combined-invariant"
        }),
    )
    .await;
    assert!(fire.status().is_success());
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(fire_json["received"], true);
    assert_eq!(fire_json["dns_active"], true);

    let hosts = std::fs::read_to_string(&hosts_path).expect("hosts");
    assert!(
        hosts.contains(BLOCK_MARKER),
        "L3 must write marker after Stop"
    );
    assert!(
        hosts.contains("api.binance.com"),
        "COM Kill hosts must apply after Stop"
    );

    let audit = get_kill_audit_tail(PORT, 5).await;
    assert_eq!(audit["ok"], true);
    let entries = audit["entries"].as_array().expect("entries");
    assert!(
        entries
            .iter()
            .any(|e| e["event_type"] == "fire" && e["broker"] == "binance_com"),
        "audit fire row required: {entries:?}"
    );

    // Sync still paused; poll still frozen.
    assert_eq!(
        get_json(PORT, "/api/daemon/broker/sync-state").await["runtimeStatus"],
        "paused"
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(counter.poll_count(), polls_at_stop);

    let dismiss = post_json(PORT, "/api/daemon/dismiss-kill-switch", json!({})).await;
    assert!(dismiss.status().is_success());
    let dismiss_json: Value = dismiss.json().await.expect("dismiss json");
    assert_eq!(dismiss_json["dns_active"], false);

    let after_hosts = std::fs::read_to_string(&hosts_path).expect("hosts after");
    assert!(!after_hosts.contains(BLOCK_MARKER));

    let audit_after = get_kill_audit_tail(PORT, 5).await;
    let after_entries = audit_after["entries"].as_array().expect("entries");
    assert!(after_entries.iter().any(|e| e["event_type"] == "dismiss"));

    // Dismiss Kill ≠ Start sync.
    assert_eq!(
        get_json(PORT, "/api/daemon/broker/sync-state").await["runtimeStatus"],
        "paused"
    );
    assert_eq!(counter.poll_count(), polls_at_stop);
    assert_eq!(get_health_boot_id(PORT).await, boot_before);

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}
