//! Per-book account slots (fills, funds, orders). Obtain(tradebook) reads fills here.

use crate::broker::BrokerFill;
use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerOpenOrdersSnapshot};
use std::collections::HashMap;

use super::descriptor::{
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Slot<T> {
    pub as_of_ms: i64,
    pub provenance_path: String,
    pub value: T,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookAccount {
    pub fills: Option<Slot<Vec<BrokerFill>>>,
    pub funds: Option<Slot<BrokerBalancesSnapshot>>,
    // F3 dark — not wired in Step 0.
    pub holdings: Option<Slot<Vec<()>>>,
    pub positions: Option<Slot<Vec<()>>>,
    pub orders: Option<Slot<BrokerOpenOrdersSnapshot>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccountBook {
    slots: HashMap<String, BookAccount>,
}

impl AccountBook {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn replace_fills(
        &mut self,
        book_id: &str,
        rows: Vec<BrokerFill>,
        path: &str,
        as_of_ms: i64,
    ) {
        let entry = self.slots.entry(book_id.to_string()).or_default();
        entry.fills = Some(Slot {
            as_of_ms,
            provenance_path: path.to_string(),
            value: rows,
        });
    }

    pub fn fills_slot(&self, book_id: &str) -> Option<&Slot<Vec<BrokerFill>>> {
        self.slots.get(book_id).and_then(|b| b.fills.as_ref())
    }

    pub fn clear_fills(&mut self, book_id: &str) {
        if let Some(entry) = self.slots.get_mut(book_id) {
            entry.fills = None;
        }
    }

    pub fn clear_books(&mut self, book_ids: &[&str]) {
        for id in book_ids {
            self.slots.remove(*id);
        }
    }

    pub fn books_for_adapter_name(name: &str) -> &'static [&'static str] {
        let lower = name.trim().to_ascii_lowercase();
        if lower.contains("binance") {
            &[BINANCE_COM_SPOT_BOOK_ID, BINANCE_COM_OPTIONS_BOOK_ID]
        } else if lower.contains("kotak") {
            &[KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID]
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;

    fn sample_fill(symbol: &str) -> BrokerFill {
        BrokerFill {
            fill_id: format!("F-{symbol}"),
            trade_id: "T1".into(),
            symbol: symbol.into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 100.0,
            filled_at: Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap(),
            broker: "binance_com".into(),
            ..BrokerFill::default()
        }
    }

    #[test]
    fn empty_slot_is_unavailable() {
        let book = AccountBook::new();
        assert!(book.fills_slot(BINANCE_COM_SPOT_BOOK_ID).is_none());
    }

    #[test]
    fn empty_vec_slot_is_success_with_zero_rows() {
        let mut book = AccountBook::new();
        book.replace_fills(BINANCE_COM_SPOT_BOOK_ID, vec![], "/api/v3/myTrades", 1_000);
        let slot = book.fills_slot(BINANCE_COM_SPOT_BOOK_ID).expect("slot");
        assert!(slot.value.is_empty());
        assert_eq!(slot.provenance_path, "/api/v3/myTrades");
    }

    #[test]
    fn clear_books_removes_named_slots() {
        let mut book = AccountBook::new();
        book.replace_fills(
            BINANCE_COM_SPOT_BOOK_ID,
            vec![sample_fill("BTCUSDT")],
            "/api/v3/myTrades",
            1_000,
        );
        book.clear_books(&[BINANCE_COM_SPOT_BOOK_ID]);
        assert!(book.fills_slot(BINANCE_COM_SPOT_BOOK_ID).is_none());
    }

    #[test]
    fn books_for_adapter_name_maps_known_slugs() {
        assert_eq!(
            AccountBook::books_for_adapter_name("binance_com"),
            &[BINANCE_COM_SPOT_BOOK_ID, BINANCE_COM_OPTIONS_BOOK_ID]
        );
        assert_eq!(
            AccountBook::books_for_adapter_name("binance_us"),
            &[BINANCE_COM_SPOT_BOOK_ID, BINANCE_COM_OPTIONS_BOOK_ID]
        );
        assert_eq!(
            AccountBook::books_for_adapter_name("kotak_neo_wasm"),
            &[KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID]
        );
    }
}
