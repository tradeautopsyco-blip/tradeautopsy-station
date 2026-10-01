//! Phase 3 (issue #59): capture delivery online finalize first tracer bullet.

mod common;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use serde_json::{json, Value};
use serial_test::serial;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
struct UpstreamCapture {
    headers: Arc<Mutex<Option<HeaderMap>>>,
    body: Arc<Mutex<Option<Value>>>,
    calls: Arc<Mutex<Vec<(String, Value)>>>,
}

async fn upstream_accept(
    State(state): State<UpstreamCapture>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    state
        .calls
        .lock()
        .expect("calls mutex")
        .push((request_id, body.clone()));
    *state.headers.lock().expect("headers mutex") = Some(headers);
    *state.body.lock().expect("body mutex") = Some(body);
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "pending_capture_id": "11111111-2222-4333-8444-555555555555",
                "status": "accepted"
            }
        })),
    )
}

async fn upstream_duplicate(
    State(state): State<UpstreamCapture>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    state
        .calls
        .lock()
        .expect("calls mutex")
        .push((request_id, body.clone()));
    *state.headers.lock().expect("headers mutex") = Some(headers);
    *state.body.lock().expect("body mutex") = Some(body);
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "pending_capture_id": "11111111-2222-4333-8444-555555555555",
                "status": "duplicate"
            }
        })),
    )
}

#[derive(Clone)]
struct RetryBehavior {
    state: UpstreamCapture,
    fail_count: Arc<Mutex<u32>>,
}

async fn upstream_502_then_accept(
    State(behavior): State<RetryBehavior>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    behavior
        .state
        .calls
        .lock()
        .expect("calls mutex")
        .push((request_id, body.clone()));
    *behavior.state.headers.lock().expect("headers mutex") = Some(headers);
    *behavior.state.body.lock().expect("body mutex") = Some(body);

    let mut guard = behavior.fail_count.lock().expect("retry count mutex");
    if *guard > 0 {
        *guard -= 1;
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "success": false,
                "error": { "code": "UPSTREAM_ERROR", "message": "bad gateway" }
            })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "pending_capture_id": "11111111-2222-4333-8444-555555555555",
                "status": "accepted"
            }
        })),
    )
}

async fn upstream_retry_then_accept(
    State(behavior): State<RetryBehavior>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    behavior
        .state
        .calls
        .lock()
        .expect("calls mutex")
        .push((request_id, body.clone()));
    *behavior.state.headers.lock().expect("headers mutex") = Some(headers);
    *behavior.state.body.lock().expect("body mutex") = Some(body);

    let mut guard = behavior.fail_count.lock().expect("retry count mutex");
    if *guard > 0 {
        *guard -= 1;
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "success": false,
                "error": { "code": "RATE_LIMITED", "message": "retry later" }
            })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "pending_capture_id": "11111111-2222-4333-8444-555555555555",
                "status": "accepted"
            }
        })),
    )
}

async fn upstream_always_503(
    State(state): State<UpstreamCapture>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    state
        .calls
        .lock()
        .expect("calls mutex")
        .push((request_id, body.clone()));
    *state.headers.lock().expect("headers mutex") = Some(headers);
    *state.body.lock().expect("body mutex") = Some(body);
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "success": false,
            "error": { "code": "UPSTREAM_ERROR", "message": "unavailable" }
        })),
    )
}

async fn upstream_validation_error(
    State(state): State<UpstreamCapture>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    state
        .calls
        .lock()
        .expect("calls mutex")
        .push((request_id, body.clone()));
    *state.headers.lock().expect("headers mutex") = Some(headers);
    *state.body.lock().expect("body mutex") = Some(body);
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "success": false,
            "error": { "code": "VALIDATION_ERROR", "message": "bad request body" }
        })),
    )
}

fn spawn_upstream(port: u16) -> (tokio::task::JoinHandle<()>, UpstreamCapture) {
    let state = UpstreamCapture::default();
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_accept),
        )
        .with_state(state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    });
    (handle, state)
}

fn spawn_upstream_duplicate(port: u16) -> (tokio::task::JoinHandle<()>, UpstreamCapture) {
    let state = UpstreamCapture::default();
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_duplicate),
        )
        .with_state(state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    });
    (handle, state)
}

fn spawn_upstream_502_then_accept(
    port: u16,
    fail_count: u32,
) -> (tokio::task::JoinHandle<()>, UpstreamCapture) {
    let state = UpstreamCapture::default();
    let behavior = RetryBehavior {
        state: state.clone(),
        fail_count: Arc::new(Mutex::new(fail_count)),
    };
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_502_then_accept),
        )
        .with_state(behavior);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    });
    (handle, state)
}

fn spawn_upstream_retry_then_accept(
    port: u16,
    fail_count: u32,
) -> (tokio::task::JoinHandle<()>, UpstreamCapture) {
    let state = UpstreamCapture::default();
    let behavior = RetryBehavior {
        state: state.clone(),
        fail_count: Arc::new(Mutex::new(fail_count)),
    };
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_retry_then_accept),
        )
        .with_state(behavior);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    });
    (handle, state)
}

fn spawn_upstream_always_503(port: u16) -> (tokio::task::JoinHandle<()>, UpstreamCapture) {
    let state = UpstreamCapture::default();
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_always_503),
        )
        .with_state(state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    });
    (handle, state)
}

fn spawn_upstream_validation(port: u16) -> (tokio::task::JoinHandle<()>, UpstreamCapture) {
    let state = UpstreamCapture::default();
    let router = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_validation_error),
        )
        .with_state(state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, router).await.expect("serve upstream");
    });
    (handle, state)
}

#[tokio::test]
#[serial]
async fn online_finalize_forwards_request_id_and_strict_body_to_upstream() {
    const AGENT_PORT: u16 = 19_450;
    const UPSTREAM_PORT: u16 = 19_451;
    let path = "/api/daemon/journal/toolbar-capture/accept";

    let (upstream_handle, upstream) = spawn_upstream(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );

    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let body = json!({
        "draftText": "Phase 3 online finalize test",
        "tradeId": "11111111-2222-4333-8444-555555555555",
        "explicitPending": false,
        "idempotencyKey": "idem-online-1",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
    let url = format!("http://127.0.0.1:{AGENT_PORT}{path}");
    let request_id = "01JTZF9F8AWD2NX5AZ5V6K8N6T";

    let resp = apply_wire_v1(
        client().post(&url).body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides {
            request_id: Some(request_id),
            ..Default::default()
        },
    )
    .header("content-type", "application/json")
    .send()
    .await
    .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);
    let response_json: Value = resp.json().await.expect("response json");
    assert_eq!(response_json["success"], Value::Bool(true));
    assert_eq!(
        response_json["data"]["status"],
        Value::String("accepted".to_string())
    );
    assert!(
        upstream
            .headers
            .lock()
            .expect("headers mutex")
            .as_ref()
            .and_then(|h| h.get("x-request-id"))
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v == request_id),
        "upstream should receive forwarded x-request-id"
    );
    assert_eq!(
        upstream
            .body
            .lock()
            .expect("body mutex")
            .clone()
            .expect("upstream body"),
        body
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}

#[tokio::test]
#[serial]
async fn online_finalize_duplicate_is_client_success() {
    const AGENT_PORT: u16 = 19_452;
    const UPSTREAM_PORT: u16 = 19_453;
    let path = "/api/daemon/journal/toolbar-capture/accept";

    let (upstream_handle, _upstream) = spawn_upstream_duplicate(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );

    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let body = json!({
        "draftText": "Phase 3 duplicate path",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-duplicate-1",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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

    assert_eq!(resp.status(), StatusCode::OK);
    let response_json: Value = resp.json().await.expect("response json");
    assert_eq!(response_json["success"], Value::Bool(true));
    assert_eq!(
        response_json["data"]["status"],
        Value::String("duplicate".to_string())
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}

#[tokio::test]
#[serial]
async fn online_finalize_rejects_unknown_fields_and_does_not_forward() {
    const AGENT_PORT: u16 = 19_454;
    const UPSTREAM_PORT: u16 = 19_455;
    let path = "/api/daemon/journal/toolbar-capture/accept";

    let (upstream_handle, upstream) = spawn_upstream(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );

    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let body = json!({
        "draftText": "unknown field should fail",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-unknown-1",
        "r2Key": null,
        "behavioralHints": {
            "risk": 0.9
        }
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let response_json: Value = resp.json().await.expect("response json");
    assert_eq!(
        response_json["error_class"],
        Value::String("VALIDATION".to_string())
    );
    assert!(
        upstream.body.lock().expect("upstream body mutex").is_none(),
        "upstream must not receive invalid strict-contract body"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
}

fn temp_outbox_path(test_name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "tradeautopsy-agent-{test_name}-{}.db",
        uuid::Uuid::new_v4()
    ));
    p
}

async fn wait_for_state(db_path: &PathBuf, want: &str, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < u128::from(timeout_ms) {
        if let Ok(conn) = rusqlite::Connection::open(db_path) {
            if let Ok(mut stmt) =
                conn.prepare("SELECT state FROM capture_outbox ORDER BY id DESC LIMIT 1")
            {
                let got = stmt.query_row([], |row| row.get::<_, String>(0));
                if let Ok(state) = got {
                    if state == want {
                        return true;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    false
}

fn read_last_outbox_row(db_path: &PathBuf) -> Option<(String, i64, Option<String>)> {
    let conn = rusqlite::Connection::open(db_path).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT state, attempts, last_error
             FROM capture_outbox
             ORDER BY id DESC
             LIMIT 1",
        )
        .ok()?;
    stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .ok()
}

async fn wait_for_agent_health(port: u16, timeout_ms: u64) -> bool {
    let path = "/api/daemon/health";
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < u128::from(timeout_ms) {
        let url = format!("http://127.0.0.1:{port}{path}");
        let req = apply_wire_v1(
            client().get(&url),
            "GET",
            path,
            b"",
            WireHeaderOverrides::default(),
        );
        if let Ok(resp) = req.send().await {
            if resp.status() == StatusCode::OK {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    false
}

fn count_state(db_path: &PathBuf, want: &str) -> usize {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let mut stmt = match conn.prepare("SELECT COUNT(*) FROM capture_outbox WHERE state = ?") {
        Ok(s) => s,
        Err(_) => return 0,
    };
    stmt.query_row([want], |row| row.get::<_, i64>(0))
        .map(|v| v as usize)
        .unwrap_or(0)
}

async fn wait_for_count_state(
    db_path: &PathBuf,
    want: &str,
    expected: usize,
    timeout_ms: u64,
) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < u128::from(timeout_ms) {
        if count_state(db_path, want) == expected {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(90)).await;
    }
    false
}

fn state_counts(db_path: &PathBuf) -> (usize, usize, usize, usize) {
    (
        count_state(db_path, "ENQUEUED"),
        count_state(db_path, "INFLIGHT"),
        count_state(db_path, "ACKED"),
        count_state(db_path, "DEAD_LETTER"),
    )
}

fn dump_rows(db_path: &PathBuf) -> Vec<(i64, String, i64, i64)> {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, state, attempts, next_attempt_at_ms
         FROM capture_outbox
         ORDER BY id ASC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    let now = chrono::Utc::now().timestamp_millis();
    let mut rows = match stmt.query([]) {
        Ok(r) => r,
        Err(_) => return out,
    };
    while let Ok(Some(row)) = rows.next() {
        let id: i64 = row.get(0).unwrap_or_default();
        let state: String = row.get(1).unwrap_or_default();
        let attempts: i64 = row.get(2).unwrap_or_default();
        let next_at: i64 = row.get(3).unwrap_or_default();
        out.push((id, state, attempts, next_at - now));
    }
    out
}

#[tokio::test]
#[serial]
async fn offline_finalize_is_queued_in_outbox_enqueued_state() {
    const AGENT_PORT: u16 = 19_456;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("offline-enqueued");

    std::env::set_var("TRADEAUTOPSY_SERVER_BASE_URL", "http://127.0.0.1:39999");
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let body = json!({
        "draftText": "queue me while offline",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-offline-1",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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

    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    let mut stmt = conn
        .prepare("SELECT state FROM capture_outbox ORDER BY id DESC LIMIT 1")
        .expect("prepare");
    let state: String = stmt.query_row([], |row| row.get(0)).expect("state row");
    assert_eq!(state, "ENQUEUED");

    agent_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn restart_recovers_inflight_row_and_drains_to_acked() {
    const AGENT_PORT: u16 = 19_457;
    const RESTARTED_PORT: u16 = 19_459;
    const UPSTREAM_PORT: u16 = 19_458;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("restart-inflight");

    std::env::set_var("TRADEAUTOPSY_SERVER_BASE_URL", "http://127.0.0.1:39999");
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let body = json!({
        "draftText": "crash recovery",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-restart-1",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    agent_handle.abort();
    tokio::time::sleep(Duration::from_millis(200)).await;

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
        conn.execute(
            "UPDATE capture_outbox SET state = 'INFLIGHT' WHERE state <> 'ACKED'",
            [],
        )
        .expect("mark inflight");
    }

    let (upstream_handle, _upstream) = spawn_upstream(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    let restarted = spawn_test_agent(RESTARTED_PORT);
    let restarted_ok = wait_for_agent_health(RESTARTED_PORT, 2_000).await;
    assert!(
        restarted_ok,
        "restarted agent failed health check; handle finished={}",
        restarted.is_finished()
    );
    let acked = wait_for_state(&db_path, "ACKED", 8_000).await;
    let diag = read_last_outbox_row(&db_path);
    assert!(
        acked,
        "row should be drainable after restart and become ACKED; got {diag:?}"
    );

    restarted.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn offline_queue_reconnect_drains_fifo_without_duplicate_server_effects() {
    const AGENT_PORT: u16 = 19_460;
    const UPSTREAM_PORT: u16 = 19_461;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("offline-reconnect-fifo");

    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    let keys = ["idem-fifo-1", "idem-fifo-2", "idem-fifo-3"];
    for key in keys {
        let body = json!({
            "draftText": format!("queued {key}"),
            "tradeId": null,
            "explicitPending": true,
            "idempotencyKey": key,
            "r2Key": null
        });
        let body_bytes = serde_json::to_vec(&body).expect("body json");
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
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    let (upstream_handle, upstream) = spawn_upstream(UPSTREAM_PORT);
    let drained = wait_for_count_state(&db_path, "ACKED", 3, 20_000).await;
    let counts = state_counts(&db_path);
    let rows = dump_rows(&db_path);
    assert!(
        drained,
        "all queued captures should drain to ACKED; counts={counts:?} rows={rows:?}"
    );

    let calls = upstream.calls.lock().expect("calls mutex").clone();
    assert_eq!(
        calls.len(),
        3,
        "server should receive exactly one call per capture"
    );
    let observed_keys: Vec<String> = calls
        .iter()
        .map(|(_, body)| {
            body["idempotencyKey"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(
        observed_keys,
        vec![
            "idem-fifo-1".to_string(),
            "idem-fifo-2".to_string(),
            "idem-fifo-3".to_string()
        ],
        "drain order should remain FIFO by enqueue order"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn bounded_workers_parallel_drain_each_capture_once_upstream() {
    const AGENT_PORT: u16 = 19_472;
    const UPSTREAM_PORT: u16 = 19_473;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("multi-worker-drain");
    const N: usize = 12;

    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);
    std::env::set_var("AGENT_OUTBOX_WORKERS", "4");

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    for i in 0..N {
        let key = format!("idem-mw-{i}");
        let body = json!({
            "draftText": format!("multi-worker {key}"),
            "tradeId": null,
            "explicitPending": true,
            "idempotencyKey": key,
            "r2Key": null
        });
        let body_bytes = serde_json::to_vec(&body).expect("body json");
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
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    let (upstream_handle, upstream) = spawn_upstream(UPSTREAM_PORT);
    let drained = wait_for_count_state(&db_path, "ACKED", N, 30_000).await;
    let counts = state_counts(&db_path);
    assert!(
        drained,
        "bounded workers should drain full backlog to ACKED; counts={counts:?}"
    );
    assert_eq!(count_state(&db_path, "INFLIGHT"), 0);
    assert_eq!(count_state(&db_path, "ENQUEUED"), 0);
    assert_eq!(count_state(&db_path, "DEAD_LETTER"), 0);

    let calls = upstream.calls.lock().expect("calls mutex").clone();
    assert_eq!(
        calls.len(),
        N,
        "upstream must see exactly one finalize per capture"
    );
    let mut observed_keys: Vec<String> = calls
        .iter()
        .map(|(_, body)| {
            body["idempotencyKey"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    observed_keys.sort();
    let mut expected: Vec<String> = (0..N).map(|i| format!("idem-mw-{i}")).collect();
    expected.sort();
    assert_eq!(observed_keys, expected);

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    std::env::remove_var("AGENT_OUTBOX_WORKERS");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn retryable_429_eventually_acks_and_retries_with_safe_request_ids() {
    const AGENT_PORT: u16 = 19_462;
    const UPSTREAM_PORT: u16 = 19_463;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("retry-429");

    let (upstream_handle, upstream) = spawn_upstream_retry_then_accept(UPSTREAM_PORT, 2);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    let body = json!({
        "draftText": "rate-limit retry",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-retry-429",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let acked = wait_for_count_state(&db_path, "ACKED", 1, 10_000).await;
    assert!(acked, "429 retries should eventually ACK");

    let calls = upstream.calls.lock().expect("calls mutex").clone();
    assert!(calls.len() >= 3, "should include retries before accept");
    for (_, body) in &calls {
        assert_eq!(
            body["idempotencyKey"],
            Value::String("idem-retry-429".to_string())
        );
    }
    let mut request_ids: Vec<String> = calls.iter().map(|(rid, _)| rid.clone()).collect();
    request_ids.sort();
    request_ids.dedup();
    assert!(
        request_ids.len() >= 2,
        "retry path should rotate request-id for safe retries"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn retryable_502_eventually_acks_like_other_5xx() {
    const AGENT_PORT: u16 = 19_468;
    const UPSTREAM_PORT: u16 = 19_469;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("retry-502");

    let (upstream_handle, _upstream) = spawn_upstream_502_then_accept(UPSTREAM_PORT, 1);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    let body = json!({
        "draftText": "5xx retry",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-retry-502",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let acked = wait_for_count_state(&db_path, "ACKED", 1, 10_000).await;
    assert!(
        acked,
        "502 responses should use retry path then eventually ACK"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn validation_error_goes_dead_letter_without_retry_loop() {
    const AGENT_PORT: u16 = 19_464;
    const UPSTREAM_PORT: u16 = 19_465;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("validation-deadletter");

    let (upstream_handle, upstream) = spawn_upstream_validation(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    let body = json!({
        "draftText": "validation path",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-validation",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response_json: Value = resp.json().await.expect("response json");
    assert_eq!(
        response_json["error_class"],
        Value::String("DEAD_LETTER".to_string())
    );
    assert_eq!(
        response_json["reason"],
        Value::String("validation".to_string())
    );

    let dead = wait_for_count_state(&db_path, "DEAD_LETTER", 1, 3_000).await;
    assert!(
        dead,
        "validation failures must dead-letter deterministically"
    );
    let calls = upstream.calls.lock().expect("calls mutex").clone();
    assert_eq!(calls.len(), 1, "validation path should not retry loop");

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn outbox_status_endpoint_surfaces_dead_letter_for_settings() {
    const AGENT_PORT: u16 = 19_466;
    const UPSTREAM_PORT: u16 = 19_467;
    let accept_path = "/api/daemon/journal/toolbar-capture/accept";
    let status_path = "/api/daemon/journal/toolbar-capture/outbox/status";
    let db_path = temp_outbox_path("status-deadletter");

    let (upstream_handle, _upstream) = spawn_upstream_validation(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    let body = json!({
        "draftText": "surface in settings",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-status-deadletter",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
    let accept_url = format!("http://127.0.0.1:{AGENT_PORT}{accept_path}");
    let accept_resp = apply_wire_v1(
        client().post(&accept_url).body(body_bytes.clone()),
        "POST",
        accept_path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .header("content-type", "application/json")
    .send()
    .await
    .expect("request");
    assert_eq!(accept_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let status_url = format!("http://127.0.0.1:{AGENT_PORT}{status_path}");
    let status_resp = apply_wire_v1(
        client().get(&status_url),
        "GET",
        status_path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("status request");

    assert_eq!(status_resp.status(), StatusCode::OK);
    let status_json: Value = status_resp.json().await.expect("status json");
    assert_eq!(status_json["success"], Value::Bool(true));
    assert_eq!(status_json["data"]["counts"]["dead_letter"], Value::from(1));
    assert_eq!(status_json["data"]["counts"]["acked"], Value::from(0));
    let first = &status_json["data"]["dead_letters"][0];
    assert_eq!(
        first["idempotencyKey"],
        Value::String("idem-status-deadletter".to_string())
    );
    assert_eq!(first["reason"], Value::String("validation".to_string()));
    assert!(
        status_json["data"]["active_deliveries"].is_array(),
        "status should include in-flight delivery tracking"
    );

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
#[serial]
async fn retry_exhaustion_dead_letters_with_max_attempts_reason() {
    const AGENT_PORT: u16 = 19_468;
    const UPSTREAM_PORT: u16 = 19_469;
    let path = "/api/daemon/journal/toolbar-capture/accept";
    let db_path = temp_outbox_path("max-attempts-deadletter");

    let (upstream_handle, _upstream) = spawn_upstream_always_503(UPSTREAM_PORT);
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{UPSTREAM_PORT}"),
    );
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);
    std::env::set_var("AGENT_OUTBOX_MAX_ATTEMPTS", "2");
    std::env::set_var("AGENT_OUTBOX_BASE_BACKOFF_MS", "1");
    std::env::set_var("AGENT_OUTBOX_MAX_BACKOFF_MS", "2");

    let agent_handle = spawn_test_agent(AGENT_PORT);
    assert!(wait_for_agent_health(AGENT_PORT, 2_000).await);

    let body = json!({
        "draftText": "retry cap",
        "tradeId": null,
        "explicitPending": true,
        "idempotencyKey": "idem-max-attempts",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&body).expect("body json");
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
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let dead = wait_for_count_state(&db_path, "DEAD_LETTER", 1, 8_000).await;
    assert!(dead, "persistent 503 should dead-letter after max attempts");
    let row = read_last_outbox_row(&db_path).expect("row");
    assert_eq!(row.0, "DEAD_LETTER");
    assert_eq!(row.2.as_deref(), Some("max_attempts"));

    agent_handle.abort();
    upstream_handle.abort();
    std::env::remove_var("TRADEAUTOPSY_SERVER_BASE_URL");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    std::env::remove_var("AGENT_OUTBOX_MAX_ATTEMPTS");
    std::env::remove_var("AGENT_OUTBOX_BASE_BACKOFF_MS");
    std::env::remove_var("AGENT_OUTBOX_MAX_BACKOFF_MS");
    let _ = std::fs::remove_file(db_path);
}
