//! Station Slice B Phase 3 — brain `fog_of_war` command poll executes L3 (#190).

mod common;

use axum::{routing::get, Json, Router};
use common::{spawn_test_agent_with_options, TestAgentOptions, TEST_USER_ID};
use serde_json::json;
use std::time::Duration;
use tradeautopsy_agent::BLOCK_MARKER;

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn daemon_command_poll_fog_of_war_level_3_writes_hosts_marker() {
    const AGENT_PORT: u16 = 39_610;
    let dir = std::env::temp_dir();
    let hosts_path = dir.join(format!("rta-daemon-cmd-{}.hosts", uuid::Uuid::new_v4()));
    std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
    std::env::set_var(
        "TRADEAUTOPSY_HOSTS_FILE",
        hosts_path.to_string_lossy().to_string(),
    );
    std::env::set_var("AGENT_COMMAND_POLL_MS", "250");

    let upstream = Router::new().route(
        "/api/daemon/command",
        get(|| async {
            Json(json!({
                "commands": [{
                    "id": "cmd-test-1",
                    "type": "fog_of_war",
                    "payload": { "level": 3, "trigger": "jest_poll", "broker": "zerodha" },
                    "created_at": "2026-01-01T00:00:00Z",
                }],
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind upstream");
    let upstream_port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, upstream)
            .await
            .expect("upstream serve");
    });

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{upstream_port}"));
    opts.daemon_poll_user_id = Some(TEST_USER_ID.to_string());
    opts.command_poll_ms = 250;

    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(350)).await;
    tokio::time::sleep(Duration::from_millis(1200)).await;

    let content = std::fs::read_to_string(&hosts_path).expect("read hosts");
    assert!(content.contains(BLOCK_MARKER));
    assert!(content.contains("kite.zerodha.com"));

    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
    std::env::remove_var("AGENT_COMMAND_POLL_MS");
    let _ = std::fs::remove_file(hosts_path);
    handle.abort();
}

#[test]
fn daemon_command_parse_unit() {
    use tradeautopsy_agent::{parse_daemon_command_type, DaemonCommandKind};
    assert_eq!(
        parse_daemon_command_type("fog_of_war"),
        Some(DaemonCommandKind::FogOfWar)
    );
    assert_eq!(parse_daemon_command_type("nope"), None);
}
