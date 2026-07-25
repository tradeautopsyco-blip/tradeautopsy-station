//! GET `/api/daemon/broker/sync-state` — last poll + posture (design §4.2).

use crate::api::AppState;
use axum::extract::State;
use axum::Json;
use chrono::Utc;
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
    let now_ms = Utc::now().timestamp_millis();
    let desk = crate::ubi::desk_profile_for_slug(snap.active_broker_slug.as_deref());

    Json(json!({
        "syncState": sync_state,
        "runtimeStatus": state.broker_sync_control.card_status().as_str(),
        "broker": snap.backend_broker_label,
        "brokerSlug": snap.active_broker_slug,
        "quoteCurrency": desk.as_ref().map(|d| d.quote_currency.clone()),
        "calcProfileId": desk.as_ref().map(|d| d.calc_profile_id.clone()),
        "lastPollAtMs": snap.last_poll_at_ms,
        "lastSuccessAtMs": snap.last_success_at_ms,
        "circuitOpen": snap.circuit_open,
        "consecutiveFailures": snap.consecutive_failures,
        "lastError": snap.last_error,
        "dataClasses": snap.data_classes,
        "requiresManualRetry": snap.data_classes.any_requires_manual_retry(),
        "rateLimitRetryAtMs": snap.data_classes.earliest_rate_limit_retry_ms(now_ms),
        "failingDataClasses": snap.data_classes.failing_class_labels(),
    }))
}
