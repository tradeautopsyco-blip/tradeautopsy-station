//! Wave 7 — GET `/api/daemon/morning-brief`.

mod common;

use common::{apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides};
use serde_json::Value;
use std::time::Duration;

#[tokio::test]
async fn morning_brief_returns_contract_envelope() {
    const AGENT_PORT: u16 = 39_722;
    let handle = spawn_test_agent_with_options(AGENT_PORT, TestAgentOptions::default());
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/morning-brief";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("get");
    assert_eq!(resp.status(), 200);
    let out: Value = resp.json().await.expect("json");
    assert_eq!(out["briefing_contract_version"], 1);
    assert!(out.get("briefing").is_some());
    assert_eq!(out["briefing"]["patterns"].as_array().map(|a| a.len()), Some(0));

    handle.abort();
}
