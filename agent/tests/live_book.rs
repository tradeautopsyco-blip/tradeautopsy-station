//! LiveBook — agent live-state is Station read authority after one snapshot.

mod common;

use axum::routing::get;
use axum::{Json, Router};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::MemoryStationTokenStore;

/// Planted snapshot — literals the agent must serve after LiveBook is hydrated.
const PLANTED_SCHEMA_VERSION: i64 = 1;
const PLANTED_PLAN_STATE: &str = "GREEN";
const PLANTED_SYNC_STATE: &str = "GREEN";

fn planted_snapshot() -> Value {
    json!({
        "schemaVersion": 1,
        "notch": { "plan_state": "GREEN", "sync_state": "GREEN" }
    })
}

#[tokio::test]
async fn live_state_serves_planted_livebook_without_second_upstream() {
    const AGENT_PORT: u16 = 39_611;
    let upstream_hits = Arc::new(AtomicU64::new(0));
    let hits = Arc::clone(&upstream_hits);
    let upstream = Router::new().route(
        "/api/bar/v1/live-state",
        get(move || {
            let hits = Arc::clone(&hits);
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                Json(json!({ "schemaVersion": 99, "leaked": true }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind upstream");
    let upstream_port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, upstream)
            .await
            .expect("upstream serve");
    });
    tokio::time::sleep(Duration::from_millis(60)).await;

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{upstream_port}"));
    opts.live_book_snapshot = Some(planted_snapshot());
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/live-state";
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
    .expect("agent");

    assert_eq!(resp.status(), 200);
    let livebook = resp
        .headers()
        .get("x-livebook")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    assert_eq!(livebook.as_deref(), Some("local"));
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["schemaVersion"], PLANTED_SCHEMA_VERSION);
    assert_eq!(body["notch"]["plan_state"], PLANTED_PLAN_STATE);
    assert_eq!(body["notch"]["sync_state"], PLANTED_SYNC_STATE);
    assert_eq!(
        upstream_hits.load(Ordering::SeqCst),
        0,
        "hydrated LiveBook must not GET hosted /api/bar/v1/live-state"
    );

    handle.abort();
}

#[tokio::test]
async fn live_state_snapshots_once_then_serves_local() {
    const AGENT_PORT: u16 = 39_612;
    let upstream_hits = Arc::new(AtomicU64::new(0));
    let hits = Arc::clone(&upstream_hits);
    let upstream = Router::new().route(
        "/api/bar/v1/live-state",
        get(move || {
            let hits = Arc::clone(&hits);
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                Json(planted_snapshot())
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind upstream");
    let upstream_port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, upstream)
            .await
            .expect("upstream serve");
    });
    tokio::time::sleep(Duration::from_millis(60)).await;

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{upstream_port}"));
    opts.station_token_store = Some(Arc::new(MemoryStationTokenStore::default()));
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/live-state";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");

    let first = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("first agent get");

    assert_eq!(first.status(), 200);
    let first_header = first
        .headers()
        .get("x-livebook")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    assert_eq!(first_header.as_deref(), Some("snapshot"));
    let first_body: Value = first.json().await.expect("json");
    assert_eq!(first_body["schemaVersion"], PLANTED_SCHEMA_VERSION);
    assert_eq!(first_body["notch"]["plan_state"], PLANTED_PLAN_STATE);
    assert_eq!(first_body["notch"]["sync_state"], PLANTED_SYNC_STATE);
    assert_eq!(upstream_hits.load(Ordering::SeqCst), 1);

    let second = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("second agent get");

    assert_eq!(second.status(), 200);
    let second_header = second
        .headers()
        .get("x-livebook")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    assert_eq!(second_header.as_deref(), Some("local"));
    let second_body: Value = second.json().await.expect("json");
    assert_eq!(second_body["schemaVersion"], PLANTED_SCHEMA_VERSION);
    assert_eq!(second_body["notch"]["plan_state"], PLANTED_PLAN_STATE);
    assert_eq!(second_body["notch"]["sync_state"], PLANTED_SYNC_STATE);
    assert_eq!(
        upstream_hits.load(Ordering::SeqCst),
        1,
        "following GET must not hit hosted /api/bar/v1/live-state again"
    );

    handle.abort();
}
