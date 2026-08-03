//! POST `/api/daemon/broker/kotak/session/mint` — TOTP+MPIN session mint (SDK-aligned).
//!
//! Writes the minted session into the shared Keychain vault. Response is `{ ok: true }` only —
//! never echoes tokens, TOTP, or MPIN.

use crate::api::AppState;
use crate::ubi::{
    mint_totp_session, KotakMintRequest, ReqwestKotakSessionHttp,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct KotakSessionMintBody {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
    #[serde(rename = "consumerKey")]
    pub consumer_key: String,
    #[serde(rename = "mobileNumber")]
    pub mobile_number: String,
    pub ucc: String,
    pub totp: String,
    pub mpin: String,
}

fn kotak_http() -> &'static ReqwestKotakSessionHttp {
    static HTTP: OnceLock<ReqwestKotakSessionHttp> = OnceLock::new();
    HTTP.get_or_init(|| ReqwestKotakSessionHttp::new().expect("kotak session http"))
}

pub async fn mint_handler(
    State(state): State<AppState>,
    Json(body): Json<KotakSessionMintBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.broker_slug.trim() != "kotak_neo" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "brokerSlug must be kotak_neo",
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

    let request = KotakMintRequest {
        consumer_key: body.consumer_key,
        mobile_number: body.mobile_number,
        ucc: body.ucc,
        totp: body.totp,
        mpin: body.mpin,
    };

    let minted = tokio::task::spawn_blocking(move || mint_totp_session(kotak_http(), &request))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "upstream",
                    "message": "kotak mint task failed",
                })),
            )
        })?
        .map_err(|e| {
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
            "kotak_neo",
            body.broker_connection_id.trim(),
            &blob,
        )
        .map_err(|e| {
            tracing::warn!("kotak session vault save failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "upstream",
                    "message": "failed to persist kotak session",
                })),
            )
        })?;

    Ok(Json(json!({ "ok": true })))
}
