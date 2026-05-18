use crate::{api::AppState, queued_response_json, ProcessNowResult, UpstreamClient};
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::header;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ToolbarCaptureAcceptBody {
    draft_text: String,
    #[serde(default)]
    trade_id: Option<String>,
    #[serde(default)]
    explicit_pending: Option<bool>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    r2_key: Option<String>,
}

fn validation_error(message: &str, request_id: Option<&str>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error_class": "VALIDATION",
            "message": message,
            "retry_after_ms": Value::Null,
            "request_id": request_id.map(Value::from).unwrap_or(Value::Null),
        })),
    )
        .into_response()
}

pub async fn accept_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(raw_body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let body: ToolbarCaptureAcceptBody = match serde_json::from_value(raw_body.clone()) {
        Ok(v) => v,
        Err(_) => return validation_error("request body failed validation", request_id),
    };
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return validation_error("x-user-id missing after wire verification", request_id);
    };

    if body
        .trade_id
        .as_ref()
        .is_some_and(|s| uuid::Uuid::parse_str(s).is_err())
    {
        return validation_error("tradeId must be UUID", request_id);
    }
    if body.idempotency_key.as_ref().is_some_and(|s| s.len() > 256) {
        return validation_error("idempotencyKey too long", request_id);
    }
    if body.r2_key.as_ref().is_some_and(|s| s.len() > 512) {
        return validation_error("r2Key too long", request_id);
    }

    let request_id = request_id
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| ulid::Ulid::new().to_string());
    let enqueue_id = match state
        .outbox
        .enqueue_capture(user_id, &request_id, raw_body)
        .await
    {
        Ok(id) => id,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "OUTBOX",
                    "message": format!("failed to enqueue capture: {err}"),
                    "retry_after_ms": Value::Null,
                    "request_id": request_id,
                })),
            )
                .into_response();
        }
    };

    match state.outbox.process_now(enqueue_id).await {
        Ok(ProcessNowResult::Acked { response_body }) => {
            (StatusCode::OK, response_body).into_response()
        }
        Ok(ProcessNowResult::Queued) => {
            (StatusCode::ACCEPTED, Json(queued_response_json())).into_response()
        }
        Ok(ProcessNowResult::DeadLetter { reason }) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error_class": "VALIDATION",
                "message": reason,
                "retry_after_ms": Value::Null,
                "request_id": request_id,
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error_class": "OUTBOX",
                "message": format!("delivery failure: {err}"),
                "retry_after_ms": Value::Null,
                "request_id": request_id,
            })),
        )
            .into_response(),
    }
}

fn parse_retry_after_ms(resp: &reqwest::Response) -> Option<u64> {
    let h = resp.headers().get(reqwest::header::RETRY_AFTER)?;
    let s = h.to_str().ok()?;
    let secs: u64 = s.parse().ok()?;
    Some(secs.saturating_mul(1000))
}

pub(crate) fn upstream_json_response(status: reqwest::StatusCode, text: String) -> Response {
    let axum_status = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    Response::builder()
        .status(axum_status)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )
        .body(Body::from(text))
        .unwrap()
        .into_response()
}

pub(crate) async fn forward_daemon_json_with_optional_429_retry(
    upstream: &UpstreamClient,
    method: reqwest::Method,
    path: &str,
    user_id: &str,
    bar_request_id: Option<&str>,
    body: Option<&Value>,
    retry_on_429: bool,
) -> Result<(reqwest::StatusCode, String), String> {
    let url = format!("{}{}", upstream.config.base_url, path);
    const MAX_ATTEMPTS: u32 = 4;
    let mut attempt = 0u32;
    let request_id_owned = bar_request_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| ulid::Ulid::new().to_string());
    loop {
        let req = upstream
            .http
            .request(method.clone(), &url)
            .header(
                reqwest::header::HeaderName::from_static("x-daemon-secret"),
                upstream.config.daemon_secret.as_str(),
            )
            .header("x-user-id", user_id)
            .header("x-request-id", request_id_owned.as_str());
        let req = if let Some(b) = body {
            req.json(b)
        } else {
            req
        };
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if retry_on_429
            && status == reqwest::StatusCode::TOO_MANY_REQUESTS
            && attempt + 1 < MAX_ATTEMPTS
        {
            let wait_ms = parse_retry_after_ms(&resp).unwrap_or(120 * (1u64 << attempt));
            let _ = resp.text().await;
            tokio::time::sleep(Duration::from_millis(wait_ms.min(30_000))).await;
            attempt += 1;
            continue;
        }
        let text = resp.text().await.unwrap_or_default();
        return Ok((status, text));
    }
}

/// Phase 7: bar → agent → hosted presign (429 backoff toward upstream).
pub async fn screenshot_presign_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return validation_error("x-user-id missing after wire verification", request_id);
    };

    let Some(pc) = body.get("pending_capture_id").and_then(|v| v.as_str()) else {
        return validation_error("pending_capture_id required", request_id);
    };
    if uuid::Uuid::parse_str(pc).is_err() {
        return validation_error("pending_capture_id must be UUID", request_id);
    }
    let Some(ct) = body.get("content_type").and_then(|v| v.as_str()) else {
        return validation_error("content_type required", request_id);
    };
    if ct.is_empty() || ct.len() > 64 {
        return validation_error("content_type invalid", request_id);
    }

    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/daemon/screenshot/presign",
        user_id,
        request_id,
        Some(&body),
        true,
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

/// Phase 7: bar → agent → hosted PATCH pending capture (screenshot r2_key).
pub async fn pending_patch_handler(
    State(state): State<AppState>,
    Path(pending_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return validation_error("x-user-id missing after wire verification", request_id);
    };
    if uuid::Uuid::parse_str(&pending_id).is_err() {
        return validation_error("pending id must be UUID", request_id);
    }

    let path = format!("/api/daemon/journal/toolbar-capture/pending/{pending_id}");
    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::PATCH,
        &path,
        user_id,
        request_id,
        Some(&body),
        true,
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
