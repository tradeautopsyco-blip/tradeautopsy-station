//! Phase 9 (#66) — loopback Prometheus exposition for QA (design §11.2).

mod common;

use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use futures::StreamExt;
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn prometheus_metrics_reports_sse_health_counter() {
    const PORT: u16 = 19_453;
    const METRICS_PORT: u16 = 19_454;
    std::env::set_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS", "90");

    let mut opts = TestAgentOptions::default();
    opts.metrics_port = Some(METRICS_PORT);
    let handle = spawn_test_agent_with_options(PORT, opts);

    tokio::time::sleep(Duration::from_millis(260)).await;

    let path_health = "/api/daemon/health";
    let health_url = format!("http://127.0.0.1:{PORT}{path_health}");
    let health_resp = apply_wire_v1(
        client().get(&health_url),
        "GET",
        path_health,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("health");
    assert_eq!(health_resp.status(), 200);
    let hj: serde_json::Value = health_resp.json().await.expect("health json");
    assert_eq!(
        hj["metrics_listen_port"],
        json!(METRICS_PORT),
        "health should advertise Prometheus loopback port for bar diagnostics"
    );

    let path_sse = "/api/daemon/events/stream";
    let sse_url = format!("http://127.0.0.1:{PORT}{path_sse}");
    let req = apply_wire_v1(
        client().get(&sse_url),
        "GET",
        path_sse,
        b"",
        WireHeaderOverrides::default(),
    );
    let resp = req
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .expect("sse");
    assert_eq!(resp.status(), 200);

    let mut stream = resp.bytes_stream();
    let _ = stream.next().await;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let metrics_url = format!("http://127.0.0.1:{METRICS_PORT}/metrics");
    let txt = client()
        .get(&metrics_url)
        .send()
        .await
        .expect("metrics")
        .text()
        .await
        .expect("metrics body");

    let mut saw_nonzero = false;
    let mut saw_uptime_line = false;
    for line in txt.lines() {
        if line.starts_with("sse_events_published_total{type=\"agent_health\"}") {
            let n: u64 = line.rsplit(' ').next().unwrap_or("0").parse().unwrap_or(0);
            saw_nonzero |= n >= 1;
        }
        if line.starts_with("agent_uptime_seconds ") {
            saw_uptime_line = true;
        }
    }
    assert!(
        saw_nonzero,
        "expected sse_events_published_total for agent_health >= 1 in:\n{txt}"
    );
    assert!(
        saw_uptime_line,
        "expected agent_uptime_seconds gauge line in:\n{txt}"
    );

    handle.abort();
    std::env::remove_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS");
}
