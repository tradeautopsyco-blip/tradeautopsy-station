use crate::api::AppState;
use crate::broker_sync_control::BrokerSyncStartRequest;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn start_handler(
    State(state): State<AppState>,
    Json(body): Json<BrokerSyncStartRequest>,
) -> Result<Json<Value>, (axum::http::StatusCode, Json<Value>)> {
    state
        .broker_sync_control
        .start(&body)
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn retry_handler(State(state): State<AppState>) -> Json<Value> {
    let _ = state.broker_sync_control.retry_failed_classes();
    Json(json!({ "ok": true }))
}

pub async fn stop_handler(State(state): State<AppState>) -> Json<Value> {
    let _ = state.broker_sync_control.stop();
    Json(json!({ "ok": true }))
}
