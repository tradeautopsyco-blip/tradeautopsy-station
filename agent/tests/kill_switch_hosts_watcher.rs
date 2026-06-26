//! Station Slice B Phase 5 — /etc/hosts watcher re-applies L3 block on tamper (#192).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use serde_json::json;
use std::time::Duration;
use tradeautopsy_agent::BLOCK_MARKER;

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn hosts_watcher_reapplies_block_after_manual_tamper() {
    const AGENT_PORT: u16 = 39_611;
    let dir = std::env::temp_dir();
    let hosts_path = dir.join(format!(
        "rta-killswitch-watcher-{}.hosts",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        hosts_path.to_string_lossy().to_string(),
    );

    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let fire_path = "/api/daemon/kill-switch";
    let fire_url = format!("http://127.0.0.1:{AGENT_PORT}{fire_path}");
    let fire_body = json!({ "level": 3, "broker": "zerodha", "reason": "watcher-test" });
    let fire_bytes = serde_json::to_vec(&fire_body).expect("json");
    let fire_resp = apply_wire_v1(
        client().post(&fire_url).json(&fire_body),
        "POST",
        fire_path,
        &fire_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("fire request");
    assert!(fire_resp.status().is_success());

    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("tamper hosts");
    assert!(!std::fs::read_to_string(&hosts_path)
        .expect("read tampered")
        .contains(BLOCK_MARKER));

    tokio::time::sleep(Duration::from_millis(900)).await;

    let restored = std::fs::read_to_string(&hosts_path).expect("read restored");
    assert!(restored.contains(BLOCK_MARKER));
    assert!(restored.contains("kite.zerodha.com"));

    let dismiss_path = "/api/daemon/dismiss-kill-switch";
    let dismiss_url = format!("http://127.0.0.1:{AGENT_PORT}{dismiss_path}");
    let dismiss_body = json!({});
    let dismiss_bytes = serde_json::to_vec(&dismiss_body).expect("json");
    let dismiss_resp = apply_wire_v1(
        client().post(&dismiss_url).json(&dismiss_body),
        "POST",
        dismiss_path,
        &dismiss_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("dismiss request");
    assert!(dismiss_resp.status().is_success());

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn kill_switch_without_broker_returns_400() {
    const AGENT_PORT: u16 = 39_612;
    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let fire_path = "/api/daemon/kill-switch";
    let fire_url = format!("http://127.0.0.1:{AGENT_PORT}{fire_path}");
    let fire_body = json!({ "level": 3, "reason": "missing-broker" });
    let fire_bytes = serde_json::to_vec(&fire_body).expect("json");
    let fire_resp = apply_wire_v1(
        client().post(&fire_url).json(&fire_body),
        "POST",
        fire_path,
        &fire_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("fire request");
    assert_eq!(fire_resp.status(), 400);

    handle.abort();
}
