//! Bar v1 — forward Notch calls to hosted `/api/bar/v1/*` (Station Bearer identity).

use crate::api::capture::{forward_daemon_json_with_optional_429_retry, upstream_json_response};
use crate::api::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

fn livebook_json(source: &'static str, book: Value) -> Response {
    (
        StatusCode::OK,
        [(
            axum::http::header::HeaderName::from_static("x-livebook"),
            source,
        )],
        Json(book),
    )
        .into_response()
}

pub async fn live_state_handler(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    if let Some(book) = state.live_book.snapshot() {
        return livebook_json("local", book);
    }

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::GET,
        "/api/bar/v1/live-state",
        request_id,
        None,
        false,
    )
    .await
    {
        Ok((st, text)) => {
            if st.is_success() {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    state.live_book.hydrate(value.clone());
                    return livebook_json("snapshot", value);
                }
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

pub async fn declare_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/declarations",
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

pub async fn cancel_declaration_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    let declaration_id = body
        .get("declaration_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(declaration_id) = declaration_id else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "declaration_id required",
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response();
    };
    if uuid::Uuid::parse_str(declaration_id).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "declaration_id must be a UUID",
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response();
    };

    let chip = body
        .get("cancel_reason_chip")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(chip) = chip else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "cancel_reason_chip required",
                "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
            })),
        )
            .into_response();
    };

    let upstream_path = format!("/api/bar/v1/declarations/{declaration_id}/cancel");
    let upstream_body = json!({ "cancel_reason_chip": chip });

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        &upstream_path,
        request_id,
        Some(&upstream_body),
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

pub async fn stop_me_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/declarations/stop-me",
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

pub async fn stop_me_clear_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/declarations/stop-me/clear",
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

pub async fn live_interference_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/notch/live-interference",
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

pub async fn protective_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/protective",
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

pub async fn swing_check_in_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/notch/swing-check-in",
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

pub async fn post_trade_debrief_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::PATCH,
        "/api/bar/v1/post-trade-debrief",
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

/// GET loss limits + bar activation gate — forward to Console `/api/bar/v1/profile/loss-limits`.
pub async fn loss_limits_get_handler(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::GET,
        "/api/bar/v1/profile/loss-limits",
        request_id,
        None,
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

/// POST loss limits acknowledgement — forward to Console `/api/bar/v1/profile/loss-limits`.
pub async fn loss_limits_post_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/bar/v1/profile/loss-limits",
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
