//! Phase 9 (#66) — bar `x-request-id` forwarded through agent proxy paths (design §11.1).

mod common;

use axum::{routing::post, Json, Router};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn kill_switch_proxy_forwards_bar_request_id_to_upstream() {
    const AGENT_PORT: u16 = 19_450;
    let captured = Arc::new(Mutex::new(None::<String>));
    let cap = captured.clone();

    let upstream_app = Router::new().route(
        "/api/daemon/kill-switch/ack",
        post(move |headers: axum::http::HeaderMap| {
            let cap = cap.clone();
            async move {
                let rid = headers
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                *cap.lock().expect("mutex") = rid;
                Json(json!({ "ok": true }))
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind upstream");
    let upstream_port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, upstream_app)
            .await
            .expect("upstream serve");
    });

    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{upstream_port}"));
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let fixed_rid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let path = "/api/daemon/kill-switch/ack";
    let agent_url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let body = b"{}";
    let resp = apply_wire_v1(
        client()
            .post(&agent_url)
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_vec()),
        "POST",
        path,
        body,
        WireHeaderOverrides {
            request_id: Some(fixed_rid),
            ..WireHeaderOverrides::default()
        },
    )
    .send()
    .await
    .expect("proxy");

    assert!(
        resp.status().is_success(),
        "unexpected agent status {}",
        resp.status()
    );

    tokio::time::sleep(Duration::from_millis(120)).await;

    let seen = captured.lock().expect("mutex").clone();
    assert_eq!(
        seen.as_deref(),
        Some(fixed_rid),
        "upstream must observe bar x-request-id"
    );

    handle.abort();
}
