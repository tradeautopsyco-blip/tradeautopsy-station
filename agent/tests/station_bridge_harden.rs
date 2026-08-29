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
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
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
    (StatusCode::OK, Json(json!({ "ltp": 1234.5 })))
}

async fn assert_wire_401_sig_invalid(resp: reqwest::Response) {
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let v: Value = resp.json().await.expect("json error body");
    assert_eq!(v["error_class"].as_str(), Some("SIG_INVALID"));
}

/// Slice A — `/instruments/ltp` must reject unsigned callers (wire-v1 HMAC only).
#[tokio::test]
#[serial]
async fn unsigned_instruments_ltp_returns_401_sig_invalid() {
    const AGENT_PORT: u16 = 39_705;

    let agent_handle = spawn_test_agent_with_options(AGENT_PORT, TestAgentOptions::default());
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/instruments/ltp?symbol=RELIANCE&exchange=NSE&segment=NSE";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    // Proto present but no HMAC / daemon secret — unsigned caller.
    let resp = client()
        .get(&url)
        .header("x-proto-version", "1")
        .send()
        .await
        .expect("agent request");

    assert_wire_401_sig_invalid(resp).await;

    agent_handle.abort();
}

/// Slice A — `/instruments/search` must reject unsigned callers (wire-v1 HMAC only).
#[tokio::test]
#[serial]
async fn unsigned_instruments_search_returns_401_sig_invalid() {
    const AGENT_PORT: u16 = 39_706;

    let agent_handle = spawn_test_agent_with_options(AGENT_PORT, TestAgentOptions::default());
    tokio::time::sleep(Duration::from_millis(320)).await;

    let url = format!("http://127.0.0.1:{AGENT_PORT}/instruments/search?q=RE");
    let resp = client()
        .get(&url)
        .header("x-proto-version", "1")
        .send()
        .await
        .expect("agent request");

    assert_wire_401_sig_invalid(resp).await;

    agent_handle.abort();
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

    // Wire signs path without query; upstream still Bearer-only (no wire identity leak).
    let wire_path = "/instruments/ltp";
    let url = format!(
        "http://127.0.0.1:{AGENT_PORT}{wire_path}?symbol=RELIANCE&exchange=NSE&segment=NSE"
    );
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        wire_path,
        b"",
        WireHeaderOverrides::default(),
    )
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
