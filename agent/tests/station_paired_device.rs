//! Station Phase 2 — Keychain `device_id` + quiet refresh (Console #379).
//!
//! BAR retries inline after HTTP 401. Capture accept stays queued (HTTP 202);
//! the outbox refreshes once and retries delivery.

mod common;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tradeautopsy_agent::{MemoryStationTokenStore, StationTokenStore, StationTokens};

const DEVICE_ID: &str = "22222222-2222-4222-8222-222222222222";
const REFRESH: &str = "paired-refresh-fixture";

fn access_jwt(exp: u64) -> String {
    let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp},"aud":"station"}}"#));
    format!("e30.{payload}.sig")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Clone)]
struct FakeConsole {
    old_access: String,
    new_access: String,
    bar_auths: Arc<Mutex<Vec<String>>>,
    capture_auths: Arc<Mutex<Vec<String>>>,
    refresh_bodies: Arc<Mutex<Vec<Value>>>,
}

fn bearer(headers: &HeaderMap) -> String {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

async fn refresh_token(
    State(state): State<FakeConsole>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    state.refresh_bodies.lock().expect("refresh").push(body);
    (
        StatusCode::OK,
        Json(json!({
            "access_token": state.new_access,
            "refresh_token": "paired-refresh-rotated",
            "expires_in": 3600,
            "refresh_expires_in": 2_592_000,
            "device_id": DEVICE_ID
        })),
    )
}

async fn bar_declare(
    State(state): State<FakeConsole>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    let auth = bearer(&headers);
    state.bar_auths.lock().expect("bar").push(auth.clone());
    if auth == format!("Bearer {}", state.old_access) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false })));
    }
    (
        StatusCode::OK,
        Json(json!({ "ok": true, "declarationId": "d-refreshed" })),
    )
}

async fn capture_accept(
    State(state): State<FakeConsole>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    let auth = bearer(&headers);
    state
        .capture_auths
        .lock()
        .expect("capture")
        .push(auth.clone());
    if auth == format!("Bearer {}", state.old_access) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false })));
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

async fn spawn_fake(state: FakeConsole) -> String {
    let router = Router::new()
        .route("/api/auth/station/token", post(refresh_token))
        .route("/api/bar/v1/declarations", post(bar_declare))
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(capture_accept),
        )
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake console");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("fake console");
    });
    tokio::time::sleep(Duration::from_millis(40)).await;
    format!("http://127.0.0.1:{port}")
}

fn paired_store(access: &str) -> Arc<dyn StationTokenStore> {
    let store = Arc::new(MemoryStationTokenStore::default());
    store
        .save(&StationTokens {
            access_token: access.to_string(),
            refresh_token: REFRESH.to_string(),
            expires_in: 3600,
            refresh_expires_in: Some(2_592_000),
            device_id: Some(DEVICE_ID.to_string()),
        })
        .expect("seed tokens");
    store
}

fn assert_refresh_grant(body: &Value) {
    assert_eq!(body["grant_type"], "refresh_token");
    assert_eq!(body["device_id"], DEVICE_ID);
    assert_eq!(body["refresh_token"], REFRESH);
    assert!(body.get("access_token").is_none());
}

#[tokio::test]
#[serial]
async fn bar_401_refreshes_device_and_retries() {
    let old_access = access_jwt(now_secs() + 3600);
    let new_access = access_jwt(now_secs() + 7200);
    let fake = FakeConsole {
        old_access: old_access.clone(),
        new_access: new_access.clone(),
        bar_auths: Arc::new(Mutex::new(Vec::new())),
        capture_auths: Arc::new(Mutex::new(Vec::new())),
        refresh_bodies: Arc::new(Mutex::new(Vec::new())),
    };
    let base = spawn_fake(fake.clone()).await;
    let store = paired_store(&old_access);

    std::env::set_var("STATION_ACCESS_TOKEN", "bootstrap-must-not-be-sent");
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(base);
    opts.station_token_store = Some(store.clone());
    opts.fact_online_interval_ms = Some(60_000);
    const AGENT_PORT: u16 = 39_731;
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let path = "/api/daemon/bar/declare";
    let payload = json!({ "symbol": "RELIANCE" });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(format!("http://127.0.0.1:{AGENT_PORT}{path}"))
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("bar declare");

    assert_eq!(resp.status(), 200);
    let auths = fake.bar_auths.lock().expect("bar").clone();
    assert_eq!(auths.len(), 2, "401 must refresh and retry once");
    assert_eq!(auths[0], format!("Bearer {old_access}"));
    assert_eq!(auths[1], format!("Bearer {new_access}"));
    assert!(!auths
        .iter()
        .any(|a| a.contains("bootstrap-must-not-be-sent")));
    let refreshes = fake.refresh_bodies.lock().expect("refresh").clone();
    assert_eq!(refreshes.len(), 1);
    assert_refresh_grant(&refreshes[0]);

    handle.abort();
    std::env::remove_var("STATION_ACCESS_TOKEN");
}

#[tokio::test]
#[serial]
async fn quiet_refresh_rotates_expiring_access_before_bar() {
    let old_access = access_jwt(now_secs() + 30);
    let new_access = access_jwt(now_secs() + 3600);
    let fake = FakeConsole {
        old_access: old_access.clone(),
        new_access: new_access.clone(),
        bar_auths: Arc::new(Mutex::new(Vec::new())),
        capture_auths: Arc::new(Mutex::new(Vec::new())),
        refresh_bodies: Arc::new(Mutex::new(Vec::new())),
    };
    let base = spawn_fake(fake.clone()).await;
    let store = paired_store(&old_access);

    std::env::set_var("STATION_ACCESS_TOKEN", "bootstrap-must-not-be-sent");
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(base);
    opts.station_token_store = Some(store.clone());
    opts.fact_online_interval_ms = Some(60_000);
    const AGENT_PORT: u16 = 39_733;
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let path = "/api/daemon/bar/declare";
    let payload = json!({ "symbol": "RELIANCE" });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(format!("http://127.0.0.1:{AGENT_PORT}{path}"))
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("bar declare");

    assert_eq!(resp.status(), 200);
    let auths = fake.bar_auths.lock().expect("bar").clone();
    assert_eq!(auths, vec![format!("Bearer {new_access}")]);
    let refreshes = fake.refresh_bodies.lock().expect("refresh").clone();
    assert!(!refreshes.is_empty());
    assert_refresh_grant(&refreshes[0]);
    let saved = store.load().expect("load").expect("tokens");
    assert_eq!(saved.expires_in, 3600);
    assert_eq!(saved.refresh_expires_in, Some(2_592_000));
    assert_eq!(saved.device_id.as_deref(), Some(DEVICE_ID));
    assert_eq!(saved.access_token, new_access);

    handle.abort();
    std::env::remove_var("STATION_ACCESS_TOKEN");
}

#[tokio::test]
#[serial]
async fn capture_401_refreshes_and_returns_202_queued() {
    let old_access = access_jwt(now_secs() + 3600);
    let new_access = access_jwt(now_secs() + 7200);
    let fake = FakeConsole {
        old_access: old_access.clone(),
        new_access: new_access.clone(),
        bar_auths: Arc::new(Mutex::new(Vec::new())),
        capture_auths: Arc::new(Mutex::new(Vec::new())),
        refresh_bodies: Arc::new(Mutex::new(Vec::new())),
    };
    let base = spawn_fake(fake.clone()).await;
    let store = paired_store(&old_access);

    let db_path = std::env::temp_dir().join(format!("paired-capture-{}.db", uuid::Uuid::new_v4()));
    std::env::set_var("AGENT_OUTBOX_DB_PATH", &db_path);
    std::env::set_var("STATION_ACCESS_TOKEN", "bootstrap-must-not-be-sent");
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(base);
    opts.station_token_store = Some(store);
    opts.fact_online_interval_ms = Some(60_000);
    const AGENT_PORT: u16 = 39_735;
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let path = "/api/daemon/journal/toolbar-capture/accept";
    let payload = json!({
        "draftText": "paired device capture",
        "tradeId": "11111111-2222-4333-8444-555555555555",
        "explicitPending": false,
        "idempotencyKey": "paired-capture-1",
        "r2Key": null
    });
    let body_bytes = serde_json::to_vec(&payload).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(format!("http://127.0.0.1:{AGENT_PORT}{path}"))
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes.clone()),
        "POST",
        path,
        &body_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("capture");

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::ACCEPTED,
        "capture success is 202 queued"
    );
    let response_json: Value = resp.json().await.expect("json");
    assert_eq!(response_json["success"], true);
    assert_eq!(response_json["data"]["status"], "queued");

    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    let mut acked = false;
    while std::time::Instant::now() < deadline {
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            if let Ok(state) = conn.query_row(
                "SELECT state FROM capture_outbox ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            ) {
                if state == "ACKED" {
                    acked = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert!(acked, "outbox must retry after refresh and ack");

    let auths = fake.capture_auths.lock().expect("capture").clone();
    assert!(auths.len() >= 2, "401 must be retried");
    assert_eq!(auths[0], format!("Bearer {old_access}"));
    assert_eq!(auths[1], format!("Bearer {new_access}"));
    let refreshes = fake.refresh_bodies.lock().expect("refresh").clone();
    assert_eq!(refreshes.len(), 1);
    assert_refresh_grant(&refreshes[0]);

    handle.abort();
    std::env::remove_var("STATION_ACCESS_TOKEN");
    std::env::remove_var("AGENT_OUTBOX_DB_PATH");
    let _ = std::fs::remove_file(&db_path);
}
