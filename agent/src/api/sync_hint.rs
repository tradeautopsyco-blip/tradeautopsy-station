//! Unsigned loopback sync hint for founder dogfood — why obtain(tradebook) is dark.

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
    let book = state
        .account_book
        .lock()
        .expect("account_book mutex poisoned");

    let fills_slots: Vec<Value> = [
        crate::data::BINANCE_COM_SPOT_BOOK_ID,
        crate::data::KOTAK_NSE_BSE_CASH_BOOK_ID,
        crate::data::KOTAK_NSE_NFO_BOOK_ID,
    ]
    .into_iter()
    .map(|book_id| {
            let slot = book.fills_slot(book_id);
            json!({
                "book_id": book_id,
                "polled": slot.is_some(),
                "fill_count": slot.map(|s| s.value.len()),
                "as_of_ms": slot.map(|s| s.as_of_ms),
                "provenance_path": slot.map(|s| s.provenance_path.as_str()),
            })
        })
    .collect();

    Json(json!({
        "broker_connected": snap.broker_connected,
        "active_broker_slug": snap.active_broker_slug,
        "runtime_status": state.broker_sync_control.card_status().as_str(),
        "last_poll_at_ms": snap.last_poll_at_ms,
        "last_success_at_ms": snap.last_success_at_ms,
        "last_error": snap.last_error,
        "fills_trade_history": {
            "current": snap.data_classes.fills_trade_history.current,
            "last_success_at_ms": snap.data_classes.fills_trade_history.last_success_at_ms,
            "last_error_category": snap.data_classes.fills_trade_history.last_error_category,
        },
        "fills_slots": fills_slots,
        "hint": sync_hint_message(&snap, &fills_slots),
    }))
}

fn sync_hint_message(snap: &crate::broker_sync::BrokerRuntimeState, slots: &[Value]) -> &'static str {
    if !snap.broker_connected {
        return "No broker sync running — Connect + Start in Station Brokers, then re-curl obtain.";
    }
    if !snap.data_classes.fills_trade_history.current {
        if snap.last_error.is_some() {
            return "Sync running but fills poll not yet successful — check last_error (session/TOTP/kill switch).";
        }
        return "Sync starting — wait for first fills poll (~3s), then re-curl obtain.";
    }
    let kotak = slots
        .iter()
        .find(|s| s["book_id"] == "kotak-nse-bse-cash")
        .and_then(|s| s["polled"].as_bool())
        .unwrap_or(false);
    let binance = slots
        .iter()
        .find(|s| s["book_id"] == "binance-com-spot")
        .and_then(|s| s["polled"].as_bool())
        .unwrap_or(false);
    match snap.active_broker_slug.as_deref() {
        Some(s) if s.contains("kotak") && !kotak => {
            "Kotak sync active but cash slot missing — fills poll may have failed before write."
        }
        Some(s) if s.contains("binance") && !binance => {
            "Binance sync active but spot slot missing — fills poll may have failed before write."
        }
        _ => "Fills poll succeeded — obtain(tradebook) should be success for the active book.",
    }
}
