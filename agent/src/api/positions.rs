//! GET `/api/daemon/positions` — leftover fill inventory for Pulse + Today Open now (S2).
//!
//! Mark-to-market is omitted until a truthful owner exists. Clients must not treat
//! missing `unrealizedPnl` as zero.

use crate::api::AppState;
use crate::today::OpenInventoryRow;
use axum::extract::State;
use axum::Json;
use chrono::SecondsFormat;
use serde_json::{json, Map, Value};
use std::sync::atomic::Ordering;

fn position_json(row: &OpenInventoryRow) -> Value {
    let mut obj = Map::new();
    obj.insert("symbol".into(), json!(row.symbol));
    obj.insert("qty".into(), json!(row.qty));
    obj.insert("side".into(), json!(row.side));
    obj.insert("direction".into(), json!(row.side));
    if let Some(at) = row.first_filled_at {
        obj.insert(
            "firstFilledAt".into(),
            json!(at.to_rfc3339_opts(SecondsFormat::Millis, true)),
        );
    }
    Value::Object(obj)
}

pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let positions = match state.today_service.open_inventory() {
        Ok(rows) => rows.iter().map(position_json).collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    Json(json!({
        "kill_switch_active": state.fog_active.load(Ordering::SeqCst),
        "open_orders": 0,
        "positions": positions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::today::OpenInventoryRow;
    use chrono::{TimeZone, Utc};

    #[test]
    fn position_json_emits_first_filled_at_rfc3339_millis() {
        let first_filled_at = Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap();
        let row = OpenInventoryRow {
            symbol: "BTCUSDT".into(),
            qty: 0.01,
            side: "LONG",
            first_filled_at: Some(first_filled_at),
        };
        let body = position_json(&row);
        assert_eq!(body["symbol"], "BTCUSDT");
        assert_eq!(body["qty"], 0.01);
        assert_eq!(body["side"], "LONG");
        assert_eq!(body["firstFilledAt"], "2026-07-04T12:00:00.000Z");
    }

    #[test]
    fn position_json_omits_first_filled_at_when_none() {
        let row = OpenInventoryRow {
            symbol: "BTCUSDT".into(),
            qty: 0.01,
            side: "LONG",
            first_filled_at: None,
        };
        let body = position_json(&row);
        assert!(body.get("firstFilledAt").is_none() || body["firstFilledAt"].is_null());
    }
}
