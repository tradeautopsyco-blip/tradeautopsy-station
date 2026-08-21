//! GET `/api/daemon/positions` — leftover fill inventory for Pulse + Today Open now (S2).
//!
//! Mark-to-market is omitted until a truthful owner exists. Clients must not treat
//! missing `unrealizedPnl` as zero.

use crate::api::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let positions = match state.today_service.open_inventory() {
        Ok(rows) => rows
            .into_iter()
            .map(|row| {
                json!({
                    "symbol": row.symbol,
                    "qty": row.qty,
                    "side": row.side,
                    "direction": row.side,
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    Json(json!({
        "kill_switch_active": state.fog_active.load(Ordering::SeqCst),
        "open_orders": 0,
        "positions": positions,
    }))
}
