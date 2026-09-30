//! Console `station_open_positions_snapshot` — built from local account book (Kotak cash holdings).

use crate::api::AppState;
use crate::data::{AccountBook, KOTAK_NSE_BSE_CASH_BOOK_ID};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

const OPEN_TRIP_NS: Uuid = Uuid::NAMESPACE_DNS;

fn stable_detection_id(book_id: &str, symbol: &str) -> String {
    let name = format!("tradeautopsy.open-trip/{book_id}/{symbol}");
    Uuid::new_v5(&OPEN_TRIP_NS, name.as_bytes()).to_string()
}

fn row_from_holding(
    book_id: &str,
    h: &crate::broker_data_class::BrokerPortfolioHolding,
) -> Option<Value> {
    let qty = h.quantity;
    if !qty.is_finite() || qty <= 0.0 {
        return None;
    }
    let avg = h.average_price;
    let market = h.market_value;
    if !avg.is_finite() || !market.is_finite() {
        return None;
    }
    let ltp = market / qty;
    let unrealized = market - avg * qty;
    let symbol = h.symbol.trim();
    if symbol.is_empty() {
        return None;
    }
    Some(json!({
        "id": format!("pos-{}-{}", book_id, symbol),
        "symbol": symbol,
        "displayName": symbol,
        "qty": qty,
        "ltp": ltp,
        "avgPrice": avg,
        "unrealizedPnl": unrealized,
        "detectionId": stable_detection_id(book_id, symbol),
        "declaration": null,
        "consoleTradeId": null,
    }))
}

/// Build Console v1 open-positions `value` from the in-memory account book.
pub fn build_open_positions_value(book: &AccountBook, broker_label: &str) -> Option<Value> {
    let mut positions: Vec<Value> = Vec::new();

    if let Some(slot) = book.holdings_slot(KOTAK_NSE_BSE_CASH_BOOK_ID) {
        for h in &slot.value.holdings {
            if let Some(row) = row_from_holding(KOTAK_NSE_BSE_CASH_BOOK_ID, h) {
                positions.push(row);
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    Some(json!({
        "v": 1,
        "brokerLabel": broker_label,
        "updatedAt": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "positions": positions,
    }))
}

pub fn build_from_app_state(state: &AppState) -> Option<Value> {
    let broker_label = {
        let st = state.broker_status.lock().ok()?;
        st.backend_broker_label
            .clone()
            .unwrap_or_else(|| "Kotak Neo".to_string())
    };
    let book = state.account_book.lock().ok()?;
    build_open_positions_value(&book, &broker_label)
}

pub fn try_enqueue_from_state(state: &AppState) {
    let Some(body) = build_from_app_state(state) else {
        return;
    };
    let _ = state.fact_outbox.enqueue_open_positions_snapshot(body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker_data_class::{BrokerHoldingsSnapshot, BrokerPortfolioHolding};

    #[test]
    fn stable_detection_id_is_uuid_v5() {
        let a = stable_detection_id("kotak-nse-bse-cash", "TATASTEEL");
        let b = stable_detection_id("kotak-nse-bse-cash", "TATASTEEL");
        assert_eq!(a, b);
        assert!(Uuid::parse_str(&a).is_ok());
    }

    #[test]
    fn builds_from_holdings() {
        let mut book = AccountBook::new();
        book.replace_holdings(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            BrokerHoldingsSnapshot {
                holdings: vec![BrokerPortfolioHolding {
                    symbol: "TATASTEEL".into(),
                    exchange_segment: "nse_cm".into(),
                    quantity: 2.0,
                    sellable_quantity: 2.0,
                    average_price: 100.0,
                    market_value: 210.0,
                    instrument_type: "EQ".into(),
                }],
            },
            "/holdings",
            1,
        );
        let v = build_open_positions_value(&book, "Kotak Neo").expect("payload");
        let positions = v.get("positions").and_then(|p| p.as_array()).unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(
            positions[0].get("symbol").and_then(|s| s.as_str()),
            Some("TATASTEEL")
        );
        assert_eq!(positions[0].get("ltp").and_then(|n| n.as_f64()), Some(105.0));
    }
}
