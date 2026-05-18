//! Phase 9 — wire rejection observability (design §11.2 `hmac_verify_failures_total`).

mod common;

use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use std::time::Duration;

#[tokio::test]
async fn prometheus_hmac_failure_counter_increments_on_sig_invalid() {
    const PORT: u16 = 19_455;
    const METRICS_PORT: u16 = 19_456;

    let mut opts = TestAgentOptions::default();
    opts.metrics_port = Some(METRICS_PORT);
    let handle = spawn_test_agent_with_options(PORT, opts);
    tokio::time::sleep(Duration::from_millis(260)).await;

    let path = "/api/daemon/health";
    let url = format!("http://127.0.0.1:{PORT}{path}");
    let bad = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides {
            signature_b64: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
            ..WireHeaderOverrides::default()
        },
    )
    .send()
    .await
    .expect("req");
    assert_eq!(bad.status(), reqwest::StatusCode::UNAUTHORIZED);

    tokio::time::sleep(Duration::from_millis(120)).await;

    let metrics_url = format!("http://127.0.0.1:{METRICS_PORT}/metrics");
    let txt = client()
        .get(&metrics_url)
        .send()
        .await
        .expect("metrics")
        .text()
        .await
        .expect("body");

    let mut failures = 0u64;
    for line in txt.lines() {
        if line.starts_with("hmac_verify_failures_total ") {
            failures = line.rsplit(' ').next().unwrap_or("0").parse().unwrap_or(0);
            break;
        }
    }
    assert!(
        failures >= 1,
        "expected at least one counted wire auth failure, got {failures} in:\n{txt}"
    );

    handle.abort();
}
