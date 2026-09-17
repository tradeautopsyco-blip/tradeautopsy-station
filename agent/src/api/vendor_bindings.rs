//! PUT `/api/daemon/vendor-bindings` — Box Enable reaches GapVendorConfig + vault.

use crate::api::AppState;
use crate::data::{
    binding_for, clamp_budget, secret_looks_like_url, AMFI_ADAPTER_ID, LICENSED_HISTORY_ADAPTER_ID,
    LICENSED_HISTORY_VAULT_CONNECTION_ID,
};
use crate::CredentialBlob;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct VendorBindingPut {
    pub adapter_id: String,
    pub enabled: bool,
    pub api_key: Option<String>,
    pub history_budget: Option<u32>,
}

fn refuse(status: StatusCode, class: &str, message: &str) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "error_class": class,
            "message": message,
        })),
    )
}

pub async fn put_handler(
    State(state): State<AppState>,
    Json(body): Json<VendorBindingPut>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let adapter_id = body.adapter_id.trim();
    let Some(spec) = binding_for(adapter_id) else {
        return Err(refuse(
            StatusCode::BAD_REQUEST,
            "unknown_vendor",
            "adapter is not a declared vendor binding",
        ));
    };

    if let Some(key) = body.api_key.as_deref() {
        if secret_looks_like_url(key) {
            return Err(refuse(
                StatusCode::BAD_REQUEST,
                "url_is_not_a_key",
                "URL is not a key.",
            ));
        }
    }

    if adapter_id == AMFI_ADAPTER_ID {
        *state
            .amfi_enabled
            .lock()
            .expect("amfi_enabled mutex poisoned") = body.enabled;
        return Ok(Json(json!({
            "ok": true,
            "adapter_id": AMFI_ADAPTER_ID,
            "enabled": body.enabled,
        })));
    }

    if adapter_id != LICENSED_HISTORY_ADAPTER_ID {
        return Err(refuse(
            StatusCode::BAD_REQUEST,
            "unknown_vendor",
            "adapter is not a declared vendor binding",
        ));
    }

    let mut cfg = state.gap_vendor.lock().expect("gap_vendor mutex poisoned");
    if let Some(key) = body
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        cfg.key = Some(key.to_string());
        state
            .broker_sync_control
            .credential_vault()
            .save(
                "prod",
                LICENSED_HISTORY_ADAPTER_ID,
                LICENSED_HISTORY_VAULT_CONNECTION_ID,
                &CredentialBlob::hmac(key, "unused"),
            )
            .map_err(|e| {
                tracing::warn!("vendor vault save failed: {e}");
                refuse(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "upstream",
                    "failed to store vendor key",
                )
            })?;
    }

    if body.enabled && spec.requires_key {
        let has_key = cfg
            .key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_some();
        if !has_key {
            drop(cfg);
            let vault = state.broker_sync_control.credential_vault();
            if let Ok(Some(CredentialBlob::HmacApiKeySecret { api_key, .. })) = vault.load(
                "prod",
                LICENSED_HISTORY_ADAPTER_ID,
                LICENSED_HISTORY_VAULT_CONNECTION_ID,
            ) {
                if !api_key.trim().is_empty() && !secret_looks_like_url(&api_key) {
                    let mut cfg = state.gap_vendor.lock().expect("gap_vendor mutex poisoned");
                    cfg.key = Some(api_key);
                    cfg.enabled = true;
                    cfg.history_budget = clamp_budget(spec, body.history_budget);
                    return Ok(Json(json!({
                        "ok": true,
                        "adapter_id": LICENSED_HISTORY_ADAPTER_ID,
                        "enabled": true,
                    })));
                }
            }
            return Err(refuse(
                StatusCode::BAD_REQUEST,
                "missing_key",
                "licensed_history requires a Keychain key",
            ));
        }
    }

    cfg.enabled = body.enabled;
    if body.enabled {
        cfg.history_budget = clamp_budget(spec, body.history_budget);
    }
    Ok(Json(json!({
        "ok": true,
        "adapter_id": LICENSED_HISTORY_ADAPTER_ID,
        "enabled": cfg.enabled,
    })))
}
