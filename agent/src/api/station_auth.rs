//! Station device-login loopback API (A8 II → Exit II proof).
//!
//! `device_code` stays in agent memory only. Responses expose `user_code` / session identity.

use crate::api::AppState;
use crate::device_login::{
    begin_device_login, complete_device_login, prove_station_session,
    DeviceLoginPublic, StationRefreshError, StationSessionIdentity,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

fn anyhow_message(err: &anyhow::Error) -> String {
    err.chain()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(": ")
}

/// Public AuthKit client id shipped with Station. Env overrides for local WorkOS apps.
const SHIPPED_WORKOS_STATION_CLIENT_ID: &str = "client_01KBEHG7XWN269N97M1EAKXV07";

fn workos_station_client_id() -> Result<String, String> {
    let from_env = std::env::var("WORKOS_STATION_CLIENT_ID")
        .unwrap_or_default()
        .trim()
        .to_string();
    let id = if from_env.is_empty() {
        SHIPPED_WORKOS_STATION_CLIENT_ID.to_string()
    } else {
        from_env
    };
    if id.is_empty() {
        Err("WORKOS_STATION_CLIENT_ID is empty".to_string())
    } else {
        Ok(id)
    }
}

pub async fn station_auth_begin_handler(State(state): State<AppState>) -> Response {
    let client_id = match workos_station_client_id() {
        Ok(id) => id,
        Err(msg) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error_class": "CONFIG",
                    "message": msg,
                })),
            )
                .into_response();
        }
    };

    match begin_device_login(&state.upstream.http, &client_id).await {
        Ok(pending) => {
            let public = pending.public.clone();
            *state
                .device_login_pending
                .lock()
                .expect("device login pending mutex") = Some(pending);
            let console_base = state.upstream.config.base_url.as_str();
            (StatusCode::OK, Json(public_to_json(&public, console_base))).into_response()
        }
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error_class": "UPSTREAM",
                "message": anyhow_message(&err),
            })),
        )
            .into_response(),
    }
}

pub async fn station_auth_complete_handler(State(state): State<AppState>) -> Response {
    let pending = {
        let mut slot = state
            .device_login_pending
            .lock()
            .expect("device login pending mutex");
        slot.take()
    };
    let Some(pending) = pending else {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "error_class": "VALIDATION",
                "message": "No pending device login — call begin first",
            })),
        )
            .into_response();
    };

    let store = state.station_token_store.clone();
    match complete_device_login(
        &state.upstream.http,
        &pending,
        &state.upstream.config.base_url,
        store.as_ref(),
    )
    .await
    {
        Ok(tokens) => match prove_station_session(
            &state.upstream.http,
            &state.upstream.config.base_url,
            &tokens.access_token,
        )
        .await
        {
            Ok(identity) => {
                let outbox = state.fact_outbox.clone();
                tokio::spawn(async move {
                    let _ = outbox.flush_station_presence().await;
                });
                (StatusCode::OK, Json(identity_to_json(&identity))).into_response()
            }
            Err(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error_class": "SESSION_PROOF",
                    "message": anyhow_message(&err),
                })),
            )
                .into_response(),
        },
        Err(err) => {
            // Restore pending so the UI can retry complete without re-begin
            // when WorkOS is still authorization_pending-style failures that
            // bubbled as expiry/errors after a long poll — only restore on
            // non-terminal? For simplicity restore always except we already
            // took it; restore so a second complete can retry if mint failed.
            *state
                .device_login_pending
                .lock()
                .expect("device login pending mutex") = Some(pending);
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error_class": "UPSTREAM",
                    "message": anyhow_message(&err),
                })),
            )
                .into_response()
        }
    }
}

pub async fn station_auth_session_handler(State(state): State<AppState>) -> Response {
    if state.station_token_store.load().ok().flatten().is_none() {
        return (StatusCode::OK, Json(json!({ "signed_in": false }))).into_response();
    }

    if !state.upstream.config.is_loopback_http_bootstrap() {
        match state.upstream.ensure_fresh_station_access(None).await {
            Err(StationRefreshError::Revoked) => {
                return (StatusCode::OK, Json(json!({ "signed_in": false }))).into_response();
            }
            Ok(()) | Err(StationRefreshError::Transient(_)) => {}
        }
    }

    let tokens = match state.station_token_store.load() {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (StatusCode::OK, Json(json!({ "signed_in": false }))).into_response();
        }
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error_class": "KEYCHAIN",
                    "message": anyhow_message(&err),
                })),
            )
                .into_response();
        }
    };

    match prove_station_session(
        &state.upstream.http,
        &state.upstream.config.base_url,
        &tokens.access_token,
    )
    .await
    {
        Ok(identity) => {
            let mut body = identity_to_json(&identity);
            if let Some(obj) = body.as_object_mut() {
                obj.insert("signed_in".into(), Value::Bool(true));
            }
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(err) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "signed_in": false,
                "error_class": "SESSION_PROOF",
                "message": anyhow_message(&err),
            })),
        )
            .into_response(),
    }
}

pub async fn station_auth_sign_out_handler(State(state): State<AppState>) -> Response {
    match state.station_token_store.clear() {
        Ok(()) => (StatusCode::OK, Json(json!({ "signed_in": false }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error_class": "KEYCHAIN",
                "message": anyhow_message(&err),
            })),
        )
            .into_response(),
    }
}

fn public_to_json(public: &DeviceLoginPublic, _console_base: &str) -> Value {
    // Explicit map so we never accidentally serialize private fields.
    // Same URL as verification_uri_complete — one browser sign-in (AuthKit device), not Console OAuth first.
    json!({
        "user_code": public.user_code,
        "verification_uri": public.verification_uri,
        "verification_uri_complete": public.verification_uri_complete,
        "browser_url": public.verification_uri_complete,
        "expires_in": public.expires_in,
        "interval": public.interval,
    })
}

fn identity_to_json(identity: &StationSessionIdentity) -> Value {
    json!({
        "profile_id": identity.profile_id,
        "email": identity.email,
        "aud": identity.aud,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_response_json_omits_device_code() {
        let public = DeviceLoginPublic {
            user_code: "RRGQ-BJVS".into(),
            verification_uri: "https://example.authkit.app/device".into(),
            verification_uri_complete: "https://example.authkit.app/device?user_code=RRGQ-BJVS"
                .into(),
            expires_in: 300,
            interval: 5,
        };
        let json = public_to_json(
            &public,
            "https://www.tradeautopsy.in",
        );
        assert!(json.get("device_code").is_none());
        assert_eq!(json["user_code"], "RRGQ-BJVS");
        assert_eq!(
            json["browser_url"].as_str().unwrap(),
            json["verification_uri_complete"].as_str().unwrap()
        );
    }
}
