//! Station Slice B Phase 4 — Ed25519 kill switch audit (#191).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use serde_json::json;
use std::time::Duration;

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l3_fire_and_dismiss_append_signed_audit_rows() {
    const AGENT_PORT: u16 = 39_610;
    let dir = std::env::temp_dir();
    let hosts_path = dir.join(format!(
        "rta-killswitch-audit-{}.hosts",
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
    let fire_body = json!({ "level": 3, "broker": "zerodha", "reason": "integration-test" });
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

    let audit_path = "/api/daemon/kill-switch/audit";
    let audit_url = format!("http://127.0.0.1:{AGENT_PORT}{audit_path}?tail=5");
    let audit_resp = apply_wire_v1(
        client().get(&audit_url),
        "GET",
        audit_path,
        &[],
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("audit request");
    assert!(audit_resp.status().is_success());
    let audit_json: serde_json::Value = audit_resp.json().await.expect("audit json");
    assert_eq!(audit_json["ok"], true);
    let entries = audit_json["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["event_type"], "fire");
    assert_eq!(entries[0]["level"], 3);
    assert_eq!(entries[0]["broker"], "zerodha");
    assert_eq!(entries[0]["verified"], true);

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

    let audit_after_path = "/api/daemon/kill-switch/audit";
    let audit_after_url = format!("http://127.0.0.1:{AGENT_PORT}{audit_after_path}?tail=5");
    let audit_after_resp = apply_wire_v1(
        client().get(&audit_after_url),
        "GET",
        audit_after_path,
        &[],
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("audit after dismiss");
    assert!(audit_after_resp.status().is_success());
    let audit_after: serde_json::Value = audit_after_resp.json().await.expect("audit json");
    let after_entries = audit_after["entries"].as_array().expect("entries");
    assert_eq!(after_entries.len(), 2);
    assert_eq!(after_entries[0]["event_type"], "dismiss");
    assert_eq!(after_entries[0]["verified"], true);
    assert_eq!(after_entries[1]["event_type"], "fire");
    assert_eq!(after_entries[1]["verified"], true);

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}
