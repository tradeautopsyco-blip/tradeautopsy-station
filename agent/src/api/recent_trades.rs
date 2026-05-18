//! GET `/api/daemon/toolbar/recent-trades` — wire v1, local SQLite (design §4.2).

use crate::api::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize, Default)]
pub struct RecentTradesParams {
    limit: Option<usize>,
}

pub async fn handler(
    Query(q): Query<RecentTradesParams>,
    State(state): State<AppState>,
) -> Json<Value> {
    let limit = q.limit.unwrap_or(80).clamp(1, 200);
    let trades = state
        .recent_trades
        .fetch_recent_json(limit)
        .unwrap_or_else(|_| vec![]);

    let snap = state
        .broker_status
        .lock()
        .expect("broker_status mutex poisoned")
        .clone();

    Json(json!({
        "trades": trades,
        "syncState": snap.sync_state_literal(state.broker_limits.fresh_secs, state.broker_limits.stale_secs),
        "broker": snap.backend_broker_label,
        "lastPollAtMs": snap.last_poll_at_ms,
        "circuitOpen": snap.circuit_open,
    }))
}
