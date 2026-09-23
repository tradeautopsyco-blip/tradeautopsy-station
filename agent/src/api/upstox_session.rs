//! Upstox OAuth loopback redirect (ADR 0006).
//!
//! `GET …/callback` is unauthenticated (browser redirect). `POST …/begin` is HMAC-protected.

use crate::api::AppState;
use crate::ubi::upstox_session::{
    begin_connect, exchange_auth_code, take_pending_connect, ReqwestUpstoxSessionHttp,
    UpstoxExchangeErrorClass,
};
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct UpstoxConnectBeginBody {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "clientSecret")]
    pub client_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct UpstoxCallbackQuery {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

fn upstox_http() -> &'static ReqwestUpstoxSessionHttp {
    static HTTP: OnceLock<ReqwestUpstoxSessionHttp> = OnceLock::new();
    HTTP.get_or_init(|| ReqwestUpstoxSessionHttp::new().expect("upstox session http"))
}

pub async fn connect_begin_handler(
    State(_state): State<AppState>,
    Json(body): Json<UpstoxConnectBeginBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.broker_slug.trim() != "upstox" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error_class": "invalid_credentials",
                "message": "brokerSlug must be upstox",
            })),
        ));
    }
    let (connect_state, login_url) = begin_connect(
        body.environment.trim(),
        body.broker_connection_id.trim(),
        body.client_id.trim(),
        body.client_secret.trim(),
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
        "redirectUri": crate::ubi::upstox_session::upstox_callback_base_url(),
    })))
}

pub async fn callback_handler(
    State(state): State<AppState>,
    Query(query): Query<UpstoxCallbackQuery>,
) -> Response {
    if query.error.as_deref().is_some() {
        tracing::warn!(
            "upstox callback error={} state={}",
            query.error.as_deref().unwrap_or(""),
            truncate_state(query.state.as_deref().unwrap_or(""))
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Upstox login failed. Return to Station and try Connect again.",
        );
    }
    let Some(state_nonce) = query.state.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        tracing::warn!("upstox callback missing state");
        return connect_html_response(StatusCode::BAD_REQUEST, "Invalid callback (missing state).");
    };
    let Some(code) = query.code.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        tracing::warn!(
            "upstox callback missing code state={}",
            truncate_state(state_nonce)
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Invalid callback (missing authorization code).",
        );
    };
    let Some(pending) = take_pending_connect(state_nonce) else {
        tracing::warn!(
            "upstox callback stale or unknown state={}",
            truncate_state(state_nonce)
        );
        return connect_html_response(
            StatusCode::BAD_REQUEST,
            "Connect session expired or already used. Start Connect again from Station.",
        );
    };

    let mint_result = tokio::task::spawn_blocking({
        let client_id = pending.client_id.clone();
        let client_secret = pending.client_secret.clone();
        let redirect_uri = pending.redirect_uri.clone();
        let code = code.to_string();
        move || {
            exchange_auth_code(
                upstox_http(),
                &client_id,
                &client_secret,
                &redirect_uri,
                &code,
                chrono::Utc::now().timestamp_millis(),
            )
        }
    })
    .await;

    let minted = match mint_result {
        Ok(Ok(session)) => session,
        Ok(Err(e)) => {
            let class = e.class;
            tracing::warn!(
                "upstox token exchange failed class={} state={}",
                class.as_str(),
                truncate_state(state_nonce)
            );
            let message = if class == UpstoxExchangeErrorClass::SessionExpired {
                "Upstox session expired. Return to Station and connect again."
            } else {
                "Could not complete Upstox login. Return to Station and try again."
            };
            return connect_html_response(StatusCode::BAD_REQUEST, message);
        }
        Err(_) => {
            return connect_html_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error during Upstox login.",
            );
        }
    };

    let blob = minted.into_credential_blob();
    if let Err(e) = state.broker_sync_control.credential_vault().save(
        &pending.environment,
        "upstox",
        &pending.connection_id,
        &blob,
    ) {
        tracing::warn!(
            "upstox session vault save failed: {e} state={}",
            truncate_state(state_nonce)
        );
        return connect_html_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not save session. Return to Station and try Connect again.",
        );
    }

    tracing::info!(
        "upstox connect succeeded state={}",
        truncate_state(state_nonce)
    );
    connect_html_response(
        StatusCode::OK,
        "Upstox login complete. You can return to TradeAutopsy Station.",
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
