//! Per-book account slots (fills, funds, orders). Obtain(tradebook) reads fills here.

use crate::broker::BrokerFill;
use crate::broker_data_class::{
    BrokerBalancesSnapshot, BrokerHoldingsSnapshot, BrokerOpenOrdersSnapshot,
    BrokerPositionsSnapshot,
};
use std::collections::HashMap;

use super::descriptor::{
    BINANCE_COM_COINM_BOOK_ID, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID,
    BINANCE_COM_USDM_BOOK_ID, KOTAK_MCX_FUTURE_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_CDS_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};

fn funds_book_allowed(book_id: &str) -> bool {
    matches!(
        book_id,
        BINANCE_COM_SPOT_BOOK_ID
            | BINANCE_COM_OPTIONS_BOOK_ID
            | BINANCE_COM_USDM_BOOK_ID
            | BINANCE_COM_COINM_BOOK_ID
            | KOTAK_NSE_BSE_CASH_BOOK_ID
            | KOTAK_NSE_NFO_BOOK_ID
            | KOTAK_NSE_CDS_BOOK_ID
            | KOTAK_MCX_FUTURE_BOOK_ID
    )
}

fn positions_book_allowed(book_id: &str) -> bool {
    matches!(
        book_id,
        BINANCE_COM_OPTIONS_BOOK_ID
            | BINANCE_COM_USDM_BOOK_ID
            | BINANCE_COM_COINM_BOOK_ID
            | KOTAK_NSE_BSE_CASH_BOOK_ID
            | KOTAK_NSE_NFO_BOOK_ID
            | KOTAK_NSE_CDS_BOOK_ID
            | KOTAK_MCX_FUTURE_BOOK_ID
    )
}

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
    pub holdings: Option<Slot<BrokerHoldingsSnapshot>>,
    pub positions: Option<Slot<BrokerPositionsSnapshot>>,
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

    pub fn replace_funds(
        &mut self,
        book_id: &str,
        snapshot: BrokerBalancesSnapshot,
        path: &str,
        as_of_ms: i64,
    ) {
        if !funds_book_allowed(book_id) {
            return;
        }
        let entry = self.slots.entry(book_id.to_string()).or_default();
        entry.funds = Some(Slot {
            as_of_ms,
            provenance_path: path.to_string(),
            value: snapshot,
        });
    }

    pub fn replace_holdings(
        &mut self,
        book_id: &str,
        snapshot: BrokerHoldingsSnapshot,
        path: &str,
        as_of_ms: i64,
    ) {
        if book_id != KOTAK_NSE_BSE_CASH_BOOK_ID {
            return;
        }
        let entry = self.slots.entry(book_id.to_string()).or_default();
        entry.holdings = Some(Slot {
            as_of_ms,
            provenance_path: path.to_string(),
            value: snapshot,
        });
    }

    pub fn replace_positions(
        &mut self,
        book_id: &str,
        snapshot: BrokerPositionsSnapshot,
        path: &str,
        as_of_ms: i64,
    ) {
        if !positions_book_allowed(book_id) {
            return;
        }
        let entry = self.slots.entry(book_id.to_string()).or_default();
        entry.positions = Some(Slot {
            as_of_ms,
            provenance_path: path.to_string(),
            value: snapshot,
        });
    }

    pub fn replace_orders(
        &mut self,
        book_id: &str,
        snapshot: BrokerOpenOrdersSnapshot,
        path: &str,
        as_of_ms: i64,
    ) {
        if book_id != BINANCE_COM_SPOT_BOOK_ID
            && book_id != KOTAK_NSE_BSE_CASH_BOOK_ID
            && book_id != KOTAK_NSE_NFO_BOOK_ID
        {
            return;
        }
        let entry = self.slots.entry(book_id.to_string()).or_default();
        entry.orders = Some(Slot {
            as_of_ms,
            provenance_path: path.to_string(),
            value: snapshot,
        });
    }

    pub fn fills_slot(&self, book_id: &str) -> Option<&Slot<Vec<BrokerFill>>> {
        self.slots.get(book_id).and_then(|b| b.fills.as_ref())
    }

    pub fn funds_slot(&self, book_id: &str) -> Option<&Slot<BrokerBalancesSnapshot>> {
        self.slots.get(book_id).and_then(|b| b.funds.as_ref())
    }

    pub fn holdings_slot(&self, book_id: &str) -> Option<&Slot<BrokerHoldingsSnapshot>> {
        self.slots.get(book_id).and_then(|b| b.holdings.as_ref())
    }

    pub fn positions_slot(&self, book_id: &str) -> Option<&Slot<BrokerPositionsSnapshot>> {
        self.slots.get(book_id).and_then(|b| b.positions.as_ref())
    }

    pub fn orders_slot(&self, book_id: &str) -> Option<&Slot<BrokerOpenOrdersSnapshot>> {
        self.slots.get(book_id).and_then(|b| b.orders.as_ref())
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
        } else if lower.contains("zerodha") {
            &[
                crate::data::ZERODHA_NSE_BSE_CASH_BOOK_ID,
                crate::data::ZERODHA_NSE_NFO_BOOK_ID,
            ]
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker_data_class::{BrokerHolding, BrokerOpenOrder, BrokerPortfolioHolding};
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
    fn replace_funds_spot_only() {
        let mut book = AccountBook::new();
        let snapshot = BrokerBalancesSnapshot {
            holdings: vec![BrokerHolding {
                asset: "BTC".into(),
                free: 0.01,
                locked: 0.0,
            }],
            unrealized_pnl: None,
        };
        book.replace_funds(
            BINANCE_COM_SPOT_BOOK_ID,
            snapshot.clone(),
            "/api/v3/account",
            2_000,
        );
        let slot = book
            .funds_slot(BINANCE_COM_SPOT_BOOK_ID)
            .expect("funds slot");
        assert_eq!(slot.value, snapshot);
        assert_eq!(slot.provenance_path, "/api/v3/account");
        book.replace_funds(
            BINANCE_COM_OPTIONS_BOOK_ID,
            BrokerBalancesSnapshot::default(),
            "/eapi/v1/marginAccount",
            3_000,
        );
        assert!(
            book.funds_slot(BINANCE_COM_OPTIONS_BOOK_ID).is_some(),
            "empty options snapshot is a plant, not a leak from spot"
        );
        book.replace_funds(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            BrokerBalancesSnapshot {
                holdings: vec![BrokerHolding {
                    asset: "INR".into(),
                    free: 19.41,
                    locked: 18.78,
                }],
                unrealized_pnl: None,
            },
            "/quick/user/limits",
            4_000,
        );
        assert!(book.funds_slot(KOTAK_NSE_BSE_CASH_BOOK_ID).is_some());
        assert!(
            book.funds_slot(KOTAK_NSE_NFO_BOOK_ID).is_none(),
            "cash plant must not create the NFO slot"
        );
        book.replace_funds(
            KOTAK_NSE_NFO_BOOK_ID,
            BrokerBalancesSnapshot {
                holdings: vec![BrokerHolding {
                    asset: "INR".into(),
                    free: 19.41,
                    locked: 18.78,
                }],
                unrealized_pnl: None,
            },
            "/quick/user/limits",
            5_000,
        );
        assert!(book.funds_slot(KOTAK_NSE_NFO_BOOK_ID).is_some());
    }

    #[test]
    fn replace_orders_spot_and_kotak() {
        use crate::broker_data_class::BrokerOpenOrdersSnapshot;
        let mut book = AccountBook::new();
        let snapshot = BrokerOpenOrdersSnapshot {
            orders: vec![BrokerOpenOrder {
                order_id: "1".into(),
                symbol: "BTCUSDT".into(),
                side: "BUY".into(),
                qty: 1.0,
                price: Some(65000.0),
                product: None,
                exchange_segment: None,
                status: None,
                unfilled_qty: None,
            }],
        };
        book.replace_orders(
            BINANCE_COM_SPOT_BOOK_ID,
            snapshot.clone(),
            "/api/v3/openOrders",
            2_000,
        );
        let slot = book
            .orders_slot(BINANCE_COM_SPOT_BOOK_ID)
            .expect("orders slot");
        assert_eq!(slot.value.orders.len(), 1);
        book.replace_orders(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            BrokerOpenOrdersSnapshot::empty(),
            "/quick/user/orders",
            3_000,
        );
        assert!(book.orders_slot(KOTAK_NSE_BSE_CASH_BOOK_ID).is_some());
        book.replace_orders(
            BINANCE_COM_OPTIONS_BOOK_ID,
            BrokerOpenOrdersSnapshot::empty(),
            "/api/v3/openOrders",
            4_000,
        );
        assert!(book.orders_slot(BINANCE_COM_OPTIONS_BOOK_ID).is_none());
    }

    #[test]
    fn replace_holdings_kotak_cash_only() {
        let mut book = AccountBook::new();
        let snapshot = BrokerHoldingsSnapshot {
            holdings: vec![BrokerPortfolioHolding {
                symbol: "IDBI".into(),
                exchange_segment: "nse_cm".into(),
                quantity: 1.0,
                sellable_quantity: 1.0,
                average_price: 92.0,
                market_value: 90.0,
                instrument_type: "Equity".into(),
            }],
        };
        book.replace_holdings(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            snapshot.clone(),
            "/portfolio/v1/holdings",
            1_000,
        );
        assert!(book.holdings_slot(KOTAK_NSE_BSE_CASH_BOOK_ID).is_some());
        book.replace_holdings(
            KOTAK_NSE_NFO_BOOK_ID,
            BrokerHoldingsSnapshot::default(),
            "/portfolio/v1/holdings",
            2_000,
        );
        assert!(book.holdings_slot(KOTAK_NSE_NFO_BOOK_ID).is_none());
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
            AccountBook::books_for_adapter_name("kotak_neo_wasm"),
            &[KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID]
        );
    }
}
