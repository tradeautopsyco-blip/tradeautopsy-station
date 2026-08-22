//! Station Slice B Phase 1 — L3 DNS kill switch routes (#188).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, wait_ready, WireHeaderOverrides};
use serde_json::json;
use tradeautopsy_agent::{hosts_for_broker, plan_l3_dns, BLOCK_MARKER};

#[test]
fn broker_slug_maps_to_expected_hosts() {
    let zerodha = hosts_for_broker("zerodha");
    assert!(zerodha.contains(&"kite.zerodha.com"));

    let kotak = hosts_for_broker("kotak");
    assert!(kotak.contains(&"neo.kotaksecurities.com"));

    let kotak_neo = hosts_for_broker("kotak_neo");
    assert!(kotak_neo.contains(&"neo.kotaksecurities.com"));
    assert!(!kotak_neo.contains(&"kite.zerodha.com"));

    let com = hosts_for_broker("binance_com");
    assert!(com.contains(&"api.binance.com"));
    assert!(!com.iter().any(|h| h.contains("kotak")));
    assert!(hosts_for_broker("unknown_slug").is_empty());
}

#[test]
fn r8_empty_host_map_refuses_l3_block() {
    assert!(
        plan_l3_dns(true, &[]).is_err(),
        "website_block + empty map must refuse (R8)"
    );
    assert_eq!(
        plan_l3_dns(false, &[]).expect("block off is ok"),
        false,
        "website_block=false must skip DNS even when the map is empty"
    );
    assert_eq!(
        plan_l3_dns(true, &["kite.zerodha.com"]).expect("known hosts"),
        true
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn kill_switch_level_3_writes_hosts_marker_and_dismiss_clears() {
    const AGENT_PORT: u16 = 39_608;
    let dir = std::env::temp_dir();
    let hosts_path = dir.join(format!(
        "rta-killswitch-route-{}.hosts",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        hosts_path.to_string_lossy().to_string(),
    );

    let handle = spawn_test_agent(AGENT_PORT);
    wait_ready(AGENT_PORT).await;

    let fire_path = "/api/daemon/kill-switch";
    let fire_url = format!("http://127.0.0.1:{AGENT_PORT}{fire_path}");
    let fire_body = json!({ "level": 3, "broker": "zerodha" });
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
    let fire_json: serde_json::Value = fire_resp.json().await.expect("fire json");
    assert_eq!(fire_json["received"], true);
    assert_eq!(fire_json["level"], 3);
    assert_eq!(fire_json["dns_active"], true);

    let content = std::fs::read_to_string(&hosts_path).expect("read hosts");
    assert!(content.contains(BLOCK_MARKER));
    assert!(content.contains("kite.zerodha.com"));

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
    let dismiss_json: serde_json::Value = dismiss_resp.json().await.expect("dismiss json");
    assert_eq!(dismiss_json["ok"], true);
    assert_eq!(dismiss_json["dns_active"], false);

    let after = std::fs::read_to_string(&hosts_path).expect("read after dismiss");
    assert!(!after.contains(BLOCK_MARKER));

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn kill_switch_level_2_does_not_write_hosts_marker() {
    const AGENT_PORT: u16 = 39_609;
    let dir = std::env::temp_dir();
    let hosts_path = dir.join(format!("rta-killswitch-l2-{}.hosts", uuid::Uuid::new_v4()));
    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        hosts_path.to_string_lossy().to_string(),
    );

    let handle = spawn_test_agent(AGENT_PORT);
    wait_ready(AGENT_PORT).await;

    let path = "/api/daemon/kill-switch";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let body = json!({ "level": 2, "broker": "zerodha" });
    let body_bytes = serde_json::to_vec(&body).expect("json");
    let resp = apply_wire_v1(
        client().post(&url).json(&body),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("request");

    assert!(resp.status().is_success());
    let out: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(out["received"], true);
    assert_eq!(out["dns_active"], false);

    let content = std::fs::read_to_string(&hosts_path).expect("read hosts");
    assert!(!content.contains(BLOCK_MARKER));

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}
