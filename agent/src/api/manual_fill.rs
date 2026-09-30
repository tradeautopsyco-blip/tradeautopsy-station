use crate::{
    api::capture::{
        accept_capture_value, forward_daemon_json_with_optional_429_retry, upstream_json_response,
        validation_error,
    },
    api::AppState,
    event_bus::AgentEvent,
    journal_manual_fill::{
        build_manual_fill_capture_body, manual_fill_link_strategy, persist_manual_fill,
        ManualFillAcceptBody, ManualFillValidationError, JOURNAL_FILL_SOURCE_MANUAL,
    },
};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::SecondsFormat;
use serde_json::json;

pub async fn accept_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ManualFillAcceptBody>,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    let Some(user_id) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) else {
        return validation_error(
            "x-user-id (wire hint) missing after wire verification",
            request_id,
        );
    };

    let persisted = match persist_manual_fill(&body) {
        Ok(p) => p,
        Err(ManualFillValidationError::Message(msg)) => {
            return validation_error(&msg, request_id);
        }
    };

    if let Err(err) = state.recent_trades.upsert_fill(&persisted.broker_fill) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error_class": "STORAGE",
                "message": format!("failed to store manual fill: {err}"),
                "retry_after_ms": serde_json::Value::Null,
                "request_id": request_id.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
            })),
        )
            .into_response();
    }

    let filled_at = persisted
        .broker_fill
        .filled_at
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    state.event_bus.publish(AgentEvent::ToolbarShow {
        trigger: JOURNAL_FILL_SOURCE_MANUAL.to_string(),
        detection_id: ulid::Ulid::new().to_string(),
        trade_id: persisted.trade_id.clone(),
        symbol: persisted.broker_fill.symbol.clone(),
        side: persisted.broker_fill.side.clone(),
        qty: persisted.broker_fill.qty,
        price: persisted.broker_fill.price,
        filled_at,
        broker: persisted.broker_fill.broker.clone(),
    });

    let capture_body = build_manual_fill_capture_body(&persisted, &body);
    let link = manual_fill_link_strategy(body.console_trade_id.as_deref());
    let mut capture_response =
        accept_capture_value(&state, user_id, request_id, capture_body).await;

    let status = capture_response.status();
    let bytes = std::mem::take(capture_response.body_mut());
    let raw = axum::body::to_bytes(bytes, 65_536)
        .await
        .unwrap_or_default();
    let envelope = serde_json::from_slice::<serde_json::Value>(&raw).unwrap_or(json!({}));
    let (out_status, out_body) =
        reshape_manual_fill_response(status, envelope, &persisted.fill_id, link.explicit_pending);
    (out_status, Json(out_body)).into_response()
}

/// GET Console `trades` for “Link to today’s trade” (daemon recent-trades, not local fills).
pub async fn console_recent_trades_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let request_id = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::GET,
        "/api/daemon/journal/toolbar/recent-trades",
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
                "retry_after_ms": serde_json::Value::Null,
                "request_id": request_id.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
            })),
        )
            .into_response(),
    }
}

/// Notch-facing status: local 202 enqueue vs Console 2xx vs Console 400 `error.code`.
/// Does not put the Station-local fill id in `tradeId`.
pub(crate) fn reshape_manual_fill_response(
    status: StatusCode,
    mut envelope: serde_json::Value,
    local_fill_id: &str,
    explicit_pending: bool,
) -> (StatusCode, serde_json::Value) {
    if status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST {
        let code = envelope
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("VALIDATION_ERROR");
        return (
            StatusCode::BAD_REQUEST,
            json!({
                "success": false,
                "error": {
                    "code": code,
                    "message": "Capture rules not satisfied"
                },
                "consoleHttpStatus": 400
            }),
        );
    }

    if let Some(data) = envelope.get_mut("data").and_then(|d| d.as_object_mut()) {
        data.remove("tradeId");
        data.insert("localFillId".into(), json!(local_fill_id));
        data.insert(
            "journalFillSource".into(),
            json!(JOURNAL_FILL_SOURCE_MANUAL),
        );
        data.insert(
            "linkStrategy".into(),
            json!(if explicit_pending {
                "pending"
            } else {
                "linked"
            }),
        );
        if status == StatusCode::ACCEPTED {
            data.insert("delivery".into(), json!("local_enqueue"));
            data.insert("consoleHttpStatus".into(), serde_json::Value::Null);
        } else if status.is_success() {
            data.insert("delivery".into(), json!("console_accept"));
            data.insert("consoleHttpStatus".into(), json!(status.as_u16()));
        }
    }

    (status, envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reshape_keeps_local_enqueue_distinct_from_console_accept() {
        let queued = json!({
            "success": true,
            "data": { "status": "queued" }
        });
        let (st, body) =
            reshape_manual_fill_response(StatusCode::ACCEPTED, queued, "manual-local", true);
        assert_eq!(st, StatusCode::ACCEPTED);
        assert_eq!(body["data"]["delivery"], "local_enqueue");
        assert!(body["data"]["consoleHttpStatus"].is_null());
        assert!(body["data"].get("tradeId").is_none());
        assert_eq!(body["data"]["linkStrategy"], "pending");

        let accepted = json!({
            "success": true,
            "data": {
                "pending_capture_id": "11111111-2222-4333-8444-555555555555",
                "status": "accepted"
            }
        });
        let (st, body) =
            reshape_manual_fill_response(StatusCode::OK, accepted, "manual-local", false);
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["data"]["delivery"], "console_accept");
        assert_eq!(body["data"]["consoleHttpStatus"], 200);
        assert_eq!(body["data"]["linkStrategy"], "linked");
        assert!(body["data"].get("tradeId").is_none());
    }

    #[test]
    fn reshape_surfaces_console_400_error_code() {
        let rejected = json!({
            "error_class": "VALIDATION",
            "message": "validation",
            "error": { "code": "VALIDATION_ERROR" }
        });
        let (st, body) = reshape_manual_fill_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            rejected,
            "manual-local",
            true,
        );
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
        assert_eq!(body["consoleHttpStatus"], 400);
    }
}
