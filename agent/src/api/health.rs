use crate::api::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let metrics_port = state
        .metrics_listen_port
        .map(Value::from)
        .unwrap_or(Value::Null);
    Json(json!({
        "status": "ok",
        "daemon": "agent",
        "version": state.runtime.version(),
        "build": state.runtime.build(),
        "boot_id": state.runtime.boot_id(),
        "uptime_secs": state.runtime.uptime_secs(),
        "sse_signing_pubkey_b64": state.sse_signer.public_key_b64(),
        "metrics_listen_port": metrics_port,
        "observability": state.metrics.snapshot_json(),
        "vendors": crate::data::vendor_health_rows(
            &state.gap_vendor.lock().expect("gap_vendor mutex poisoned"),
            *state.amfi_enabled.lock().expect("amfi_enabled mutex poisoned"),
        ),
    }))
}
