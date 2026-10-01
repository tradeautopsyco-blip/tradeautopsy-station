//! Bar v1 — forward Notch calls to hosted `/api/bar/v1/*` (Station Bearer identity).

use crate::api::capture::{forward_daemon_json_with_optional_429_retry, upstream_json_response};
use crate::api::AppState;
use crate::live_book::LiveBookEvent;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

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

fn declarations_upstream_path(params: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(scope) = params.get("scope") {
        if matches!(scope.as_str(), "week" | "pending" | "recent") {
            parts.push(format!("scope={scope}"));
        }
    }
    if let Some(limit) = params.get("limit") {
        if !limit.is_empty() && limit.len() <= 4 && limit.chars().all(|c| c.is_ascii_digit()) {
            parts.push(format!("limit={limit}"));
        }
    }
    if parts.is_empty() {
        "/api/bar/v1/declarations".to_string()
    } else {
        format!("/api/bar/v1/declarations?{}", parts.join("&"))
    }
}

/// GET week/pending/recent declarations — Journal consume-only (no declare POST).
pub async fn declarations_list_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let upstream_path = declarations_upstream_path(&params);

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::GET,
        &upstream_path,
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

pub async fn declare_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let local_id = uuid::Uuid::new_v4().to_string();
    state
        .live_book
        .apply(LiveBookEvent::declare_from_body(&body, local_id.clone()));

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
        Ok((st, text)) if st.is_success() => {
            if let Some(server_id) = declaration_id_from_body(&text) {
                state.live_book.apply(LiveBookEvent::ReconcileArchive {
                    local_id,
                    server_id,
                });
            }
            upstream_json_response(st, text)
        }
        Ok((st, text)) if st.as_u16() == 401 => {
            archive_kept_response(local_id, request_id, st.as_u16(), Some(&text))
        }
        Ok((st, text)) if st.is_client_error() => {
            state.live_book.apply(LiveBookEvent::Cancel {
                declaration_id: local_id,
            });
            upstream_json_response(st, text)
        }
        Ok((st, text)) => archive_kept_response(local_id, request_id, st.as_u16(), Some(&text)),
        Err(msg) => archive_kept_response(local_id, request_id, 502, Some(&msg)),
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
        Ok((st, text)) if st.is_success() => {
            state.live_book.apply(LiveBookEvent::Cancel {
                declaration_id: declaration_id.to_string(),
            });
            upstream_json_response(st, text)
        }
        Ok((st, text)) if st.as_u16() == 409 => upstream_json_response(st, text),
        Ok((st, text)) if st.is_client_error() => upstream_json_response(st, text),
        Ok((st, text)) => {
            state.live_book.apply(LiveBookEvent::Cancel {
                declaration_id: declaration_id.to_string(),
            });
            archive_kept_response(
                declaration_id.to_string(),
                request_id,
                st.as_u16(),
                Some(&text),
            )
        }
        Err(msg) => {
            state.live_book.apply(LiveBookEvent::Cancel {
                declaration_id: declaration_id.to_string(),
            });
            archive_kept_response(declaration_id.to_string(), request_id, 502, Some(&msg))
        }
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
    let declaration_id = body
        .get("declaration_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let stop_loss = body
        .get("stop_loss")
        .and_then(Value::as_f64)
        .or_else(|| body.get("sl_price").and_then(Value::as_f64));
    state.live_book.apply(LiveBookEvent::Protective {
        declaration_id: declaration_id.clone(),
        stop_loss,
    });

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
        Ok((st, text)) if st.is_success() => upstream_json_response(st, text),
        Ok((st, text)) if st.is_client_error() => upstream_json_response(st, text),
        Ok((st, text)) => archive_kept_response(
            declaration_id.unwrap_or_default(),
            request_id,
            st.as_u16(),
            Some(&text),
        ),
        Err(msg) => archive_kept_response(
            declaration_id.unwrap_or_default(),
            request_id,
            502,
            Some(&msg),
        ),
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
pub async fn loss_limits_get_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
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

/// Wave 2 — freeze Working last vs invalidation on `plan_snapshot.condition_at_close` (local LiveBook only).
pub async fn capture_working_condition_handler(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let last = body.get("last").and_then(Value::as_f64);
    let last_status = body
        .get("last_status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
        .trim()
        .to_string();
    state.live_book.apply(LiveBookEvent::CaptureWorkingCondition {
        last,
        last_status,
    });
    if let Some(book) = state.live_book.snapshot() {
        return livebook_json("local", book);
    }
    (
        StatusCode::OK,
        Json(json!({ "ok": true })),
    )
        .into_response()
}

/// POST test-only matched fill inject — forward to Console internal route (env-gated on brain).
pub async fn test_fill_matched_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/internal/bar/v1/test/fill-matched",
        request_id,
        Some(&body),
        false,
    )
    .await
    {
        Ok((st, text)) => {
            if st.is_success() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    state
                        .today_service
                        .try_record_console_test_fill_matched_cite(&body, &parsed);
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

fn declaration_id_from_body(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    v.get("declarationId")
        .or_else(|| v.get("declaration_id"))
        .or_else(|| v.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn archive_kept_response(
    declaration_id: String,
    request_id: Option<&str>,
    archive_status: u16,
    detail: Option<&str>,
) -> Response {
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "declarationId": declaration_id,
            "archive_error": {
                "status": archive_status,
                "message": detail.unwrap_or("console archive failed"),
            },
            "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
        })),
    )
        .into_response()
}
