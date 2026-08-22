//! T5 K4 (#351) — KillPolicy sqlite drives countdown + optional L3 block (TRD §6 items 2, 3, 4, 8).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides, TEST_SECRET};
use futures::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tradeautopsy_agent::{AgentConfig, KillPolicy, KillPolicyStore, BLOCK_MARKER};

fn policy_store_for_port(port: u16) -> KillPolicyStore {
    let path = AgentConfig::test_on_port(port, TEST_SECRET).kill_switch_audit_db_path;
    KillPolicyStore::open(&path).expect("open policy beside audit")
}

fn extract_sse_data_json(chunk: &str) -> Option<Value> {
    for line in chunk.lines() {
        if let Some(raw) = line.strip_prefix("data:") {
            let candidate = raw.trim();
            if let Ok(json) = serde_json::from_str::<Value>(candidate) {
                return Some(json);
            }
        }
    }
    None
}

async fn fire_and_read_kill_switch_state(port: u16, body: Value) -> (Value, Value) {
    let path = "/api/daemon/events/stream";
    let url = format!("http://127.0.0.1:{port}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("sse connect");
    assert_eq!(resp.status(), 200);
    let mut stream = resp.bytes_stream();

    let fire = fire_kill_switch(port, body).await;

    let deadline = tokio::time::sleep(Duration::from_secs(8));
    tokio::pin!(deadline);
    let payload = loop {
        tokio::select! {
            _ = &mut deadline => panic!("timed out waiting for kill_switch_state"),
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { panic!("sse ended") };
                let text = String::from_utf8_lossy(&chunk);
                if let Some(ev) = extract_sse_data_json(&text) {
                    if ev["type"] == "kill_switch_state" {
                        break ev["payload"].clone();
                    }
                }
            }
        }
    };
    (fire, payload)
}

async fn fire_kill_switch(port: u16, body: Value) -> Value {
    let path = "/api/daemon/kill-switch";
    let url = format!("http://127.0.0.1:{port}{path}");
    let bytes = serde_json::to_vec(&body).expect("json");
    let resp = apply_wire_v1(
        client().post(&url).json(&body),
        "POST",
        path,
        &bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("fire");
    assert!(resp.status().is_success(), "fire status {}", resp.status());
    resp.json().await.expect("fire json")
}

async fn audit_entries(port: u16) -> Vec<Value> {
    let path = "/api/daemon/kill-switch/audit";
    let url = format!("http://127.0.0.1:{port}{path}?tail=5");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        &[],
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("audit");
    assert!(resp.status().is_success());
    let json: Value = resp.json().await.expect("audit json");
    json["entries"].as_array().expect("entries").clone()
}

fn seed_hosts_file(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rta-kill-policy-{label}-{}.hosts",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        path.to_string_lossy().to_string(),
    );
    path
}

#[tokio::test]
#[serial_test::serial]
async fn policy_default_row_inserted_on_boot() {
    const AGENT_PORT: u16 = 39_620;
    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let store = policy_store_for_port(AGENT_PORT);
    let policy = store.load_or_insert_defaults().expect("load after boot");
    assert_eq!(
        policy,
        KillPolicy {
            default_level: 3,
            countdown_secs: 90,
            website_block: true,
        }
    );

    handle.abort();
}

#[tokio::test]
#[serial_test::serial]
async fn l2_countdown_uses_policy_when_not_90() {
    const AGENT_PORT: u16 = 39_621;
    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let store = policy_store_for_port(AGENT_PORT);
    store
        .save(&KillPolicy {
            default_level: 3,
            countdown_secs: 45,
            website_block: true,
        })
        .expect("save countdown 45");

    let (fire, payload) =
        fire_and_read_kill_switch_state(AGENT_PORT, json!({ "level": 2, "broker": "zerodha" }))
            .await;
    assert_eq!(fire["received"], true);
    assert_eq!(payload["active"], true);
    assert_eq!(payload["level"], "L2");
    assert_eq!(
        payload["countdown_secs"], 45,
        "countdown must come from policy, not magic 90"
    );
    assert_eq!(payload["requires_ack"], false);

    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l3_website_block_on_writes_hosts() {
    const AGENT_PORT: u16 = 39_622;
    let hosts_path = seed_hosts_file("block-on");
    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let policy = policy_store_for_port(AGENT_PORT)
        .load_or_insert_defaults()
        .expect("defaults");
    assert!(policy.website_block);

    let fire = fire_kill_switch(
        AGENT_PORT,
        json!({ "level": 3, "broker": "zerodha", "reason": "policy-block-on" }),
    )
    .await;
    assert_eq!(fire["received"], true);
    assert_eq!(fire["dns_active"], true);

    let content = std::fs::read_to_string(&hosts_path).expect("read hosts");
    assert!(content.contains(BLOCK_MARKER));
    assert!(content.contains("kite.zerodha.com"));

    let entries = audit_entries(AGENT_PORT).await;
    assert_eq!(entries[0]["event_type"], "fire");
    let hosts: Vec<String> =
        serde_json::from_str(entries[0]["hosts_json"].as_str().expect("hosts_json"))
            .expect("parse hosts");
    assert!(!hosts.is_empty());

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn l3_website_block_off_no_hosts_requires_ack_empty_audit() {
    const AGENT_PORT: u16 = 39_623;
    let hosts_path = seed_hosts_file("block-off");
    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    policy_store_for_port(AGENT_PORT)
        .save(&KillPolicy {
            default_level: 3,
            countdown_secs: 90,
            website_block: false,
        })
        .expect("save block off");

    let (fire, payload) = fire_and_read_kill_switch_state(
        AGENT_PORT,
        json!({ "level": 3, "broker": "zerodha", "reason": "policy-block-off" }),
    )
    .await;
    assert_eq!(fire["received"], true);
    assert_eq!(fire["dns_active"], false);

    let content = std::fs::read_to_string(&hosts_path).expect("read hosts");
    assert!(!content.contains(BLOCK_MARKER));
    assert_eq!(payload["active"], true);
    assert_eq!(payload["level"], "L3");
    assert_eq!(payload["requires_ack"], true);
    assert_eq!(payload["countdown_secs"], 90);

    let entries = audit_entries(AGENT_PORT).await;
    assert_eq!(entries[0]["event_type"], "fire");
    assert_eq!(entries[0]["hosts_json"], "[]");

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}

#[tokio::test]
#[serial_test::serial]
async fn omitted_level_uses_policy_default_level() {
    const AGENT_PORT: u16 = 39_624;
    let handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    policy_store_for_port(AGENT_PORT)
        .save(&KillPolicy {
            default_level: 2,
            countdown_secs: 30,
            website_block: false,
        })
        .expect("save default L2");

    let (fire, payload) =
        fire_and_read_kill_switch_state(AGENT_PORT, json!({ "broker": "zerodha" })).await;
    assert_eq!(fire["received"], true);
    assert_eq!(
        fire["level"], 2,
        "omitted body level must use policy default_level"
    );
    assert_eq!(payload["level"], "L2");
    assert_eq!(payload["countdown_secs"], 30);

    handle.abort();
}
