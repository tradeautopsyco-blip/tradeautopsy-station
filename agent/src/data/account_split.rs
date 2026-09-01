//! Route polled fills into named account books by adapter + segment rules.

use crate::broker::BrokerFill;
use crate::kotak_nfo_scrip::{KotakNfoContract, KotakNfoScripMaster};
use std::collections::HashMap;

use super::descriptor::{
    BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};

fn normalize_adapter_name(adapter_name: &str) -> &str {
    if adapter_name.contains("binance_com") {
        "binance_com"
    } else if adapter_name.contains("kotak") {
        "kotak_neo"
    } else {
        adapter_name
    }
}

fn norm_seg(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

fn is_kotak_nfo_segment(segment: &str) -> bool {
    segment == "nse_fo"
}

fn is_kotak_cash_segment(segment: &str) -> bool {
    matches!(segment, "nse_cm" | "bse_cm")
}

fn kotak_nfo_fill_ok(fill: &BrokerFill, master: &KotakNfoScripMaster) -> bool {
    let prod_raw = fill.product.as_deref().unwrap_or("").trim();
    if prod_raw.is_empty() {
        return false;
    }
    let prod = prod_raw.to_ascii_uppercase();
    match prod.as_str() {
        "NRML" => true,
        "CNC" | "CO" => false,
        "BO" => false,
        "MIS" => match master.get_by_trading_symbol(&fill.symbol) {
            Some(row) => {
                let inst = row.instrument_type.trim().to_ascii_uppercase();
                matches!(inst.as_str(), "OPTIDX" | "FUTIDX" | "FUTSTK")
            }
            None => false,
        },
        _ => false,
    }
}

/// Master hit only — never fall back to symbol-suffix guess (TB2 stamp lock).
fn instrument_type_from_master(row: &KotakNfoContract) -> Option<String> {
    let opt = row.option_type.trim().to_ascii_uppercase();
    if matches!(opt.as_str(), "CE" | "PE") {
        return Some(opt);
    }
    let inst = row.instrument_type.trim().to_ascii_uppercase();
    if matches!(inst.as_str(), "FUTIDX" | "FUTSTK") {
        return Some("FUT".to_string());
    }
    None
}

/// Stamp NFO book fills from an exact master join. Prefix search is not used.
pub fn stamp_nfo_fills(fills: &mut [BrokerFill], master: &KotakNfoScripMaster) {
    for fill in fills.iter_mut() {
        if let Some(row) = master.get_by_trading_symbol(&fill.symbol) {
            fill.lot = Some(row.lot);
            fill.instrument_type = instrument_type_from_master(row);
        }
    }
}

pub fn split_fills_by_book(
    adapter_name: &str,
    fills: Vec<BrokerFill>,
    nfo_master: Option<&KotakNfoScripMaster>,
) -> HashMap<String, Vec<BrokerFill>> {
    let slug = normalize_adapter_name(adapter_name);
    match slug {
        "binance_com" => {
            let mut out = HashMap::new();
            out.insert(BINANCE_COM_SPOT_BOOK_ID.to_string(), fills);
            out
        }
        "kotak_neo" => {
            let empty = KotakNfoScripMaster::empty();
            let master = nfo_master.unwrap_or(&empty);
            let mut cash = Vec::new();
            let mut nfo = Vec::new();
            for fill in fills {
                let segment = norm_seg(fill.exchange_segment.as_deref().unwrap_or(""));
                if is_kotak_nfo_segment(&segment) && kotak_nfo_fill_ok(&fill, master) {
                    nfo.push(fill);
                } else if is_kotak_cash_segment(&segment) && kotak_cash_fill_ok(&fill) {
                    cash.push(fill);
                }
            }
            let mut out = HashMap::new();
            out.insert(KOTAK_NSE_BSE_CASH_BOOK_ID.to_string(), cash);
            out.insert(KOTAK_NSE_NFO_BOOK_ID.to_string(), nfo);
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

fn kotak_cash_product_ok(product: &str) -> bool {
    matches!(product, "CNC" | "MIS")
}

fn kotak_nfo_row_ok(symbol: &str, product: &str, master: &KotakNfoScripMaster) -> bool {
    let prod = product.trim().to_ascii_uppercase();
    match prod.as_str() {
        "NRML" => true,
        "CNC" | "CO" | "BO" => false,
        "MIS" => match master.get_by_trading_symbol(symbol) {
            Some(row) => {
                let inst = row.instrument_type.trim().to_ascii_uppercase();
                matches!(inst.as_str(), "OPTIDX" | "FUTIDX" | "FUTSTK")
            }
            None => false,
        },
        _ => false,
    }
}

pub fn split_kotak_orders_by_book(
    orders: Vec<crate::broker_data_class::BrokerOpenOrder>,
    master: &KotakNfoScripMaster,
) -> HashMap<String, Vec<crate::broker_data_class::BrokerOpenOrder>> {
    let mut cash = Vec::new();
    let mut nfo = Vec::new();
    for order in orders {
        let segment = norm_seg(order.exchange_segment.as_deref().unwrap_or(""));
        let product = order.product.as_deref().unwrap_or("");
        if is_kotak_nfo_segment(&segment) && kotak_nfo_row_ok(&order.symbol, product, master) {
            nfo.push(order);
        } else if is_kotak_cash_segment(&segment) && kotak_cash_product_ok(product) {
            cash.push(order);
        }
    }
    let mut out = HashMap::new();
    out.insert(KOTAK_NSE_BSE_CASH_BOOK_ID.to_string(), cash);
    out.insert(KOTAK_NSE_NFO_BOOK_ID.to_string(), nfo);
    out
}

pub fn split_kotak_positions_by_book(
    positions: Vec<crate::broker_data_class::BrokerPositionRow>,
    master: &KotakNfoScripMaster,
) -> HashMap<String, Vec<crate::broker_data_class::BrokerPositionRow>> {
    let mut cash = Vec::new();
    let mut nfo = Vec::new();
    for row in positions {
        let segment = norm_seg(&row.exchange_segment);
        if is_kotak_nfo_segment(&segment) && kotak_nfo_row_ok(&row.trading_symbol, &row.product, master)
        {
            nfo.push(row);
        } else if is_kotak_cash_segment(&segment) && kotak_cash_product_ok(&row.product) {
            cash.push(row);
        }
    }
    let mut out = HashMap::new();
    out.insert(KOTAK_NSE_BSE_CASH_BOOK_ID.to_string(), cash);
    out.insert(KOTAK_NSE_NFO_BOOK_ID.to_string(), nfo);
    out
}

pub fn split_kotak_holdings_by_book(
    holdings: Vec<crate::broker_data_class::BrokerPortfolioHolding>,
) -> HashMap<String, Vec<crate::broker_data_class::BrokerPortfolioHolding>> {
    let mut out = HashMap::new();
    out.insert(KOTAK_NSE_BSE_CASH_BOOK_ID.to_string(), holdings);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;

    const NFO_FIXTURE_CSV: &str = include_str!("../../fixtures/kotak/nse_fo_header.csv");

    fn kotak_fill(segment: &str, product: &str, lot: Option<i64>) -> BrokerFill {
        kotak_fill_sym(segment, product, "SYM", lot)
    }

    fn kotak_fill_sym(segment: &str, product: &str, symbol: &str, lot: Option<i64>) -> BrokerFill {
        BrokerFill {
            fill_id: format!("K-{segment}-{product}-{symbol}"),
            trade_id: "T1".into(),
            symbol: symbol.into(),
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

    fn fixture_master() -> KotakNfoScripMaster {
        KotakNfoScripMaster::from_csv_bytes(NFO_FIXTURE_CSV.as_bytes()).unwrap()
    }

    fn mis_master_with_optstk() -> KotakNfoScripMaster {
        let header = NFO_FIXTURE_CSV.lines().next().expect("header");
        let base = NFO_FIXTURE_CSV
            .lines()
            .nth(1)
            .expect("fixture row")
            .split(',')
            .collect::<Vec<_>>();
        let mut optstk = base.clone();
        optstk[0] = "56527";
        optstk[3] = "OPTSTK";
        optstk[5] = "RELIANCE26SEP2400CE";
        optstk[6] = "CE";
        let csv = format!("{header}\n{}\n{}", base.join(","), optstk.join(","));
        KotakNfoScripMaster::from_csv_bytes(csv.as_bytes()).unwrap()
    }

    #[test]
    fn binance_all_fills_go_to_spot_book() {
        let fills = vec![BrokerFill {
            symbol: "BTCUSDT".into(),
            ..BrokerFill::default()
        }];
        let split = split_fills_by_book("binance_com_wasm", fills, None);
        assert_eq!(split.len(), 1);
        assert!(split.contains_key(BINANCE_COM_SPOT_BOOK_ID));
    }

    #[test]
    fn kotak_nse_fo_not_in_cash_book() {
        let master = fixture_master();
        let fills = vec![kotak_fill("nse_fo", "NRML", Some(50))];
        let split = split_fills_by_book("kotak_neo", fills, Some(&master));
        assert_eq!(split.get(KOTAK_NSE_BSE_CASH_BOOK_ID).map(|v| v.len()), Some(0));
        assert_eq!(split.get(KOTAK_NSE_NFO_BOOK_ID).map(|v| v.len()), Some(1));
    }

    #[test]
    fn kotak_cash_drops_nse_fo_and_bad_lot() {
        let master = fixture_master();
        let mixed = vec![
            kotak_fill("nse_cm", "CNC", None),
            kotak_fill("nse_fo", "NRML", Some(50)),
            kotak_fill("nse_cm", "CNC", Some(50)),
        ];
        let split = split_fills_by_book("kotak_neo", mixed, Some(&master));
        let cash = split
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID)
            .expect("cash book");
        assert_eq!(cash.len(), 1);
        assert_eq!(cash[0].exchange_segment.as_deref(), Some("nse_cm"));
        assert_eq!(split.get(KOTAK_NSE_NFO_BOOK_ID).map(|v| v.len()), Some(1));
    }

    #[test]
    fn kotak_mixed_cash_cnc_and_nfo_nrml_two_keys() {
        let master = fixture_master();
        let mixed = vec![
            kotak_fill("nse_cm", "CNC", None),
            kotak_fill("NSE_FO", "Nrml", None),
        ];
        let split = split_fills_by_book("kotak_neo", mixed, Some(&master));
        assert!(split.contains_key(KOTAK_NSE_BSE_CASH_BOOK_ID));
        assert!(split.contains_key(KOTAK_NSE_NFO_BOOK_ID));
        assert_eq!(split[KOTAK_NSE_BSE_CASH_BOOK_ID].len(), 1);
        assert_eq!(split[KOTAK_NSE_NFO_BOOK_ID].len(), 1);
    }

    #[test]
    fn kotak_mis_optstk_dropped_optidx_kept() {
        let master = mis_master_with_optstk();
        let fills = vec![
            kotak_fill_sym("nse_fo", "MIS", "NIFTY2692221000PE", None),
            kotak_fill_sym("nse_fo", "MIS", "RELIANCE26SEP2400CE", None),
        ];
        let split = split_fills_by_book("kotak_neo", fills, Some(&master));
        let nfo = split.get(KOTAK_NSE_NFO_BOOK_ID).expect("nfo book");
        assert_eq!(nfo.len(), 1);
        assert_eq!(nfo[0].symbol, "NIFTY2692221000PE");
    }

    #[test]
    fn kotak_mis_empty_master_dropped() {
        let fills = vec![kotak_fill_sym("nse_fo", "MIS", "NIFTY2692221000PE", None)];
        let split = split_fills_by_book("kotak_neo", fills, None);
        assert_eq!(split[KOTAK_NSE_NFO_BOOK_ID].len(), 0);
    }

    #[test]
    fn kotak_cnc_on_fo_absent() {
        let master = fixture_master();
        let fills = vec![kotak_fill("nse_fo", "CNC", None)];
        let split = split_fills_by_book("kotak_neo", fills, Some(&master));
        assert_eq!(split[KOTAK_NSE_NFO_BOOK_ID].len(), 0);
        assert_eq!(split[KOTAK_NSE_BSE_CASH_BOOK_ID].len(), 0);
    }

    #[test]
    fn kotak_bse_fo_vanishes() {
        let master = fixture_master();
        let fills = vec![kotak_fill("bse_fo", "NRML", None)];
        let split = split_fills_by_book("kotak_neo", fills, Some(&master));
        assert_eq!(split[KOTAK_NSE_NFO_BOOK_ID].len(), 0);
        assert_eq!(split[KOTAK_NSE_BSE_CASH_BOOK_ID].len(), 0);
    }

    #[test]
    fn kotak_prod_missing_dropped() {
        let master = fixture_master();
        let mut fill = kotak_fill("nse_fo", "NRML", None);
        fill.product = None;
        let split = split_fills_by_book("kotak_neo", vec![fill], Some(&master));
        assert_eq!(split[KOTAK_NSE_NFO_BOOK_ID].len(), 0);
    }

    #[test]
    fn stamp_nfo_fills_sets_lot_from_exact_symbol() {
        let master = fixture_master();
        let mut fills = vec![kotak_fill_sym("nse_fo", "NRML", "NIFTY2692221000PE", None)];
        stamp_nfo_fills(&mut fills, &master);
        assert_eq!(fills[0].lot, Some(65));
        assert_eq!(fills[0].instrument_type.as_deref(), Some("PE"));
    }

    #[test]
    fn stamp_nfo_fills_miss_keeps_lot_none() {
        let master = fixture_master();
        let mut fills = vec![kotak_fill_sym("nse_fo", "NRML", "NIFTY25JUL24000CE", None)];
        fills[0].instrument_type = Some("CE".into());
        stamp_nfo_fills(&mut fills, &master);
        assert_eq!(fills[0].lot, None);
        assert_eq!(fills[0].instrument_type.as_deref(), Some("CE"));
    }

    #[test]
    fn stamp_nfo_fills_master_pe_overrides_ce_suffix_guess() {
        let master = fixture_master();
        let mut fills = vec![kotak_fill_sym("nse_fo", "NRML", "NIFTY2692221000PE", None)];
        fills[0].instrument_type = Some("CE".into());
        stamp_nfo_fills(&mut fills, &master);
        assert_eq!(fills[0].instrument_type.as_deref(), Some("PE"));
    }

    #[test]
    fn kotak_bo_dropped_from_nfo() {
        let master = fixture_master();
        let fills = vec![kotak_fill("nse_fo", "BO", None)];
        let split = split_fills_by_book("kotak_neo", fills, Some(&master));
        assert_eq!(split[KOTAK_NSE_NFO_BOOK_ID].len(), 0);
    }

    #[test]
    fn stamp_nfo_fills_does_not_use_prefix_search() {
        let master = fixture_master();
        assert!(master.get_by_trading_symbol("NIFTY").is_none());
        let mut fills = vec![kotak_fill_sym("nse_fo", "NRML", "NIFTY", None)];
        stamp_nfo_fills(&mut fills, &master);
        assert_eq!(fills[0].lot, None);
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
