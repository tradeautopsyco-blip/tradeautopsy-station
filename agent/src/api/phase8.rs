//! Phase 8 — kill-switch ack + OAuth handoff proxies toward hosted daemon routes.

use crate::api::capture::{forward_daemon_json_with_optional_429_retry, upstream_json_response};
use crate::api::AppState;
use crate::AgentEvent;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

pub async fn kill_switch_ack_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "x-user-id missing after wire verification",
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response();
    };

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/daemon/kill-switch/ack",
        user_id,
        request_id,
        Some(&body),
        false,
    )
    .await
    {
        Ok((st, text)) => {
            if st.is_success() {
                state.event_bus.publish(AgentEvent::KillSwitchState {
                    active: false,
                    level: None,
                    countdown_secs: None,
                    requires_ack: false,
                });
            }
            upstream_json_response(st, text)
        }
        Err(msg) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error_class": "SERVER_DOWN",
                "message": msg,
                "retry_after_ms": Value::Null,
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response(),
    }
}

pub async fn auth_begin_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "x-user-id missing after wire verification",
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response();
    };

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/daemon/auth/begin",
        user_id,
        request_id,
        Some(&body),
        false,
    )
    .await
    {
        Ok((st, text)) => upstream_json_response(st, text),
        Err(msg) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error_class": "SERVER_DOWN",
                "message": msg,
                "retry_after_ms": Value::Null,
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response(),
    }
}

pub async fn auth_finish_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "x-user-id missing after wire verification",
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response();
    };

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/daemon/auth/finish",
        user_id,
        request_id,
        Some(&body),
        false,
    )
    .await
    {
        Ok((st, text)) => upstream_json_response(st, text),
        Err(msg) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error_class": "SERVER_DOWN",
                "message": msg,
                "retry_after_ms": Value::Null,
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response(),
    }
}
