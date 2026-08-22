//! Station Fact Plane boot wiring — online loop + hydrate-on-JWT.

mod common;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use tradeautopsy_agent::MemoryStationTokenStore;
use serde_json::Value;
use serde_json::json;
use serial_test::serial;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
struct FakeConsole {
    posts: Arc<Mutex<Vec<(HeaderMap, Value)>>>,
}

async fn events_200(
    State(state): State<FakeConsole>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> StatusCode {
    state.posts.lock().expect("posts").push((headers, body));
    StatusCode::OK
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn boot_with_jwt_posts_station_online_immediately() {
    std::env::set_var("STATION_ACCESS_TOKEN", "station.fact-plane.jwt");

    let fake = FakeConsole::default();
    let app = Router::new()
        .route("/api/daemon/events", post(events_200))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Console");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fake Console");
    });
    tokio::time::sleep(Duration::from_millis(30)).await;

    const AGENT_PORT: u16 = 39_620;
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{port}"));
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    let deadline = tokio::time::Instant::now() + Duration::from_millis(800);
    while fake.posts.lock().expect("posts").is_empty()
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let posts = fake.posts.lock().expect("posts");
    assert_eq!(
        posts.len(),
        1,
        "boot with JWT must POST station_online immediately, before the 20s cadence"
    );
    let (headers, body) = &posts[0];
    let auth = headers
        .get("authorization")
        .expect("Authorization")
        .to_str()
        .expect("utf8");
    assert!(
        auth.starts_with("Bearer ") && auth.len() > "Bearer ".len(),
        "Console ingest must use Station Caller Bearer"
    );
    assert_eq!(body["events"][0]["signal_type"], "station_online");
    assert_eq!(body["events"][0]["value"]["v"], 1);
    assert_eq!(body["events"][0]["session_id"], Value::Null);
    assert!(body["events"][0].get("user_id").is_none());
    drop(posts);

    handle.abort();
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn boot_with_jwt_posts_station_online_again_after_interval() {
    std::env::set_var("STATION_ACCESS_TOKEN", "station.fact-plane.jwt");

    let fake = FakeConsole::default();
    let app = Router::new()
        .route("/api/daemon/events", post(events_200))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Console");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fake Console");
    });
    tokio::time::sleep(Duration::from_millis(30)).await;

    const AGENT_PORT: u16 = 39_621;
    const COALESCE_MS: i64 = 15_000;
    let clock = Arc::new(AtomicI64::new(1_000_000));
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{port}"));
    opts.fact_online_interval_ms = Some(50);
    opts.fact_clock_ms = Some(clock.clone());
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);

    let deadline = tokio::time::Instant::now() + Duration::from_millis(800);
    while fake.posts.lock().expect("posts").len() < 1
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        1,
        "boot must POST once immediately"
    );

    clock.store(1_000_000 + COALESCE_MS, Ordering::SeqCst);

    let deadline = tokio::time::Instant::now() + Duration::from_millis(800);
    while fake.posts.lock().expect("posts").len() < 2
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let posts = fake.posts.lock().expect("posts");
    assert_eq!(
        posts.len(),
        2,
        "after 15s coalesce + injectable interval, must POST station_online again"
    );
    assert_eq!(posts[1].1["events"][0]["signal_type"], "station_online");
    assert_eq!(posts[1].1["events"][0]["value"]["v"], 1);
    assert_eq!(posts[1].1["events"][0]["session_id"], Value::Null);
    drop(posts);

    handle.abort();
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn boot_without_jwt_posts_zero_station_online_events() {
    let fake = FakeConsole::default();
    let app = Router::new()
        .route("/api/daemon/events", post(events_200))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Console");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fake Console");
    });
    tokio::time::sleep(Duration::from_millis(30)).await;

    const AGENT_PORT: u16 = 39_622;
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{port}"));
    opts.fact_online_interval_ms = Some(50);
    opts.station_token_store = Some(Arc::new(MemoryStationTokenStore::default()));
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        0,
        "no JWT must not POST station_online"
    );

    handle.abort();
}

const PLANTED_SCHEMA_VERSION: i64 = 1;
const PLANTED_PLAN_STATE: &str = "GREEN";
const PLANTED_SYNC_STATE: &str = "GREEN";

fn planted_snapshot() -> Value {
    json!({
        "schemaVersion": 1,
        "notch": { "plan_state": "GREEN", "sync_state": "GREEN" }
    })
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn boot_with_jwt_hydrates_livebook_once_then_agent_get_is_local() {
    std::env::set_var("STATION_ACCESS_TOKEN", "station.fact-plane.jwt");

    let live_state_hits = Arc::new(AtomicU64::new(0));
    let hits = Arc::clone(&live_state_hits);
    let fake = FakeConsole::default();
    let app = Router::new()
        .route("/api/daemon/events", post(events_200))
        .route(
            "/api/bar/v1/live-state",
            get(move || {
                let hits = Arc::clone(&hits);
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    Json(planted_snapshot())
                }
            }),
        )
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Console");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fake Console");
    });
    tokio::time::sleep(Duration::from_millis(30)).await;

    const AGENT_PORT: u16 = 39_623;
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{port}"));
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);

    let deadline = tokio::time::Instant::now() + Duration::from_millis(800);
    while live_state_hits.load(Ordering::SeqCst) < 1 && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        live_state_hits.load(Ordering::SeqCst),
        1,
        "boot with JWT must GET hosted /api/bar/v1/live-state once without Notch asking"
    );

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
    .expect("agent live-state");

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
        live_state_hits.load(Ordering::SeqCst),
        1,
        "agent GET must not hit hosted live-state again"
    );

    handle.abort();
}
