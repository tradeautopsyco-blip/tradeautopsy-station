//! T5 K1 (#349) — apply rules, audit all levels, L1 no fog (TRD §6).

mod common;

use common::{
    apply_wire_v1, client, spawn_test_agent, spawn_test_agent_with_options, TestAgentOptions,
    WireHeaderOverrides,
};
use futures::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tradeautopsy_agent::{KillPolicy, BLOCK_MARKER};

async fn wait_ready(port: u16) {
    let path = "/api/daemon/health";
    let url = format!("http://127.0.0.1:{port}{path}");
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(80)).await;
        let Ok(resp) = apply_wire_v1(
            client().get(&url),
            "GET",
            path,
            b"",
            WireHeaderOverrides::default(),
        )
        .timeout(Duration::from_millis(400))
        .send()
        .await
        else {
            continue;
        };
        if resp.status().is_success() {
            return;
        }
    }
    panic!("agent on port {port} did not become ready");
}

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

fn seed_hosts(label: &str) -> std::path::PathBuf {
    let hosts_path =
        std::env::temp_dir().join(format!("rta-t5-k1-{label}-{}.hosts", uuid::Uuid::new_v4()));
    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        hosts_path.to_string_lossy().to_string(),
    );
    hosts_path
}

fn cleanup_hosts(hosts_path: &std::path::Path) {
    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
}

fn audit_hosts(entry: &Value) -> Vec<String> {
    let raw = entry["hosts_json"].as_str().unwrap_or("[]");
    serde_json::from_str(raw).unwrap_or_default()
}

/// Subscribe first, then fire, so the one-shot kill_switch_state is not missed.
async fn sse_buffer_until(port: u16, needle: &str, deadline: Duration) -> String {
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
        return String::new();
    };
    if resp.status() != 200 {
        return String::new();
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
                    break;
                }
            }
        }
    }
    acc
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l1_apply_sse_no_fog_audit_fire_no_hosts() {
    const PORT: u16 = 39_620;
    let hosts_path = seed_hosts("l1");
    let handle = spawn_test_agent(PORT);
    wait_ready(PORT).await;

    let sse = tokio::spawn(async move {
        sse_buffer_until(PORT, "\"level\":\"L1\"", Duration::from_secs(6)).await
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 1, "broker": "zerodha", "reason": "t5-l1" }),
    )
    .await;
    assert!(fire.status().is_success());
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(fire_json["received"], true);
    assert_eq!(fire_json["level"], 1);
    assert_eq!(fire_json["dns_active"], false);
    assert_eq!(
        fire_json["fog_active"], false,
        "Q8: L1 must not set fog_active"
    );

    let sse_buf = sse.await.expect("sse join");
    assert!(
        sse_buf.contains("\"level\":\"L1\""),
        "expected L1 SSE, got: {sse_buf}"
    );
    assert!(
        sse_buf.contains("\"countdown_secs\":null") || sse_buf.contains("\"countdown_secs\": null"),
        "L1 SSE has no countdown: {sse_buf}"
    );
    assert!(
        sse_buf.contains("\"requires_ack\":false") || sse_buf.contains("\"requires_ack\": false"),
        "L1 is not ack-gated: {sse_buf}"
    );

    let content = std::fs::read_to_string(&hosts_path).expect("hosts");
    assert!(!content.contains(BLOCK_MARKER), "L1 must not write hosts");

    let audit = get_kill_audit_tail(PORT, 5).await;
    assert_eq!(audit["ok"], true);
    let entries = audit["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 1, "L1 must append fire audit: {entries:?}");
    assert_eq!(entries[0]["event_type"], "fire");
    assert_eq!(entries[0]["level"], 1);
    assert_eq!(entries[0]["broker"], "zerodha");
    assert_eq!(entries[0]["verified"], true);
    assert!(
        audit_hosts(&entries[0]).is_empty(),
        "L1 fire hosts must be []"
    );

    cleanup_hosts(&hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l1_dismiss_audits_level_1_not_hardcoded_3() {
    const PORT: u16 = 39_621;
    let handle = spawn_test_agent(PORT);
    wait_ready(PORT).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 1, "broker": "zerodha", "reason": "t5-l1-dismiss" }),
    )
    .await;
    assert!(fire.status().is_success());

    let dismiss = post_json(PORT, "/api/daemon/dismiss-kill-switch", json!({})).await;
    assert!(dismiss.status().is_success());

    let audit = get_kill_audit_tail(PORT, 5).await;
    let entries = audit["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 2, "fire + dismiss: {entries:?}");
    assert_eq!(entries[0]["event_type"], "dismiss");
    assert_eq!(
        entries[0]["level"], 1,
        "dismiss must use last applied level, not hardcoded 3"
    );
    assert_eq!(entries[0]["broker"], "zerodha");
    assert_eq!(entries[0]["verified"], true);
    assert_eq!(entries[1]["event_type"], "fire");
    assert_eq!(entries[1]["level"], 1);

    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l2_apply_no_hosts_countdown_from_policy_audit_fire() {
    const PORT: u16 = 39_622;
    let hosts_path = seed_hosts("l2");
    let opts = TestAgentOptions {
        kill_policy: KillPolicy {
            default_level: 3,
            countdown_secs: 45,
            website_block: true,
        },
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    wait_ready(PORT).await;

    let sse = tokio::spawn(async move {
        sse_buffer_until(PORT, "\"level\":\"L2\"", Duration::from_secs(6)).await
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 2, "broker": "zerodha", "reason": "t5-l2" }),
    )
    .await;
    assert!(fire.status().is_success());
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(fire_json["received"], true);
    assert_eq!(fire_json["dns_active"], false);
    assert_eq!(fire_json["fog_active"], false);

    let sse_buf = sse.await.expect("sse join");
    assert!(
        sse_buf.contains("\"level\":\"L2\""),
        "expected L2 SSE, got: {sse_buf}"
    );
    assert!(
        sse_buf.contains("\"countdown_secs\":45"),
        "countdown must come from policy (45), not hardcoded 90: {sse_buf}"
    );
    assert!(
        sse_buf.contains("\"requires_ack\":false") || sse_buf.contains("\"requires_ack\": false"),
        "L2 is not ack-gated: {sse_buf}"
    );

    let content = std::fs::read_to_string(&hosts_path).expect("hosts");
    assert!(!content.contains(BLOCK_MARKER), "L2 must not write hosts");

    let audit = get_kill_audit_tail(PORT, 5).await;
    let entries = audit["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 1, "L2 must append fire audit: {entries:?}");
    assert_eq!(entries[0]["event_type"], "fire");
    assert_eq!(entries[0]["level"], 2);
    assert_eq!(entries[0]["verified"], true);
    assert!(audit_hosts(&entries[0]).is_empty());

    cleanup_hosts(&hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l2_dismiss_audits_level_2() {
    const PORT: u16 = 39_623;
    let handle = spawn_test_agent(PORT);
    wait_ready(PORT).await;

    assert!(post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 2, "broker": "zerodha", "reason": "t5-l2-dismiss" }),
    )
    .await
    .status()
    .is_success());

    assert!(
        post_json(PORT, "/api/daemon/dismiss-kill-switch", json!({}))
            .await
            .status()
            .is_success()
    );

    let entries = get_kill_audit_tail(PORT, 5).await["entries"]
        .as_array()
        .expect("entries")
        .clone();
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0]["event_type"], "dismiss");
    assert_eq!(entries[0]["level"], 2);
    assert_eq!(entries[1]["event_type"], "fire");
    assert_eq!(entries[1]["level"], 2);

    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l3_dismiss_clears_marker_and_audits_dismiss() {
    const PORT: u16 = 39_624;
    let hosts_path = seed_hosts("l3-dismiss");
    let handle = spawn_test_agent(PORT);
    wait_ready(PORT).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 3, "broker": "zerodha", "reason": "t5-l3-dismiss" }),
    )
    .await;
    assert!(fire.status().is_success());
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(fire_json["dns_active"], true);
    let hosts = std::fs::read_to_string(&hosts_path).expect("hosts");
    assert!(hosts.contains(BLOCK_MARKER));

    let dismiss = post_json(PORT, "/api/daemon/dismiss-kill-switch", json!({})).await;
    assert!(dismiss.status().is_success());
    let dismiss_json: Value = dismiss.json().await.expect("dismiss json");
    assert_eq!(dismiss_json["dns_active"], false);

    let after = std::fs::read_to_string(&hosts_path).expect("hosts after");
    assert!(!after.contains(BLOCK_MARKER), "dismiss must clear marker");

    let entries = get_kill_audit_tail(PORT, 5).await["entries"]
        .as_array()
        .expect("entries")
        .clone();
    assert!(
        entries
            .iter()
            .any(|e| e["event_type"] == "dismiss" && e["level"] == 3 && e["verified"] == true),
        "dismiss after L3 must audit level 3: {entries:?}"
    );

    cleanup_hosts(&hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l3_website_block_false_no_hosts_overlay_ack_audit_empty() {
    const PORT: u16 = 39_625;
    let hosts_path = seed_hosts("l3-noblock");
    let opts = TestAgentOptions {
        kill_policy: KillPolicy {
            default_level: 3,
            countdown_secs: 60,
            website_block: false,
        },
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    wait_ready(PORT).await;

    let sse = tokio::spawn(async move {
        sse_buffer_until(PORT, "\"level\":\"L3\"", Duration::from_secs(6)).await
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "level": 3, "broker": "zerodha", "reason": "t5-l3-noblock" }),
    )
    .await;
    assert!(fire.status().is_success());
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(fire_json["received"], true);
    assert_eq!(fire_json["dns_active"], false);

    let sse_buf = sse.await.expect("sse join");
    assert!(sse_buf.contains("\"level\":\"L3\""), "{sse_buf}");
    assert!(
        sse_buf.contains("\"countdown_secs\":60"),
        "L3 countdown from policy: {sse_buf}"
    );
    assert!(
        sse_buf.contains("\"requires_ack\":true") || sse_buf.contains("\"requires_ack\": true"),
        "L3 requires ack: {sse_buf}"
    );

    let content = std::fs::read_to_string(&hosts_path).expect("hosts");
    assert!(
        !content.contains(BLOCK_MARKER),
        "website_block=false must not write hosts"
    );

    let entries = get_kill_audit_tail(PORT, 5).await["entries"]
        .as_array()
        .expect("entries")
        .clone();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["event_type"], "fire");
    assert_eq!(entries[0]["level"], 3);
    assert!(
        audit_hosts(&entries[0]).is_empty(),
        "L3 without block audits hosts []"
    );

    cleanup_hosts(&hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn omitted_level_uses_policy_default_level() {
    const PORT: u16 = 39_626;
    let opts = TestAgentOptions {
        kill_policy: KillPolicy {
            default_level: 1,
            countdown_secs: 90,
            website_block: true,
        },
        ..TestAgentOptions::default()
    };
    let handle = spawn_test_agent_with_options(PORT, opts);
    wait_ready(PORT).await;

    let fire = post_json(
        PORT,
        "/api/daemon/kill-switch",
        json!({ "broker": "zerodha", "reason": "t5-default-level" }),
    )
    .await;
    assert!(fire.status().is_success());
    let fire_json: Value = fire.json().await.expect("fire json");
    assert_eq!(
        fire_json["level"], 1,
        "omitted/0 level must use policy default_level"
    );
    assert_eq!(fire_json["fog_active"], false);

    let entries = get_kill_audit_tail(PORT, 5).await["entries"]
        .as_array()
        .expect("entries")
        .clone();
    assert_eq!(entries[0]["event_type"], "fire");
    assert_eq!(entries[0]["level"], 1);

    handle.abort();
}
