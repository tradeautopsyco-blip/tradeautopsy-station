//! Phase 7 (issue #64): agent proxies presign + pending PATCH to hosted daemon routes.

mod common;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{patch, post};
use axum::{Json, Router};
use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use serde_json::{json, Value};
use serial_test::serial;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct UpstreamSpy {
    presign_calls: Arc<Mutex<Vec<Value>>>,
    patch_calls: Arc<Mutex<Vec<Value>>>,
    presign_last_request_id: Arc<Mutex<Option<String>>>,
    patch_last_request_id: Arc<Mutex<Option<String>>>,
    presign_429_remaining: Arc<Mutex<u32>>,
    patch_429_remaining: Arc<Mutex<u32>>,
}

impl Clone for UpstreamSpy {
    fn clone(&self) -> Self {
        Self {
            presign_calls: Arc::clone(&self.presign_calls),
            patch_calls: Arc::clone(&self.patch_calls),
            presign_last_request_id: Arc::clone(&self.presign_last_request_id),
            patch_last_request_id: Arc::clone(&self.patch_last_request_id),
            presign_429_remaining: Arc::clone(&self.presign_429_remaining),
            patch_429_remaining: Arc::clone(&self.patch_429_remaining),
        }
    }
}

impl Default for UpstreamSpy {
    fn default() -> Self {
        Self {
            presign_calls: Arc::new(Mutex::new(Vec::new())),
            patch_calls: Arc::new(Mutex::new(Vec::new())),
            presign_last_request_id: Arc::new(Mutex::new(None)),
            patch_last_request_id: Arc::new(Mutex::new(None)),
            presign_429_remaining: Arc::new(Mutex::new(0)),
            patch_429_remaining: Arc::new(Mutex::new(0)),
        }
    }
}

async fn upstream_presign(
    State(spy): State<UpstreamSpy>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> axum::response::Response {
    assert!(headers.get("x-daemon-secret").is_some());
    assert!(headers.get("x-request-id").is_some());
    let rid = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    *spy.presign_last_request_id
        .lock()
        .expect("presign rid mutex") = rid;
    spy.presign_calls
        .lock()
        .expect("presign mutex")
        .push(body.clone());

    let mut rem = spy.presign_429_remaining.lock().expect("429 mutex");
    if *rem > 0 {
        *rem -= 1;
        return (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            [(axum::http::header::RETRY_AFTER, "1")],
            Json(json!({"success": false, "error": { "code": "RATE_LIMITED" } })),
        )
            .into_response();
    }

    (
        axum::http::StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "uploadUrl": "https://up.example/put",
                "key": "journal/k.png",
                "publicUrl": "https://cdn.example/k.png",
            }
        })),
    )
        .into_response()
}

async fn upstream_patch(
    State(spy): State<UpstreamSpy>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    assert!(headers.get("x-daemon-secret").is_some());
    let rid = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    *spy.patch_last_request_id.lock().expect("patch rid mutex") = rid;
    spy.patch_calls
        .lock()
        .expect("patch mutex")
        .push(json!({ "id": id.clone(), "body": body.clone() }));

    let mut rem = spy.patch_429_remaining.lock().expect("patch 429 mutex");
    if *rem > 0 {
        *rem -= 1;
        return (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            [(axum::http::header::RETRY_AFTER, "1")],
            Json(json!({"success": false, "error": { "code": "RATE_LIMITED" } })),
        )
            .into_response();
    }

    (
        axum::http::StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "pending_capture_id": id,
                "r2_key": body["r2_key"],
            }
        })),
    )
        .into_response()
}

fn spawn_upstream_phase7(port: u16, spy: UpstreamSpy) -> tokio::task::JoinHandle<()> {
    let router = Router::new()
        .route("/api/daemon/screenshot/presign", post(upstream_presign))
        .route(
            "/api/daemon/journal/toolbar-capture/pending/:id",
            patch(upstream_patch),
        )
        .with_state(spy);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    })
}

#[tokio::test]
#[serial]
async fn agent_presign_proxies_json_to_upstream() {
    const AGENT_PORT: u16 = 19_470;
    const UPSTREAM_PORT: u16 = 19_471;
    let spy = UpstreamSpy::default();
    let upstream_handle = spawn_upstream_phase7(UPSTREAM_PORT, spy.clone());
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let path = "/api/daemon/screenshot/presign";
    let body = json!({
        "pending_capture_id": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
        "content_type": "image/png",
        "filename": "x.png",
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let fixed_rid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    let resp = apply_wire_v1(
        client().post(&url).body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides {
            request_id: Some(fixed_rid),
            ..WireHeaderOverrides::default()
        },
    )
    .header("content-type", "application/json")
    .send()
    .await
    .expect("request");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let j: Value = resp.json().await.expect("json");
    assert_eq!(j["success"], json!(true));
    assert_eq!(j["data"]["key"], json!("journal/k.png"));

    let calls = spy.presign_calls.lock().expect("presign mutex");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], body);
    assert_eq!(
        spy.presign_last_request_id
            .lock()
            .expect("presign rid mutex")
            .as_deref(),
        Some(fixed_rid),
        "presign proxy must forward bar x-request-id"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}

#[tokio::test]
#[serial]
async fn agent_presign_retries_on_upstream_429() {
    const AGENT_PORT: u16 = 19_472;
    const UPSTREAM_PORT: u16 = 19_473;
    let spy = UpstreamSpy {
        presign_429_remaining: Arc::new(Mutex::new(2)),
        ..UpstreamSpy::default()
    };
    let upstream_handle = spawn_upstream_phase7(UPSTREAM_PORT, spy.clone());
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let path = "/api/daemon/screenshot/presign";
    let body = json!({
        "pending_capture_id": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
        "content_type": "image/png",
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");

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
    .expect("request");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(
        spy.presign_calls.lock().expect("presign mutex").len(),
        3,
        "two 429 then success"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}

#[tokio::test]
#[serial]
async fn agent_pending_patch_proxies_to_upstream() {
    const AGENT_PORT: u16 = 19_474;
    const UPSTREAM_PORT: u16 = 19_475;
    let spy = UpstreamSpy::default();
    let upstream_handle = spawn_upstream_phase7(UPSTREAM_PORT, spy.clone());
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let pid = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let path = format!("/api/daemon/journal/toolbar-capture/pending/{pid}");
    let body = json!({ "r2_key": "journal-toolbar-capture/x/y/z.png" });
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let fixed_rid = "01HZYDTSJJTGPAKFYZVNPMEBNG";

    let resp = apply_wire_v1(
        client().patch(&url).body(body_bytes.clone()),
        "PATCH",
        &path,
        &body_bytes,
        WireHeaderOverrides {
            request_id: Some(fixed_rid),
            ..WireHeaderOverrides::default()
        },
    )
    .header("content-type", "application/json")
    .send()
    .await
    .expect("request");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let j: Value = resp.json().await.expect("json");
    assert_eq!(
        j["data"]["r2_key"],
        json!("journal-toolbar-capture/x/y/z.png")
    );

    assert_eq!(spy.patch_calls.lock().expect("patch mutex").len(), 1);
    assert_eq!(
        spy.patch_last_request_id
            .lock()
            .expect("patch rid mutex")
            .as_deref(),
        Some(fixed_rid),
        "pending PATCH proxy must forward bar x-request-id"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}

#[tokio::test]
#[serial]
async fn agent_pending_patch_retries_on_upstream_429() {
    const AGENT_PORT: u16 = 19_476;
    const UPSTREAM_PORT: u16 = 19_477;
    let spy = UpstreamSpy {
        patch_429_remaining: Arc::new(Mutex::new(2)),
        ..UpstreamSpy::default()
    };
    let upstream_handle = spawn_upstream_phase7(UPSTREAM_PORT, spy.clone());
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let pid = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let path = format!("/api/daemon/journal/toolbar-capture/pending/{pid}");
    let body = json!({ "r2_key": "journal-toolbar-capture/x/y/z.png" });
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");

    let resp = apply_wire_v1(
        client().patch(&url).body(body_bytes.clone()),
        "PATCH",
        &path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .header("content-type", "application/json")
    .send()
    .await
    .expect("request");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(
        spy.patch_calls.lock().expect("patch mutex").len(),
        3,
        "two 429 then success"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}
