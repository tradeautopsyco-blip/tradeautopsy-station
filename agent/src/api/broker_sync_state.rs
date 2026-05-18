//! GET `/api/daemon/broker/sync-state` — last poll + posture (design §4.2).

use crate::api::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let snap = state
        .broker_status
        .lock()
        .expect("broker_status mutex poisoned")
        .clone();

    let sync_state = snap.sync_state_literal(
        state.broker_limits.fresh_secs,
        state.broker_limits.stale_secs,
    );

    Json(json!({
        "syncState": sync_state,
        "broker": snap.backend_broker_label,
        "lastPollAtMs": snap.last_poll_at_ms,
        "lastSuccessAtMs": snap.last_success_at_ms,
        "circuitOpen": snap.circuit_open,
        "consecutiveFailures": snap.consecutive_failures,
        "lastError": snap.last_error,
    }))
}
