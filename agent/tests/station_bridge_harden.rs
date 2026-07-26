//! T1 — Station ↔ Enforcer bridge harden regression coverage.
//!
//! Dual-hop auth (A8 IV / station-wire freeze) hardened by T1:
//! - Loopback (Notch/StationApp → agent): Wire v1 HMAC + `x-daemon-secret` are
//!   machine-integrity-only; `x-user-id` is a fixed wire hint, never Console identity.
//! - Upstream (agent → Console): `Authorization: Bearer` Station Caller JWT from
//!   Keychain only — the agent must never forward `x-daemon-secret` / `x-user-id`
//!   as who-am-I, on *any* upstream hop (not just the bar-declare hop already
//!   covered by `bar_forward.rs`).
//!
//! This file exercises two additional upstream hops that were not yet covered by
//! header-hygiene assertions: the capture-accept → outbox → upstream delivery path,
//! and the `/instruments/ltp` → upstream broker-ltp proxy path.

mod common;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use common::{apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides};
use serde_json::{json, Value};
use serial_test::serial;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
struct HeaderSpy {
    headers: Arc<Mutex<Option<HeaderMap>>>,
}

fn assert_bridge_hardened_headers(headers: &HeaderMap) {
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        auth.starts_with("Bearer "),
        "upstream request must carry Station Caller Bearer, got {auth:?}"
    );
    assert!(
        headers.get("x-daemon-secret").is_none(),
        "upstream request must never forward the loopback wire secret"
    );
    assert!(
        headers.get("x-user-id").is_none(),
        "upstream request must never forward the loopback wire hint as Console identity"
    );
}

async fn upstream_capture_accept(
    State(spy): State<HeaderSpy>,
    headers: HeaderMap,
    Json(_body): Json<Value>,
) -> impl IntoResponse {
    *spy.headers.lock().expect("headers mutex") = Some(headers);
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": { "pending_capture_id": "11111111-2222-4333-8444-555555555555", "status": "accepted" }
        })),
    )
}

#[tokio::test]
#[serial]
async fn capture_accept_outbox_delivery_never_forwards_daemon_identity_upstream() {
    const AGENT_PORT: u16 = 39_701;
    const UPSTREAM_PORT: u16 = 39_702;

    let spy = HeaderSpy::default();
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_capture_accept),
        )
        .with_state(spy.clone());
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], UPSTREAM_PORT)))
        .await
        .expect("bind upstream");
    let upstream_handle = tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve upstream");
    });

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{UPSTREAM_PORT}"));
    let agent_handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/journal/toolbar-capture/accept";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let body = json!({
        "draftText": "T1 bridge harden capture",
        "tradeId": "11111111-2222-4333-8444-555555555555",
        "explicitPending": false,
        "idempotencyKey": "t1-bridge-harden-1",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("json");

    let resp = apply_wire_v1(
        client().post(&url).body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .header("content-type", "application/json")
    .send()
    .await
    .expect("agent request");

    assert_eq!(resp.status(), StatusCode::OK);

    let headers = spy
        .headers
        .lock()
        .expect("headers mutex")
        .clone()
        .expect("upstream should have been called");
    assert_bridge_hardened_headers(&headers);

    agent_handle.abort();
    upstream_handle.abort();
}

async fn upstream_broker_ltp(
    State(spy): State<HeaderSpy>,
    headers: HeaderMap,
    Json(_body): Json<Value>,
) -> impl IntoResponse {
    *spy.headers.lock().expect("headers mutex") = Some(headers);
    (
        StatusCode::OK,
        Json(json!({ "ltp": 1234.5 })),
    )
}

#[tokio::test]
#[serial]
async fn instruments_ltp_proxy_never_forwards_daemon_identity_upstream() {
    const AGENT_PORT: u16 = 39_703;
    const UPSTREAM_PORT: u16 = 39_704;

    let spy = HeaderSpy::default();
    let router = Router::new()
        .route("/api/bar/v1/broker/ltp", post(upstream_broker_ltp))
        .with_state(spy.clone());
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], UPSTREAM_PORT)))
        .await
        .expect("bind upstream");
    let upstream_handle = tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve upstream");
    });

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{UPSTREAM_PORT}"));
    let agent_handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    // `/instruments/*` is not behind wire verification — `x-user-id` here is only the
    // same loopback wire hint UUID a real Notch/StationApp client would send, never a
    // Console identity. The proxy must still authorize upstream via Bearer only.
    let path = "/instruments/ltp?symbol=RELIANCE&exchange=NSE&segment=NSE";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let resp = client()
        .get(&url)
        .header("x-user-id", common::TEST_USER_ID)
        .send()
        .await
        .expect("agent request");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["ltp"], json!(1234.5));

    let headers = spy
        .headers
        .lock()
        .expect("headers mutex")
        .clone()
        .expect("upstream should have been called");
    assert_bridge_hardened_headers(&headers);

    agent_handle.abort();
    upstream_handle.abort();
}
