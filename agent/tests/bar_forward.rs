//! Bar forwarders — agent `/api/daemon/bar/*` → hosted `/api/bar/v1/*`.

mod common;

use axum::http::{HeaderMap, StatusCode};
use axum::routing::{patch, post};
use axum::{Json, Router};
use std::sync::{Arc, Mutex};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};
use std::time::Duration;

#[tokio::test]
async fn bar_declare_proxies_post_body_to_upstream() {
    const AGENT_PORT: u16 = 39_602;
    let upstream = Router::new().route(
        "/api/bar/v1/declarations",
        post(|axum::Json(b): axum::Json<Value>| async move {
            if b.get("symbol").and_then(|v| v.as_str()) == Some("RELIANCE") {
                (
                    StatusCode::CREATED,
                    Json(json!({ "ok": true, "declarationId": "d-test" })),
                )
            } else {
                (StatusCode::BAD_REQUEST, Json(json!({ "ok": false })))
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
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/declare";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let payload = json!({ "symbol": "RELIANCE" });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");

    assert_eq!(resp.status(), 201);
    let out: Value = resp.json().await.expect("json");
    assert_eq!(out["ok"], true);
    assert_eq!(out["declarationId"], "d-test");

    handle.abort();
}

#[tokio::test]
async fn bar_declare_forwards_station_bearer_not_daemon_identity() {
    const AGENT_PORT: u16 = 39_606;
    let captured = Arc::new(Mutex::new(None::<HeaderMap>));
    let captured_clone = Arc::clone(&captured);
    let upstream = Router::new().route(
        "/api/bar/v1/declarations",
        post(
            move |headers: HeaderMap, axum::Json(b): axum::Json<Value>| async move {
                *captured_clone.lock().expect("lock") = Some(headers);
                if b.get("symbol").and_then(|v| v.as_str()) == Some("RELIANCE") {
                    (
                        StatusCode::OK,
                        Json(json!({ "ok": true, "declarationId": "d-auth-test" })),
                    )
                } else {
                    (StatusCode::BAD_REQUEST, Json(json!({ "ok": false })))
                }
            },
        ),
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
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/declare";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let payload = json!({
        "symbol": "RELIANCE",
        "declaration_kind": "intraday",
        "side": "BUY",
        "quantity": 1,
        "stop_loss": 2450,
        "declaration_payload": { "v": 1, "s1": {}, "protective_sl_consent": true }
    });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");

    assert_eq!(resp.status(), 200);
    let out: Value = resp.json().await.expect("json");
    assert_eq!(out["declarationId"], "d-auth-test");

    let headers = captured.lock().expect("lock").take().expect("headers captured");
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(auth.starts_with("Bearer "), "Station Bearer must be forwarded");
    assert!(headers.get("x-daemon-secret").is_none(), "must not forward wire secret");
    assert!(headers.get("x-user-id").is_none(), "must not forward wire user hint as identity");

    handle.abort();
}

#[tokio::test]
async fn bar_stop_me_proxies_post_to_upstream() {
    const AGENT_PORT: u16 = 39_603;
    let upstream = Router::new().route(
        "/api/bar/v1/declarations/stop-me",
        post(|| async { Json(json!({ "ok": true, "kill_switch_active": true, "level": 3 })) }),
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
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/stop-me";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let payload = json!({ "reason": "test", "triggered_at_ms": 1700000000000i64 });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");

    assert_eq!(resp.status(), 200);
    let out: Value = resp.json().await.expect("json");
    assert_eq!(out["ok"], true);

    handle.abort();
}

#[tokio::test]
async fn bar_stop_me_clear_proxies_post_to_upstream() {
    const AGENT_PORT: u16 = 39_607;
    let upstream = Router::new().route(
        "/api/bar/v1/declarations/stop-me/clear",
        post(|| async { Json(json!({ "ok": true, "kill_switch_active": false })) }),
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
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/stop-me/clear";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let payload = json!({ "released_at_ms": 1700000000001i64 });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");

    assert_eq!(resp.status(), 200);
    let out: Value = resp.json().await.expect("json");
    assert_eq!(out["ok"], true);
    assert_eq!(out["kill_switch_active"], false);

    handle.abort();
}

#[tokio::test]
async fn bar_swing_check_in_proxies_post_to_upstream() {
    const AGENT_PORT: u16 = 39_604;
    let upstream = Router::new().route(
        "/api/bar/v1/notch/swing-check-in",
        post(|axum::Json(b): axum::Json<Value>| async move {
            if b.get("thesis_intact").and_then(|v| v.as_bool()) == Some(true) {
                (StatusCode::OK, Json(json!({ "ok": true })))
            } else {
                (StatusCode::BAD_REQUEST, Json(json!({ "ok": false })))
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
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/swing-check-in";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let payload = json!({ "v": 1, "thesis_intact": true, "at_ms": 1700000000000i64 });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");

    assert_eq!(resp.status(), 200);

    handle.abort();
}

#[tokio::test]
async fn bar_post_trade_debrief_proxies_patch_to_upstream() {
    const AGENT_PORT: u16 = 39_605;
    let upstream = Router::new().route(
        "/api/bar/v1/post-trade-debrief",
        patch(|axum::Json(b): axum::Json<Value>| async move {
            if b.get("v").and_then(|v| v.as_u64()) == Some(1) {
                (StatusCode::OK, Json(json!({ "ok": true })))
            } else {
                (StatusCode::BAD_REQUEST, Json(json!({ "ok": false })))
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
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let path = "/api/daemon/bar/post-trade-debrief";
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let payload = json!({ "v": 1, "patch": true });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .patch(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "PATCH",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");

    assert_eq!(resp.status(), 200);

    handle.abort();
}
