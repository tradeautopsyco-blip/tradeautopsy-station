//! Dhan consent-login loopback redirect (ADR 0009).
//!
//! `GET …/callback` is unauthenticated (browser redirect). `POST …/connect/begin`
//! is HMAC-protected and triggers consent `generate` before returning the login URL.

use crate::api::AppState;
use crate::ubi::dhan_session::{
    begin_connect, exchange_token_id, take_pending_connect, truncate_state,
    ReqwestDhanSessionHttp,
};
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct DhanConnectBeginBody {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
    #[serde(rename = "dhanClientId")]
    pub dhan_client_id: String,
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "appSecret")]
    pub app_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct DhanCallbackQuery {
    #[serde(default, rename = "tokenId")]
    pub token_id: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

fn dhan_http() -> &'static ReqwestDhanSessionHttp {
    static HTTP: OnceLock<ReqwestDhanSessionHttp> = OnceLock::new();
    HTTP.get_or_init(|| ReqwestDhanSessionHttp::new().expect("dhan session http"))
}

pub async fn connect_begin_handler(
    State(_state): State<AppState>,
    Json(body): Json<DhanConnectBeginBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.broker_slug.trim() != "dhan" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "brokerSlug must be dhan",
            })),
        ));
    }
    // Consent `generate` runs here (ADR 0009): the login URL carries the
    // `consentAppId`, so there is no URL to return before generate succeeds.
    // Blocking by construction (current-thread reqwest runtime) → spawn_blocking.
    let environment = body.environment.trim().to_string();
    let connection_id = body.broker_connection_id.trim().to_string();
    let dhan_client_id = body.dhan_client_id.trim().to_string();
    let app_id = body.app_id.trim().to_string();
    let app_secret = body.app_secret.trim().to_string();
    let begun = tokio::task::spawn_blocking(move || {
        begin_connect(
            dhan_http(),
            &environment,
            &connection_id,
            &dhan_client_id,
            &app_id,
            &app_secret,
        )
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error_class": "upstream",
                "message": "failed to start Dhan connect",
            })),
        )
    })?;
    let (connect_state, login_url) = begun.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": e.class.as_str(),
                "message": "Could not start Dhan consent login. Check the client ID and API credentials, then try again.",
            })),
        )
    })?;
    Ok(Json(json!({
        "ok": true,
        "state": connect_state,
        "loginUrl": login_url,
        // Swift contract (`DhanConnectBeginResponse`): snake_case primary keys.
        // `loginUrl` stays as the documented fallback; `state` is opaque to Swift.
        "consent_login_url": login_url,
        "connection_id": body.broker_connection_id.trim(),
        "redirectUri": crate::ubi::dhan_session::dhan_callback_base_url(),
    })))
}

pub async fn callback_handler(
    State(state): State<AppState>,
    Query(query): Query<DhanCallbackQuery>,
) -> Response {
    if query.error.as_deref().is_some() {
        tracing::warn!(
            "dhan callback error state={}",
            truncate_state(query.state.as_deref().unwrap_or(""))
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Dhan login failed. Return to Station and try Connect again.",
        );
    }
    let Some(state_nonce) = query.state.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        tracing::warn!("dhan callback missing state");
        return connect_html_response(StatusCode::BAD_REQUEST, "Invalid callback (missing state).");
    };
    let Some(token_id) = query
        .token_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        tracing::warn!(
            "dhan callback missing tokenId state={}",
            truncate_state(state_nonce)
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Invalid callback (missing token).",
        );
    };
    let Some(pending) = take_pending_connect(state_nonce) else {
        tracing::warn!(
            "dhan callback stale or unknown state={}",
            truncate_state(state_nonce)
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Connect session expired or already used. Start Connect again from Station.",
        );
    };

    let mint_result = tokio::task::spawn_blocking({
        let dhan_client_id = pending.dhan_client_id.clone();
        let app_id = pending.app_id.clone();
        let app_secret = pending.app_secret.clone();
        let token_id = token_id.to_string();
        move || {
            exchange_token_id(
                dhan_http(),
                &dhan_client_id,
                &app_id,
                &app_secret,
                &token_id,
            )
        }
    })
    .await;

    let minted = match mint_result {
        Ok(Ok(session)) => session,
        Ok(Err(e)) => {
            let class = e.class.as_str();
            tracing::warn!(
                "dhan consume failed class={} state={}",
                class,
                truncate_state(state_nonce)
            );
            let message = if class == "session_expired" {
                "Dhan session expired. Return to Station and connect again."
            } else if class == "rate_limited" {
                "Dhan asked us to slow down. Wait a minute, then connect again from Station."
            } else {
                "Could not complete Dhan login. Return to Station and try again."
            };
            return connect_html_response(StatusCode::BAD_REQUEST, message);
        }
        Err(_) => {
            return connect_html_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error during Dhan login.",
            );
        }
    };

    let blob = minted.into_credential_blob();
    if let Err(e) = state.broker_sync_control.credential_vault().save(
        &pending.environment,
        "dhan",
        &pending.connection_id,
        &blob,
    ) {
        tracing::warn!(
            "dhan session vault save failed: {e} state={}",
            truncate_state(state_nonce)
        );
        return connect_html_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not save session. Return to Station and try Connect again.",
        );
    }

    tracing::info!(
        "dhan connect succeeded state={}",
        truncate_state(state_nonce)
    );
    connect_html_response(
        StatusCode::OK,
        "Dhan login complete. You can return to TradeAutopsy Station.",
    )
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
