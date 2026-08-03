//! Identity-only vault clear / presence — used for Connect rollback and Delete.
//! Never accepts or returns secret material.

use crate::api::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct BrokerCredentialIdentityBody {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
}

fn validate_identity(body: &BrokerCredentialIdentityBody) -> Result<(), (StatusCode, Json<Value>)> {
    if body.broker_slug.trim().is_empty()
        || body.broker_connection_id.trim().is_empty()
        || body.environment.trim().is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "environment, brokerSlug, and brokerConnectionId are required",
            })),
        ));
    }
    Ok(())
}

/// POST `/api/daemon/broker/credentials/clear` — delete host vault entry for identity.
pub async fn clear_handler(
    State(state): State<AppState>,
    Json(body): Json<BrokerCredentialIdentityBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    validate_identity(&body)?;
    state
        .broker_sync_control
        .credential_vault()
        .delete(
            body.environment.trim(),
            body.broker_slug.trim(),
            body.broker_connection_id.trim(),
        )
        .map_err(|e| {
            tracing::warn!("broker credential vault clear failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "upstream",
                    "message": "failed to clear broker credentials",
                })),
            )
        })?;
    Ok(Json(json!({ "ok": true })))
}

/// POST `/api/daemon/broker/credentials/present` — `{ present: bool }` only.
pub async fn present_handler(
    State(state): State<AppState>,
    Json(body): Json<BrokerCredentialIdentityBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    validate_identity(&body)?;
    let present = state
        .broker_sync_control
        .credential_vault()
        .load(
            body.environment.trim(),
            body.broker_slug.trim(),
            body.broker_connection_id.trim(),
        )
        .map(|opt| opt.is_some())
        .map_err(|e| {
            tracing::warn!("broker credential vault present check failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "upstream",
                    "message": "failed to check broker credentials",
                })),
            )
        })?;
    Ok(Json(json!({ "present": present })))
}
