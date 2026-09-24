//! NSE F&O realized-PnL owner (`kotak-nse-nfo`, `zerodha-nse-nfo`). Not COM USD WAC.
//!
//! Lock: `issues/compliance/locks/kotak-nse-nfo.md` (founder P5).
//! Method: `(exit_price - entry_price) * qty * lot` per FULL-COVERAGE-PROGRAM P5.
//! Segment `nse_fo` only. Products **NRML** / **MIS** only. Currency INR.
//! DualNoBlend: never `round_trip_engine.rs`, never USD, never `netPnL * exchange_rate`.

use crate::broker::BrokerFill;
use crate::data::is_nfo_segment;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

pub const OWNER_PATH: &str = "agent/src/nfo_realized_pnl.rs";
pub const BOOK_ID: &str = "kotak-nse-nfo";

const EPS: f64 = 1e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct NfoRoundTrip {
    pub symbol: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
    pub avg_entry_price: f64,
    pub avg_exit_price: f64,
    pub qty: f64,
    pub lot: i64,
    pub realized_pnl_inr: Option<f64>,
    pub product: String,
    pub unknown_basis: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NfoReconstructResult {
    pub round_trips: Vec<NfoRoundTrip>,
    pub refused: Vec<NfoRefuse>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfoRefuse {
    pub fill_id: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct NfoRealizedPnlEngine;

impl NfoRealizedPnlEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn reconstruct(&self, fills: Vec<BrokerFill>) -> NfoReconstructResult {
        let mut accepted = Vec::new();
        let mut refused = Vec::new();
        for fill in fills {
            match classify_nfo_fill(&fill) {
                Ok(()) => accepted.push(fill),
                Err(reason) => refused.push(NfoRefuse {
                    fill_id: fill.fill_id.clone(),
                    reason,
                }),
            }
        }

        accepted.sort_by(|a, b| {
            a.filled_at
                .cmp(&b.filled_at)
                .then_with(|| a.fill_id.cmp(&b.fill_id))
        });

        let mut ledgers: BTreeMap<String, SymbolLedger> = BTreeMap::new();
        let mut round_trips = Vec::new();
        for fill in &accepted {
            let key = fill.symbol.clone();
            let ledger = ledgers.entry(key).or_default();
            ledger.apply(fill, &mut round_trips);
        }

        NfoReconstructResult {
            round_trips,
            refused,
        }
    }
}

/// True when this fill belongs to `kotak-nse-nfo` (not cash, not COM).
pub fn is_nfo_fill(fill: &BrokerFill) -> bool {
    classify_nfo_fill(fill).is_ok()
}

fn classify_nfo_fill(fill: &BrokerFill) -> Result<(), &'static str> {
    let seg = fill
        .exchange_segment
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if !is_nfo_segment(&seg) {
        return Err("segment_not_nse_fo");
    }

    let ccy = fill
        .currency
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase());
    match ccy.as_deref() {
        Some("INR") => {}
        Some(_) => return Err("currency_not_inr"),
        None => return Err("currency_missing"),
    }

    let product = fill
        .product
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    if product != "NRML" && product != "MIS" {
        return Err("product_not_nrml_mis");
    }

    let lot = fill.lot.ok_or("lot_missing")?;
    if lot <= 0 {
        return Err("lot_invalid");
    }

    Ok(())
}

/// P5 identity: `(exit - entry) * qty * lot` (long). INR only.
pub fn realized_pnl_inr_long(
    entry_price: f64,
    exit_price: f64,
    qty: f64,
    lot: i64,
) -> Option<f64> {
    if !entry_price.is_finite() || !exit_price.is_finite() || !qty.is_finite() || qty <= EPS {
        return None;
    }
    if lot <= 0 {
        return None;
    }
    let pnl = (exit_price - entry_price) * qty * (lot as f64);
    pnl.is_finite().then_some(pnl)
}

pub fn is_aggregate_eligible(rt: &NfoRoundTrip) -> bool {
    !rt.unknown_basis && rt.realized_pnl_inr.is_some()
}

pub fn aggregate_known_pnl_inr(round_trips: &[NfoRoundTrip]) -> f64 {
    round_trips
        .iter()
        .filter(|rt| is_aggregate_eligible(rt))
        .filter_map(|rt| rt.realized_pnl_inr)
        .sum()
}

#[derive(Debug, Default)]
struct SymbolLedger {
    position_qty: f64,
    open_lot: i64,
    wac_entry: f64,
    trip_opened_at: Option<DateTime<Utc>>,
    trip_product: String,
}

impl SymbolLedger {
    fn apply(&mut self, fill: &BrokerFill, out: &mut Vec<NfoRoundTrip>) {
        let lot = fill.lot.expect("classified");
        let side_buy = fill.side.eq_ignore_ascii_case("BUY");
        let signed = if side_buy { fill.qty } else { -fill.qty };

        if self.position_qty.abs() <= EPS {
            self.open_lot = lot;
            self.wac_entry = fill.price;
            self.trip_opened_at = Some(fill.filled_at);
            self.trip_product = fill
                .product
                .clone()
                .unwrap_or_else(|| "NRML".to_string())
                .to_ascii_uppercase();
            self.position_qty = signed;
            return;
        }

        if self.open_lot != lot {
            out.push(unknown_basis_trip(fill, lot));
            self.position_qty = signed;
            self.open_lot = lot;
            self.wac_entry = fill.price;
            self.trip_opened_at = Some(fill.filled_at);
            return;
        }

        let prev = self.position_qty;
        let same_direction = prev.signum() == signed.signum() || signed == 0.0;
        if same_direction {
            let new_abs = prev.abs() + signed.abs();
            self.wac_entry =
                (self.wac_entry * prev.abs() + fill.price * signed.abs()) / new_abs.max(EPS);
            self.position_qty = prev + signed;
            return;
        }

        let close_qty = signed.abs().min(prev.abs());
        let entry = self.wac_entry;
        let exit = fill.price;
        let is_long = prev > 0.0;
        let pnl = if is_long {
            realized_pnl_inr_long(entry, exit, close_qty, lot)
        } else {
            realized_pnl_inr_long(exit, entry, close_qty, lot)
        };
        out.push(NfoRoundTrip {
            symbol: fill.symbol.clone(),
            opened_at: self.trip_opened_at.unwrap_or(fill.filled_at),
            closed_at: fill.filled_at,
            avg_entry_price: entry,
            avg_exit_price: exit,
            qty: close_qty,
            lot,
            realized_pnl_inr: pnl,
            product: self.trip_product.clone(),
            unknown_basis: false,
        });

        let remainder = signed.abs() - close_qty;
        let new_pos = prev + signed;
        self.position_qty = new_pos;
        if new_pos.abs() <= EPS {
            self.trip_opened_at = None;
            self.trip_product.clear();
        } else if remainder > EPS {
            self.wac_entry = fill.price;
            self.trip_opened_at = Some(fill.filled_at);
            self.trip_product = fill
                .product
                .clone()
                .unwrap_or_else(|| "NRML".to_string())
                .to_ascii_uppercase();
        } else {
            self.wac_entry = entry;
        }
    }
}

fn unknown_basis_trip(fill: &BrokerFill, lot: i64) -> NfoRoundTrip {
    NfoRoundTrip {
        symbol: fill.symbol.clone(),
        opened_at: fill.filled_at,
        closed_at: fill.filled_at,
        avg_entry_price: fill.price,
        avg_exit_price: fill.price,
        qty: fill.qty,
        lot,
        realized_pnl_inr: None,
        product: fill
            .product
            .clone()
            .unwrap_or_else(|| "NRML".to_string()),
        unknown_basis: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn nfo_fill(
        id: &str,
        side: &str,
        qty: f64,
        price: f64,
        lot: i64,
        product: &str,
        at: DateTime<Utc>,
    ) -> BrokerFill {
        BrokerFill {
            fill_id: id.to_string(),
            trade_id: format!("t-{id}"),
            symbol: "NIFTY2692221000PE".to_string(),
            side: side.to_string(),
            qty,
            price,
            filled_at: at,
            broker: "kotak_neo".to_string(),
            fee_amount: None,
            fee_asset: Some("INR".to_string()),
            currency: Some("INR".to_string()),
            product: Some(product.to_string()),
            exchange_segment: Some("nse_fo".to_string()),
            instrument_type: Some("PE".to_string()),
            lot: Some(lot),
        }
    }

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 28, h, m, 0).unwrap()
    }

    #[test]
    fn owner_path_and_golden_qty2_lot65_entry10_exit11() {
        assert_eq!(OWNER_PATH, "agent/src/nfo_realized_pnl.rs");
        assert_eq!(BOOK_ID, "kotak-nse-nfo");
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert_eq!(
            realized_pnl_inr_long(10.0, 11.0, 2.0, 65),
            Some(130.0)
        );
        let engine = NfoRealizedPnlEngine::new();
        let result = engine.reconstruct(vec![
            nfo_fill("b1", "BUY", 2.0, 10.0, 65, "NRML", t(10, 0)),
            nfo_fill("s1", "SELL", 2.0, 11.0, 65, "NRML", t(11, 0)),
        ]);
        assert_eq!(result.round_trips.len(), 1);
        let rt = &result.round_trips[0];
        assert!((rt.realized_pnl_inr.unwrap() - 130.0).abs() < 1e-6);
    }

    #[test]
    fn refuses_cash_segment_and_spot_wac_path() {
        let engine = NfoRealizedPnlEngine::new();
        let mut cash = nfo_fill("c1", "BUY", 1.0, 100.0, 1, "CNC", t(10, 0));
        cash.exchange_segment = Some("nse_cm".to_string());
        cash.lot = Some(1);
        let result = engine.reconstruct(vec![cash]);
        assert!(result.round_trips.is_empty());
        assert_eq!(result.refused[0].reason, "segment_not_nse_fo");
        let rte = include_str!("round_trip_engine.rs");
        assert!(!rte.contains("kotak-nse-nfo"));
        assert!(!rte.contains("nfo_realized"));
    }

    #[test]
    fn refuses_nrml_on_wrong_segment_and_missing_lot() {
        let engine = NfoRealizedPnlEngine::new();
        let mut no_lot = nfo_fill("n1", "BUY", 1.0, 10.0, 65, "NRML", t(10, 0));
        no_lot.lot = None;
        let result = engine.reconstruct(vec![no_lot]);
        assert_eq!(result.refused[0].reason, "lot_missing");
    }

    #[test]
    fn dual_no_blend_inr_only() {
        let pnl = realized_pnl_inr_long(10.0, 11.0, 2.0, 65).unwrap();
        assert_ne!(pnl, pnl * 83.0);
    }
}
