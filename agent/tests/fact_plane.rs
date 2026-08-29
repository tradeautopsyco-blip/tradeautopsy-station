//! Station Fact Plane slice 2 — fact outbox → `POST /api/daemon/events`.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tradeautopsy_agent::{
    EnqueueOutcome, Fact, FactOutbox, MemoryStationTokenStore, UpstreamClient, UpstreamConfig,
};

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

#[tokio::test]
#[serial]
async fn enqueue_station_online_200_marks_row_done() {
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

    let upstream = Arc::new(
        UpstreamClient::new(UpstreamConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            daemon_secret: "unused-for-facts".to_string(),
        })
        .expect("upstream"),
    );
    let db = std::env::temp_dir().join(format!("fact-outbox-done-{}.db", uuid::Uuid::new_v4()));
    let outbox = FactOutbox::open(&db, upstream).expect("open");

    let EnqueueOutcome::Enqueued { id } = outbox.enqueue(Fact::StationOnline).expect("enqueue")
    else {
        panic!("expected enqueue");
    };

    outbox.drain().await.expect("drain");

    let row = outbox.row(&id).expect("query").expect("row");
    assert!(row.done_at_ms.is_some(), "200 must set done_at_ms");

    let posts = fake.posts.lock().expect("posts");
    assert_eq!(posts.len(), 1);
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
    assert_eq!(
        body["events"],
        json!([{
            "signal_type": "station_online",
            "value": { "v": 1, "event_id": id },
            "session_id": null
        }])
    );
    assert!(body["events"][0].get("user_id").is_none());
    drop(posts);

    outbox.drain().await.expect("second drain");
    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        1,
        "done row must not be retried"
    );
}

async fn events_500(
    State(state): State<FakeConsole>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> StatusCode {
    state.posts.lock().expect("posts").push((headers, body));
    StatusCode::INTERNAL_SERVER_ERROR
}

#[tokio::test]
#[serial]
async fn enqueue_station_online_500_leaves_row_for_retry() {
    std::env::set_var("STATION_ACCESS_TOKEN", "station.fact-plane.jwt");

    let fake = FakeConsole::default();
    let app = Router::new()
        .route("/api/daemon/events", post(events_500))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Console");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fake Console");
    });
    tokio::time::sleep(Duration::from_millis(30)).await;

    let upstream = Arc::new(
        UpstreamClient::new(UpstreamConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            daemon_secret: "unused-for-facts".to_string(),
        })
        .expect("upstream"),
    );
    let db = std::env::temp_dir().join(format!("fact-outbox-retry-{}.db", uuid::Uuid::new_v4()));
    let outbox = FactOutbox::open(&db, upstream).expect("open");

    let EnqueueOutcome::Enqueued { id } = outbox.enqueue(Fact::StationOnline).expect("enqueue")
    else {
        panic!("expected enqueue");
    };

    outbox.drain().await.expect("drain");

    let row = outbox.row(&id).expect("query").expect("row");
    assert!(row.done_at_ms.is_none(), "500 must not set done_at_ms");
    assert_eq!(fake.posts.lock().expect("posts").len(), 1);

    outbox.drain().await.expect("immediate second drain");
    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        1,
        "500 must wait 300ms before retry"
    );

    tokio::time::sleep(Duration::from_millis(300)).await;
    outbox.drain().await.expect("retry drain");
    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        2,
        "row must retry after 300ms backoff"
    );
    let row = outbox.row(&id).expect("query").expect("row");
    assert!(row.done_at_ms.is_none(), "still 500: not done");
}

#[tokio::test]
#[serial]
async fn second_station_online_within_15s_is_dropped() {
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

    let upstream = Arc::new(
        UpstreamClient::new(UpstreamConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            daemon_secret: "unused-for-facts".to_string(),
        })
        .expect("upstream"),
    );
    let db = std::env::temp_dir().join(format!("fact-outbox-coalesce-{}.db", uuid::Uuid::new_v4()));
    let outbox = FactOutbox::open(&db, upstream).expect("open");

    let EnqueueOutcome::Enqueued { id } = outbox.enqueue(Fact::StationOnline).expect("enqueue")
    else {
        panic!("expected enqueue");
    };
    outbox.drain().await.expect("drain");
    assert!(outbox
        .row(&id)
        .expect("query")
        .expect("row")
        .done_at_ms
        .is_some());
    assert_eq!(fake.posts.lock().expect("posts").len(), 1);

    assert!(
        matches!(
            outbox.enqueue(Fact::StationOnline).expect("second enqueue"),
            EnqueueOutcome::Coalesced
        ),
        "second station_online within 15s must be dropped"
    );

    outbox.drain().await.expect("drain after coalesce");
    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        1,
        "coalesce must not send a second packet"
    );
}

#[tokio::test]
#[serial]
async fn no_jwt_skips_enqueue_and_does_not_post() {
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

    let upstream = Arc::new(
        UpstreamClient::new(UpstreamConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            daemon_secret: "unused-for-facts".to_string(),
        })
        .expect("upstream"),
    );
    let db = std::env::temp_dir().join(format!("fact-outbox-no-jwt-{}.db", uuid::Uuid::new_v4()));
    let tokens = Arc::new(MemoryStationTokenStore::default());
    let outbox = FactOutbox::open(&db, upstream)
        .expect("open")
        .with_token_store(tokens);

    assert!(
        matches!(
            outbox.enqueue(Fact::StationOnline).expect("enqueue"),
            EnqueueOutcome::SkippedNoJwt
        ),
        "no JWT must not enqueue"
    );

    outbox.drain().await.expect("drain");
    assert_eq!(
        fake.posts.lock().expect("posts").len(),
        0,
        "no JWT must not POST"
    );
}
