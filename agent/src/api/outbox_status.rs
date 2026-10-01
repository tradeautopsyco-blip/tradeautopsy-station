use crate::api::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

const DEAD_LETTER_LIMIT: u32 = 25;

pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let snapshot = state
        .outbox
        .status_snapshot(DEAD_LETTER_LIMIT)
        .unwrap_or_else(|_| crate::OutboxStatusSnapshot {
            counts: crate::OutboxCounts {
                enqueued: 0,
                inflight: 0,
                acked: 0,
                dead_letter: 0,
            },
            dead_letters: Vec::new(),
            active_deliveries: Vec::new(),
        });

    Json(json!({
        "success": true,
        "data": snapshot
    }))
}
