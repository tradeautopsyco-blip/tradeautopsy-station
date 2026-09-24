//! POST `/api/daemon/broker/groww/connect` — key-entry checksum mint (ADR 0014).
//!
//! Writes the minted session into the shared Keychain vault. Response is `{ ok: true }` only.

use crate::api::AppState;
use crate::ubi::groww_session::{connect_mint, ReqwestGrowwSessionHttp};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct GrowwConnectBody {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "apiSecret")]
    pub api_secret: String,
}

fn groww_http() -> &'static ReqwestGrowwSessionHttp {
    static HTTP: OnceLock<ReqwestGrowwSessionHttp> = OnceLock::new();
    HTTP.get_or_init(|| ReqwestGrowwSessionHttp::new().expect("groww session http"))
}

pub async fn connect_handler(
    State(state): State<AppState>,
    Json(body): Json<GrowwConnectBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.broker_slug.trim() != "groww" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "brokerSlug must be groww",
            })),
        ));
    }
    if body.broker_connection_id.trim().is_empty() || body.environment.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "environment and brokerConnectionId are required",
            })),
        ));
    }

    let connection_id_for_log = body.broker_connection_id.trim().to_string();
    let connection_id = connection_id_for_log.clone();
    let api_key = body.api_key;
    let api_secret = body.api_secret;

    let minted = tokio::task::spawn_blocking(move || {
        connect_mint(groww_http(), &connection_id, &api_key, &api_secret)
    })
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "upstream",
                    "message": "groww mint task failed",
                })),
            )
        })?
        .map_err(|e| {
            tracing::warn!(
                "groww connect mint failed class={} connection_id={}",
                e.class.as_str(),
                truncate_connection_id(&connection_id_for_log)
            );
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error_class": e.class.as_str(),
                    "message": e.message,
                })),
            )
        })?;

    let blob = minted.into_credential_blob();
    state
        .broker_sync_control
        .credential_vault()
        .save(
            body.environment.trim(),
            "groww",
            body.broker_connection_id.trim(),
            &blob,
        )
        .map_err(|e| {
            tracing::warn!(
                "groww session vault save failed: {e} connection_id={}",
                truncate_connection_id(&connection_id_for_log)
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "upstream",
                    "message": "groww session vault save failed",
                })),
            )
        })?;

    tracing::info!(
        "groww connect succeeded connection_id={}",
        truncate_connection_id(&connection_id_for_log)
    );

    Ok(Json(json!({ "ok": true })))
}

fn truncate_connection_id(connection_id: &str) -> String {
    let prefix: String = connection_id.chars().take(8).collect();
    format!("{prefix}…")
}
