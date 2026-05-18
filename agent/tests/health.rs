//! Tracer bullet (issue #57): agent exposes liveness on loopback with wire v1 (issue #58).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use std::time::Duration;

#[tokio::test]
async fn get_health_returns_ok_for_headless_agent() {
    const PORT: u16 = 19_337;
    let path = "/api/daemon/health";

    let handle = spawn_test_agent(PORT);

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
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("health request should reach agent");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["daemon"], "agent");
    assert!(
        body["build"].is_string(),
        "health should expose build identity string"
    );
    assert!(
        body["boot_id"].is_string(),
        "health should expose per-boot identity"
    );
    assert!(
        body["uptime_secs"].is_u64(),
        "health should expose uptime for liveness freshness"
    );
    assert!(
        body["sse_signing_pubkey_b64"].is_string(),
        "Phase 9 — health exposes pinned SSE signing pubkey"
    );
    assert!(
        body["observability"].is_object(),
        "Phase 9 — health exposes observability snapshot"
    );
    assert!(
        body["metrics_listen_port"].is_null(),
        "metrics_listen_port null when Prometheus disabled (tests)"
    );
    assert!(
        body["observability"]["agent_uptime_seconds"].is_number(),
        "observability includes agent_uptime_seconds gauge snapshot"
    );

    handle.abort();
}
