//! Declare/cancel vs Console supersede.
//!
//! Live soak (concurrency 4) saw cancel HTTP 409 `declaration_not_cancellable`
//! because the row was already `superseded`. A second cancel of a fresh id
//! also returned 409. Both are idempotent: the declaration is not pending.

mod common;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, wait_ready, TestAgentOptions,
    WireHeaderOverrides,
};
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};

#[derive(Clone)]
struct Console {
    rows: Arc<Mutex<HashMap<String, String>>>,
    /// New declare marks every still-pending row `superseded` (Console side effect).
    supersede_on_declare: bool,
    cancel_posts: Arc<AtomicUsize>,
    /// First N cancel posts return 409 while leaving the row `pending`.
    hold_pending_conflicts: Arc<AtomicUsize>,
}

impl Console {
    fn open(supersede_on_declare: bool) -> Self {
        Self {
            rows: Arc::new(Mutex::new(HashMap::new())),
            supersede_on_declare,
            cancel_posts: Arc::new(AtomicUsize::new(0)),
            hold_pending_conflicts: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn router(self) -> Router {
        Router::new()
            .route(
                "/api/bar/v1/declarations",
                get(list_declarations).post(declare),
            )
            .route(
                "/api/bar/v1/declarations/:id/cancel",
                post(cancel_declaration),
            )
            .with_state(self)
    }
}

async fn list_declarations(State(console): State<Console>) -> Json<Value> {
    let rows = console.rows.lock().expect("rows");
    let items: Vec<Value> = rows
        .iter()
        .map(|(id, status)| json!({ "id": id, "status": status }))
        .collect();
    Json(json!({ "ok": true, "items": items }))
}

async fn declare(State(console): State<Console>, Json(_body): Json<Value>) -> Json<Value> {
    let mut rows = console.rows.lock().expect("rows");
    if console.supersede_on_declare {
        for status in rows.values_mut() {
            if status.eq_ignore_ascii_case("pending") {
                *status = "superseded".to_string();
            }
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    rows.insert(id.clone(), "pending".to_string());
    Json(json!({ "ok": true, "declarationId": id }))
}

async fn cancel_declaration(
    State(console): State<Console>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    console.cancel_posts.fetch_add(1, Ordering::SeqCst);
    let mut rows = console.rows.lock().expect("rows");
    let status = rows.get(&id).cloned().unwrap_or_default();
    if console.hold_pending_conflicts.load(Ordering::SeqCst) > 0
        && status.eq_ignore_ascii_case("pending")
    {
        console
            .hold_pending_conflicts
            .fetch_sub(1, Ordering::SeqCst);
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "ok": false,
                "error": { "code": "declaration_not_cancellable" }
            })),
        );
    }
    if status.eq_ignore_ascii_case("pending") {
        rows.insert(id, "cancelled".to_string());
        return (StatusCode::OK, Json(json!({ "ok": true })));
    }
    (
        StatusCode::CONFLICT,
        Json(json!({
            "ok": false,
            "error": { "code": "declaration_not_cancellable" }
        })),
    )
}

async fn start(agent_port: u16, upstream: Router) -> tokio::task::JoinHandle<()> {
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
    let handle = spawn_test_agent_with_options(agent_port, opts);
    wait_ready(agent_port).await;
    handle
}

async fn post_json(port: u16, path: &str, body: Value) -> (StatusCode, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let bytes = serde_json::to_vec(&body).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes.clone()),
        "POST",
        path,
        &bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("agent");
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let parsed = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, parsed)
}

async fn declare_one(port: u16) -> String {
    let (status, body) = post_json(
        port,
        "/api/daemon/bar/declare",
        json!({ "symbol": "BTCUSDT", "side": "BUY", "quantity": 0.001 }),
    )
    .await;
    assert!(status.is_success(), "declare HTTP {status} {body}");
    body["declarationId"]
        .as_str()
        .expect("declarationId")
        .to_string()
}

async fn cancel_one(port: u16, declaration_id: &str) -> (StatusCode, Value) {
    post_json(
        port,
        "/api/daemon/bar/cancel-declaration",
        json!({
            "declaration_id": declaration_id,
            "cancel_reason_chip": "scratch"
        }),
    )
    .await
}

fn pending_ids(book: &Value) -> Vec<String> {
    book["notch"]["pending_declarations"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn concurrent_declare_cancel_treats_superseded_as_idempotent() {
    const AGENT_PORT: u16 = 39_750;
    let handle = start(AGENT_PORT, Console::open(true).router()).await;

    // All four declares finish before any cancel, matching the soak: overlapping
    // declares supersede an earlier row, then that row's cancel must not 409.
    let barrier = Arc::new(tokio::sync::Barrier::new(4));
    let mut set = tokio::task::JoinSet::new();
    for _ in 0..4 {
        let barrier = Arc::clone(&barrier);
        set.spawn(async move {
            let id = declare_one(AGENT_PORT).await;
            barrier.wait().await;
            let (status, body) = cancel_one(AGENT_PORT, &id).await;
            assert!(status.is_success(), "cancel {id} HTTP {status} {body}");
            (id, body)
        });
    }
    let mut saw_superseded = false;
    while let Some(joined) = set.join_next().await {
        let (_id, body) = joined.expect("join");
        if body["idempotent"] == true && body["status"] == "superseded" {
            saw_superseded = true;
        }
    }
    assert!(
        saw_superseded,
        "concurrency 4 must retire at least one row via supersede and still cancel cleanly"
    );

    handle.abort();
}

#[tokio::test]
async fn cancel_after_supersede_clears_local_pending() {
    const AGENT_PORT: u16 = 39_751;
    let handle = start(AGENT_PORT, Console::open(true).router()).await;

    let first = declare_one(AGENT_PORT).await;
    let second = declare_one(AGENT_PORT).await;
    let (status, body) = cancel_one(AGENT_PORT, &first).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["idempotent"], true);
    assert_eq!(body["status"], "superseded");
    assert_eq!(body["declarationId"], first);

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
    .expect("live-state");
    let book: Value = resp.json().await.expect("json");
    let pending = pending_ids(&book);
    assert!(
        !pending.iter().any(|id| id == &first),
        "superseded cancel must drop the local row: {pending:?}"
    );
    assert!(
        pending.iter().any(|id| id == &second),
        "the newer pending declaration stays: {pending:?}"
    );

    handle.abort();
}

#[tokio::test]
async fn second_cancel_of_cancelled_id_is_idempotent() {
    const AGENT_PORT: u16 = 39_752;
    let handle = start(AGENT_PORT, Console::open(false).router()).await;

    let id = declare_one(AGENT_PORT).await;
    let (first_status, first_body) = cancel_one(AGENT_PORT, &id).await;
    assert_eq!(first_status, 200, "{first_body}");
    assert_eq!(first_body["ok"], true);
    assert!(first_body.get("idempotent").is_none());

    let (second_status, second_body) = cancel_one(AGENT_PORT, &id).await;
    assert_eq!(second_status, 200, "{second_body}");
    assert_eq!(second_body["idempotent"], true);
    assert_eq!(second_body["status"], "cancelled");

    handle.abort();
}

#[tokio::test]
async fn pending_conflict_retries_cancel_once() {
    const AGENT_PORT: u16 = 39_753;
    let console = Console::open(false);
    console.hold_pending_conflicts.store(1, Ordering::SeqCst);
    let posts = Arc::clone(&console.cancel_posts);
    let handle = start(AGENT_PORT, console.router()).await;

    let id = declare_one(AGENT_PORT).await;
    let (status, body) = cancel_one(AGENT_PORT, &id).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["ok"], true);
    assert!(
        body.get("idempotent").is_none(),
        "retry must land the real cancel, not an idempotent read: {body}"
    );
    assert_eq!(posts.load(Ordering::SeqCst), 2);

    handle.abort();
}

#[tokio::test]
async fn matched_and_sl_block_stay_conflicts() {
    const AGENT_PORT: u16 = 39_754;
    let matched = "00000000-0000-4000-8000-0000000000aa";
    let blocked = "00000000-0000-4000-8000-0000000000bb";
    let upstream = Router::new()
        .route(
            "/api/bar/v1/declarations",
            get(move || async move {
                Json(json!({
                    "items": [{
                        "id": matched,
                        "status": "matched"
                    }]
                }))
            }),
        )
        .route(
            "/api/bar/v1/declarations/:id/cancel",
            post(move |Path(id): Path<String>| async move {
                let code = if id == blocked {
                    "declaration_cancel_blocked"
                } else {
                    "declaration_not_cancellable"
                };
                (
                    StatusCode::CONFLICT,
                    Json(json!({ "ok": false, "error": { "code": code } })),
                )
            }),
        );
    let handle = start(AGENT_PORT, upstream).await;

    let (matched_status, matched_body) = cancel_one(AGENT_PORT, matched).await;
    assert_eq!(matched_status, 409, "{matched_body}");
    assert_eq!(matched_body["error"]["code"], "declaration_not_cancellable");

    let (blocked_status, blocked_body) = cancel_one(AGENT_PORT, blocked).await;
    assert_eq!(blocked_status, 409, "{blocked_body}");
    assert_eq!(blocked_body["error"]["code"], "declaration_cancel_blocked");

    handle.abort();
}
