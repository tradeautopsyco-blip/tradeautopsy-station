//! Wave 7 — journal N2 overlay + condition fire log.

mod common;

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use common::{
    apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, WireHeaderOverrides,
};
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};
use std::time::Duration;

#[tokio::test]
async fn condition_fire_persists_and_enriches_week_declarations() {
    const AGENT_PORT: u16 = 39_720;
    let upstream = Router::new().route(
        "/api/bar/v1/declarations",
        get(|| async {
            (
                StatusCode::OK,
                Json(json!({
                    "items": [{
                        "id": "decl-n2-1",
                        "status": "matched",
                        "symbol": "RELIANCE",
                        "snapshot": { "setup_label": "Pullback" }
                    }]
                })),
            )
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

    let fire_path = "/api/daemon/journal/condition-fire";
    let fire_url = format!("http://127.0.0.1:{AGENT_PORT}{fire_path}");
    let fire_body = json!({
        "declaration_id": "decl-n2-1",
        "rule_id": "invalidation_price",
        "fired_at_ms": 1_700_000_000_000i64,
        "working": { "invalidation_state": "breached" }
    });
    let fire_bytes = serde_json::to_vec(&fire_body).expect("json");
    let fire_resp = apply_wire_v1(
        client()
            .post(&fire_url)
            .header(CONTENT_TYPE, "application/json")
            .body(fire_bytes.clone()),
        "POST",
        fire_path,
        &fire_bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("fire");
    assert_eq!(fire_resp.status(), 200);

    let list_path = "/api/daemon/bar/declarations";
    let list_url = format!("http://127.0.0.1:{AGENT_PORT}{list_path}?scope=week");
    let list_resp = apply_wire_v1(
        client().get(&list_url),
        "GET",
        list_path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("list");
    assert_eq!(list_resp.status(), 200);
    let out: Value = list_resp.json().await.expect("json");
    let sheet = &out["items"][0]["n2_day_sheet"];
    assert_eq!(sheet["condition_fires"][0]["rule_id"], "invalidation_price");
    assert_eq!(sheet["working"]["invalidation_state"], "breached");
    assert_eq!(sheet["plan"]["setup_label"], "Pullback");

    handle.abort();
}

#[tokio::test]
async fn post_trade_debrief_upserts_n2_moment_c_for_week_list() {
    const AGENT_PORT: u16 = 39_721;
    let upstream = Router::new()
        .route(
            "/api/bar/v1/post-trade-debrief",
            post(|Json(_): Json<Value>| async move { (StatusCode::OK, Json(json!({ "ok": true }))) }),
        )
        .route(
            "/api/bar/v1/declarations",
            get(|| async {
                (
                    StatusCode::OK,
                    Json(json!({
                        "items": [{
                            "id": "decl-debrief-1",
                            "status": "matched",
                            "symbol": "TCS",
                            "notes": { "post": "" }
                        }]
                    })),
                )
            }),
        );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind");
    let upstream_port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
        axum::serve(listener, upstream)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(Duration::from_millis(60)).await;

    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{upstream_port}"));
    let handle = spawn_test_agent_with_options(AGENT_PORT, opts);
    tokio::time::sleep(Duration::from_millis(320)).await;

    let debrief_path = "/api/daemon/bar/post-trade-debrief";
    let debrief_url = format!("http://127.0.0.1:{AGENT_PORT}{debrief_path}");
    let body = json!({
        "declaration_id": "decl-debrief-1",
        "moment_a_note": "Gap down",
        "moment_c_note": "",
        "adherence": { "stop_as_declared": true },
        "completed_at_ms": 1_700_000_000_100i64
    });
    let bytes = serde_json::to_vec(&body).expect("json");
    let resp = apply_wire_v1(
        client()
            .patch(&debrief_url)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes.clone()),
        "PATCH",
        debrief_path,
        &bytes,
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("patch");
    assert_eq!(resp.status(), 200);

    let list_path = "/api/daemon/bar/declarations";
    let list_url = format!("http://127.0.0.1:{AGENT_PORT}{list_path}?scope=week");
    let list_resp = apply_wire_v1(
        client().get(&list_url),
        "GET",
        list_path,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("list");
    let out: Value = list_resp.json().await.expect("json");
    let moment_c = out["items"][0]["n2_day_sheet"]["debrief"]["moment_c_note"]
        .as_str()
        .unwrap_or("missing");
    assert_eq!(moment_c, "");

    handle.abort();
}
