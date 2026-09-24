//! Zerodha Kite Connect loopback redirect (ADR 0005).
//!
//! `GET …/callback` is unauthenticated (browser redirect). `POST …/connect/begin` is HMAC-protected.

use crate::api::AppState;
use crate::ubi::{
    begin_connect, exchange_request_token, take_pending_connect, take_single_active_pending_connect,
    ReqwestKiteSessionHttp,
};
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct ZerodhaConnectBeginBody {
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

#[derive(Debug, Deserialize)]
pub struct ZerodhaCallbackQuery {
    #[serde(default)]
    pub request_token: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

fn kite_http() -> &'static ReqwestKiteSessionHttp {
    static HTTP: OnceLock<ReqwestKiteSessionHttp> = OnceLock::new();
    HTTP.get_or_init(|| ReqwestKiteSessionHttp::new().expect("kite session http"))
}

pub async fn connect_begin_handler(
    State(_state): State<AppState>,
    Json(body): Json<ZerodhaConnectBeginBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.broker_slug.trim() != "zerodha_kite" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "brokerSlug must be zerodha_kite",
            })),
        ));
    }
    let (connect_state, login_url) = begin_connect(
        body.environment.trim(),
        body.broker_connection_id.trim(),
        body.api_key.trim(),
        body.api_secret.trim(),
    )
    .map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": message,
            })),
        )
    })?;
    Ok(Json(json!({
        "ok": true,
        "state": connect_state,
        "loginUrl": login_url,
        "redirectUri": crate::ubi::zerodha_callback_base_url(),
    })))
}

pub async fn callback_handler(
    State(state): State<AppState>,
    Query(query): Query<ZerodhaCallbackQuery>,
) -> Response {
    if query.status.as_deref() == Some("error") {
        tracing::warn!(
            "zerodha callback status=error state={}",
            truncate_state(query.state.as_deref().unwrap_or(""))
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Kite login failed. Return to Station and try Connect again.",
        );
    }
    let Some(request_token) = query
        .request_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        tracing::warn!("zerodha callback missing request_token");
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Invalid callback (missing request token).",
        );
    };

    let state_nonce = query.state.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let pending = if let Some(state_nonce) = state_nonce {
        match take_pending_connect(state_nonce) {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "zerodha callback stale or unknown state={}",
                    truncate_state(state_nonce)
                );
                return connect_html_response(
                    StatusCode::BAD_REQUEST,
                    "Connect session expired or already used. Start Connect again from Station.",
                );
            }
        }
    } else {
        tracing::warn!(
            "zerodha callback missing state — using single active connect fallback (Kite redirect)"
        );
        match take_single_active_pending_connect() {
            Some(p) => p,
            None => {
                return connect_html_response(
                    StatusCode::BAD_REQUEST,
                    "Invalid callback (missing state). Start Connect again from Station.",
                );
            }
        }
    };
    let state_log = state_nonce
        .map(truncate_state)
        .unwrap_or_else(|| "fallback".into());

    let mint_result = tokio::task::spawn_blocking({
        let api_key = pending.api_key.clone();
        let api_secret = pending.api_secret.clone();
        let request_token = request_token.to_string();
        move || {
            exchange_request_token(
                kite_http(),
                &api_key,
                &api_secret,
                &request_token,
                chrono::Utc::now().timestamp_millis(),
            )
        }
    })
    .await;

    let minted = match mint_result {
        Ok(Ok(session)) => session,
        Ok(Err(e)) => {
            tracing::warn!(
                "zerodha token exchange failed class={} state={}",
                e.class.as_str(),
                state_log
            );
            return connect_html_response(
                StatusCode::BAD_REQUEST,
                "Could not complete Kite login. Return to Station and try again.",
            );
        }
        Err(_) => {
            return connect_html_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error during Kite login.",
            );
        }
    };

    let blob = minted.into_credential_blob();
    if let Err(e) = state.broker_sync_control.credential_vault().save(
        &pending.environment,
        "zerodha_kite",
        &pending.connection_id,
        &blob,
    ) {
        tracing::warn!("zerodha session vault save failed: {e} state={}", state_log);
        return connect_html_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not save session. Return to Station and try Connect again.",
        );
    }

    tracing::info!("zerodha connect succeeded state={}", state_log);
    connect_html_response(
        StatusCode::OK,
        "Kite login complete. You can return to TradeAutopsy Station.",
    )
}

fn truncate_state(state: &str) -> String {
    let prefix: String = state.chars().take(8).collect();
    format!("{prefix}…")
}

fn connect_html_response(status: StatusCode, message: &str) -> Response {
    let body = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>TradeAutopsy Station</title></head><body><p>{message}</p></body></html>"
    );
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body),
    )
        .into_response()
}
