//! Route polled fills into named account books by adapter + segment rules.

use crate::broker::BrokerFill;
use std::collections::HashMap;

use super::descriptor::{
    BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use super::kotak_quotes::{is_cash_segment, is_nfo_segment};

fn normalize_adapter_name(adapter_name: &str) -> &str {
    if adapter_name.contains("binance_com") {
        "binance_com"
    } else if adapter_name.contains("kotak") {
        "kotak_neo"
    } else {
        adapter_name
    }
}

pub fn split_fills_by_book(
    adapter_name: &str,
    fills: Vec<BrokerFill>,
) -> HashMap<String, Vec<BrokerFill>> {
    let slug = normalize_adapter_name(adapter_name);
    match slug {
        "binance_com" => {
            let mut out = HashMap::new();
            out.insert(BINANCE_COM_SPOT_BOOK_ID.to_string(), fills);
            out
        }
        "kotak_neo" => {
            let mut cash = Vec::new();
            let mut nfo = Vec::new();
            for fill in fills {
                let segment = fill.exchange_segment.as_deref().unwrap_or("");
                if is_nfo_segment(segment) {
                    nfo.push(fill);
                } else if is_cash_segment(segment) && kotak_cash_fill_ok(&fill) {
                    cash.push(fill);
                }
            }
            let mut out = HashMap::new();
            if !cash.is_empty() {
                out.insert(KOTAK_NSE_BSE_CASH_BOOK_ID.to_string(), cash);
            }
            if !nfo.is_empty() {
                out.insert(KOTAK_NSE_NFO_BOOK_ID.to_string(), nfo);
            }
            out
        }
        _ => HashMap::new(),
    }
}

fn kotak_cash_fill_ok(fill: &BrokerFill) -> bool {
    let product = fill.product.as_deref().unwrap_or("");
    if !matches!(product, "CNC" | "MIS") {
        return false;
    }
    match fill.lot {
        Some(n) if n != 1 => false,
        _ => true,
    }
}

pub fn merge_poll_book_id(adapter_name: &str) -> &str {
    match normalize_adapter_name(adapter_name) {
        "binance_com" => BINANCE_COM_SPOT_BOOK_ID,
        "kotak_neo" => KOTAK_NSE_BSE_CASH_BOOK_ID,
        other => other,
    }
}

pub fn fills_provenance_path(adapter_name: &str) -> &str {
    match normalize_adapter_name(adapter_name) {
        "binance_com" => "/api/v3/myTrades",
        "kotak_neo" => "/quick/user/trades",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;

    fn kotak_fill(segment: &str, product: &str, lot: Option<i64>) -> BrokerFill {
        BrokerFill {
            fill_id: format!("K-{segment}-{product}"),
            trade_id: "T1".into(),
            symbol: "SYM".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 100.0,
            filled_at: Utc.with_ymd_and_hms(2026, 8, 1, 10, 0, 0).unwrap(),
            broker: "kotak_neo".into(),
            product: Some(product.into()),
            exchange_segment: Some(segment.into()),
            lot,
            ..BrokerFill::default()
        }
    }

    #[test]
    fn binance_all_fills_go_to_spot_book() {
        let fills = vec![BrokerFill {
            symbol: "BTCUSDT".into(),
            ..BrokerFill::default()
        }];
        let split = split_fills_by_book("binance_com_wasm", fills);
        assert_eq!(split.len(), 1);
        assert!(split.contains_key(BINANCE_COM_SPOT_BOOK_ID));
    }

    #[test]
    fn kotak_nse_fo_not_in_cash_book() {
        let fills = vec![kotak_fill("nse_fo", "NRML", Some(50))];
        let split = split_fills_by_book("kotak_neo", fills);
        assert!(!split.contains_key(KOTAK_NSE_BSE_CASH_BOOK_ID));
        assert_eq!(split.get(KOTAK_NSE_NFO_BOOK_ID).map(|v| v.len()), Some(1));
    }

    #[test]
    fn kotak_cash_drops_nse_fo_and_bad_lot() {
        let mixed = vec![
            kotak_fill("nse_cm", "CNC", None),
            kotak_fill("nse_fo", "NRML", Some(50)),
            kotak_fill("nse_cm", "CNC", Some(50)),
        ];
        let split = split_fills_by_book("kotak_neo", mixed);
        let cash = split
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID)
            .expect("cash book");
        assert_eq!(cash.len(), 1);
        assert_eq!(cash[0].exchange_segment.as_deref(), Some("nse_cm"));
    }

    #[test]
    fn merge_poll_book_id_returns_shipping_books() {
        assert_eq!(merge_poll_book_id("binance_com"), BINANCE_COM_SPOT_BOOK_ID);
        assert_eq!(merge_poll_book_id("kotak_neo"), KOTAK_NSE_BSE_CASH_BOOK_ID);
    }

    #[test]
    fn fills_provenance_path_maps_venue_paths() {
        assert_eq!(fills_provenance_path("binance_com"), "/api/v3/myTrades");
        assert_eq!(fills_provenance_path("kotak_neo"), "/quick/user/trades");
    }
}
