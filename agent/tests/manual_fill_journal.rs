//! Manual fill → recent-trades + capture outbox (journal lane).

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
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
struct UpstreamCapture {
    body: Arc<Mutex<Option<Value>>>,
}

async fn upstream_accept(
    State(state): State<UpstreamCapture>,
    _headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    *state.body.lock().expect("body") = Some(body);
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

async fn forward_manual_fill(
    agent_port: u16,
    upstream_port: u16,
    body: Value,
) -> (StatusCode, Value, Value) {
    let path = "/api/daemon/journal/manual-fill/accept";

    let upstream = UpstreamCapture::default();
    let upstream_body = upstream.body.clone();
    let upstream_app = Router::new()
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(upstream_accept),
        )
        .with_state(upstream.clone());
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], upstream_port));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("bind upstream");
        axum::serve(listener, upstream_app)
            .await
            .expect("upstream serve");
    });

    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        format!("http://127.0.0.1:{upstream_port}"),
    );

    let recent_db =
        std::env::temp_dir().join(format!("manual-fill-recent-{}.db", uuid::Uuid::new_v4()));
    let _agent = spawn_test_agent_with_options(
        agent_port,
        TestAgentOptions {
            recent_trades_db_path: Some(recent_db),
            ..Default::default()
        },
    );
    tokio::time::sleep(Duration::from_millis(350)).await;

    let body_bytes = serde_json::to_vec(&body).expect("body json");
    let url = format!("http://127.0.0.1:{agent_port}{path}");

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
    .expect("manual fill");

    let status = resp.status();
    let envelope: Value = resp.json().await.expect("json");
    let mut upstream = None;
    for _ in 0..40 {
        upstream = upstream_body.lock().expect("upstream").clone();
        if upstream.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let upstream = upstream.expect("forwarded");

    let rt_path = "/api/daemon/toolbar/recent-trades";
    let rt_url = format!("http://127.0.0.1:{agent_port}{rt_path}");
    let rt_resp = apply_wire_v1(
        client().get(&rt_url),
        "GET",
        rt_path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("recent-trades");
    let rt_json: Value = rt_resp.json().await.expect("rt json");
    let trades = rt_json["trades"].as_array().expect("trades");
    assert!(
        trades
            .iter()
            .any(|t| t["broker"] == "manual" && t["symbol"] == "PNB"),
        "manual fill must appear in local recent-trades"
    );

    (status, envelope, upstream)
}

#[tokio::test]
#[serial]
async fn manual_fill_pending_link_when_no_console_trade() {
    let decl = "0491f631-78d1-496b-b191-4ade1dacb0df";
    let body = json!({
        "symbol": "PNB",
        "side": "BUY",
        "quantity": 100,
        "price": 95.5,
        "filledAtMs": 1_757_000_000_000_i64,
        "preTradeDeclarationId": decl,
        "idempotencyKey": "manual-fill-pending-1",
    });
    let (status, envelope, upstream) = forward_manual_fill(19_460, 19_461, body).await;
    assert!(
        status == StatusCode::OK || status == StatusCode::ACCEPTED,
        "expected 200 or 202, got {status}"
    );
    assert_eq!(envelope["success"], true);
    assert_eq!(envelope["data"]["journalFillSource"], "manual");
    assert!(envelope["data"].get("tradeId").is_none() || envelope["data"]["tradeId"].is_null());
    if status == StatusCode::ACCEPTED {
        assert_eq!(envelope["data"]["delivery"], "local_enqueue");
    } else {
        assert_eq!(envelope["data"]["delivery"], "console_accept");
        assert_eq!(envelope["data"]["consoleHttpStatus"], 200);
    }
    assert_eq!(upstream["explicitPending"], true);
    assert!(upstream["tradeId"].is_null());
    assert_eq!(upstream["preTradeDeclarationId"], decl);
    let draft = upstream["draftText"].as_str().expect("draftText");
    assert!(draft.contains("manual fill"));
    assert!(draft.contains(decl));
    assert!(!draft.contains("\"tradeId\""));
}

#[tokio::test]
#[serial]
async fn manual_fill_links_console_trade_uuid() {
    let console = "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let body = json!({
        "symbol": "PNB",
        "side": "BUY",
        "quantity": 100,
        "price": 95.5,
        "filledAtMs": 1_757_000_000_000_i64,
        "consoleTradeId": console,
        "idempotencyKey": "manual-fill-linked-1",
    });
    let (status, envelope, upstream) = forward_manual_fill(19_462, 19_463, body).await;
    assert!(
        status == StatusCode::OK || status == StatusCode::ACCEPTED,
        "expected 200 or 202, got {status}"
    );
    assert_eq!(upstream["explicitPending"], false);
    assert_eq!(upstream["tradeId"], console);
    assert!(envelope["data"].get("tradeId").is_none() || envelope["data"]["tradeId"].is_null());
    assert_eq!(envelope["data"]["linkStrategy"], "linked");
}
