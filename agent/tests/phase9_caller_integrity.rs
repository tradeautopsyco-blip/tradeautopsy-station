//! Phase 9 (#66) — caller integrity gate (design §10.1 T2).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use serde_json::Value;
use serial_test::serial;
use std::time::Duration;

#[tokio::test]
#[serial]
async fn caller_strict_rejects_missing_bundle_then_accepts_pinned_bundle() {
    const PORT: u16 = 19_449;
    let path = "/api/daemon/health";

    std::env::set_var("AGENT_CALLER_STRICT", "1");
    std::env::set_var(
        "AGENT_EXPECTED_CALLER_BUNDLE_ID",
        "com.tradeautopsy.bar.test",
    );

    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let bad = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("reach agent");

    assert_eq!(bad.status(), reqwest::StatusCode::UNAUTHORIZED);
    let env: Value = bad.json().await.expect("json");
    assert_eq!(env["error_class"], "SIG_INVALID");

    let ok = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .header(
        "x-tradeautopsy-caller-bundle-id",
        "com.tradeautopsy.bar.test",
    )
    .send()
    .await
    .expect("reach agent");

    assert_eq!(ok.status(), reqwest::StatusCode::OK);

    handle.abort();
    std::env::remove_var("AGENT_CALLER_STRICT");
    std::env::remove_var("AGENT_EXPECTED_CALLER_BUNDLE_ID");
}
