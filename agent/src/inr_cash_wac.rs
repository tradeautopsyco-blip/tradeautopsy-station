//! India NSE/BSE cash INR WAC — sole realized-PnL owner for books `kotak-nse-bse-cash`
//! `zerodha-nse-bse-cash`, and `upstox-nse-bse-cash`.
//!
//! Locks: `issues/compliance/locks/kotak-nse-bse-cash.md` ·
//! `issues/compliance/locks/zerodha-nse-bse-cash.md` ·
//! `issues/compliance/locks/upstox-nse-bse-cash.md`.
//! Method: WAC. Currency: INR. Products: CNC + MIS only. Lot must be 1.
//! DualNoBlend: never write USD; never FX-blend fields on the trip struct.
//!
//! Do **not** feed these fills into `round_trip_engine.rs` (that file is COM USD-spot).

use crate::broker::BrokerFill;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

pub const OWNER_PATH: &str = "agent/src/inr_cash_wac.rs";
pub const BOOK_ID: &str = "kotak-nse-bse-cash";
pub const CALC_PROFILE_ID: &str = "equities_inr_cash";

const EPS: f64 = 1e-12;
/// Cash lock: NSE equity txn 0.00307%.
const NSE_EQ_TXN: f64 = 0.0000307;
/// SEBI ₹10 / crore.
const SEBI_RATE: f64 = 10.0 / 1e7;

#[derive(Debug, Clone, PartialEq)]
pub struct InrCashRoundTrip {
    pub symbol: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
    pub avg_entry_price: f64,
    pub avg_exit_price: f64,
    pub qty: f64,
    /// `None` when unknown_basis or refused fill quality.
    pub realized_pnl_inr: Option<f64>,
    pub fees_inr: Option<f64>,
    pub unknown_basis: bool,
    pub product: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InrCashReconstructResult {
    pub round_trips: Vec<InrCashRoundTrip>,
    pub refused: Vec<InrCashRefuse>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InrCashRefuse {
    pub fill_id: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct InrCashWacEngine;

impl InrCashWacEngine {
    pub fn new() -> Self {
        Self
    }

    /// Accept only cash-book fills. Others go to `refused` and are not ledgered.
    pub fn reconstruct(&self, fills: Vec<BrokerFill>) -> InrCashReconstructResult {
        let mut accepted = Vec::new();
        let mut refused = Vec::new();
        for fill in fills {
            match classify_cash_fill(&fill) {
                Ok(()) => accepted.push(fill),
                Err(reason) => refused.push(InrCashRefuse {
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

        let mut by_symbol: BTreeMap<String, SymbolLedger> = BTreeMap::new();
        let mut round_trips = Vec::new();

        for fill in accepted {
            let ledger = by_symbol
                .entry(fill.symbol.clone())
                .or_insert_with(SymbolLedger::default);
            if is_buy(&fill.side) {
                ledger.apply_buy(&fill);
            } else if is_sell(&fill.side) {
                ledger.apply_sell(&fill, &mut round_trips);
            }
        }

        round_trips.sort_by(|a, b| a.closed_at.cmp(&b.closed_at));
        InrCashReconstructResult {
            round_trips,
            refused,
        }
    }
}

/// True when this fill belongs to the INR cash book (not COM, not NFO).
pub fn is_inr_cash_fill(fill: &BrokerFill) -> bool {
    classify_cash_fill(fill).is_ok()
}

fn is_cash_segment(seg: &str) -> bool {
    matches!(seg, "nse_cm" | "bse_cm" | "nse" | "bse")
}

fn classify_cash_fill(fill: &BrokerFill) -> Result<(), &'static str> {
    let ccy = fill
        .currency
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase());
    match ccy.as_deref() {
        Some("INR") => {}
        Some(_) => return Err("currency_not_inr"),
        None => {
            // Infer from segment when currency omitted on cash path.
            let seg = fill
                .exchange_segment
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !is_cash_segment(&seg) {
                return Err("currency_missing_not_cash_segment");
            }
        }
    }

    let seg = fill
        .exchange_segment
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if !seg.is_empty() && !is_cash_segment(&seg) {
        return Err("segment_refused");
    }

    let product = fill
        .product
        .as_deref()
        .unwrap_or("MIS")
        .trim()
        .to_ascii_uppercase();
    if product == "NRML" || product == "CO" || product == "BO" {
        return Err("product_refused");
    }
    if product != "CNC"
        && product != "MIS"
        && product != "I"
        && product != "D"
    {
        return Err("product_unknown");
    }

    if let Some(lot) = fill.lot {
        if lot != 1 {
            return Err("lot_not_1");
        }
    }

    let inst = fill
        .instrument_type
        .as_deref()
        .unwrap_or("")
        .to_ascii_uppercase();
    if matches!(inst.as_str(), "CE" | "PE" | "FUT" | "FO" | "OPTIONS") {
        return Err("instrument_fo_refused");
    }

    Ok(())
}

pub fn is_aggregate_eligible(rt: &InrCashRoundTrip) -> bool {
    !rt.unknown_basis && rt.realized_pnl_inr.is_some()
}

pub fn aggregate_known_pnl_inr(round_trips: &[InrCashRoundTrip]) -> f64 {
    round_trips
        .iter()
        .filter(|rt| is_aggregate_eligible(rt))
        .filter_map(|rt| rt.realized_pnl_inr)
        .sum()
}

#[derive(Debug, Default)]
struct SymbolLedger {
    position_qty: f64,
    wac: f64,
    trip_buy_qty: f64,
    trip_buy_notional: f64,
    trip_sell_qty: f64,
    trip_sell_notional: f64,
    trip_fees_inr: f64,
    trip_opened_at: Option<DateTime<Utc>>,
    trip_product: String,
}

impl SymbolLedger {
    fn apply_buy(&mut self, fill: &BrokerFill) {
        if self.position_qty <= EPS {
            self.reset_trip();
            self.trip_opened_at = Some(fill.filled_at);
            self.trip_product = fill
                .product
                .clone()
                .unwrap_or_else(|| "MIS".to_string())
                .to_ascii_uppercase();
        }
        self.trip_fees_inr += fee_inr(fill);
        let new_qty = self.position_qty + fill.qty;
        self.wac = (self.position_qty * self.wac + fill.qty * fill.price) / new_qty;
        self.position_qty = new_qty;
        self.trip_buy_qty += fill.qty;
        self.trip_buy_notional += fill.qty * fill.price;
    }

    fn apply_sell(&mut self, fill: &BrokerFill, out: &mut Vec<InrCashRoundTrip>) {
        if self.position_qty <= EPS {
            out.push(unknown_basis(fill));
            return;
        }
        self.trip_fees_inr += fee_inr(fill);
        let sell_qty = fill.qty.min(self.position_qty);
        self.trip_sell_qty += sell_qty;
        self.trip_sell_notional += sell_qty * fill.price;
        self.position_qty -= sell_qty;
        if self.position_qty <= EPS {
            self.position_qty = 0.0;
            out.push(self.close_round_trip(&fill.symbol, fill.filled_at));
            self.wac = 0.0;
        }
    }

    fn reset_trip(&mut self) {
        self.trip_buy_qty = 0.0;
        self.trip_buy_notional = 0.0;
        self.trip_sell_qty = 0.0;
        self.trip_sell_notional = 0.0;
        self.trip_fees_inr = 0.0;
        self.trip_opened_at = None;
        self.trip_product.clear();
    }

    fn close_round_trip(&mut self, symbol: &str, closed_at: DateTime<Utc>) -> InrCashRoundTrip {
        let avg_entry = self.trip_buy_notional / self.trip_buy_qty;
        let avg_exit = self.trip_sell_notional / self.trip_sell_qty;
        let qty = self.trip_sell_qty;
        let fees = self.trip_fees_inr;
        let gross = qty * (avg_exit - avg_entry);
        let net = gross - fees;
        let opened_at = self.trip_opened_at.unwrap_or(closed_at);
        let product = if self.trip_product.is_empty() {
            "MIS".to_string()
        } else {
            self.trip_product.clone()
        };
        self.reset_trip();
        InrCashRoundTrip {
            symbol: symbol.to_string(),
            opened_at,
            closed_at,
            avg_entry_price: avg_entry,
            avg_exit_price: avg_exit,
            qty,
            realized_pnl_inr: Some(net),
            fees_inr: Some(fees),
            unknown_basis: false,
            product,
        }
    }
}

fn unknown_basis(fill: &BrokerFill) -> InrCashRoundTrip {
    InrCashRoundTrip {
        symbol: fill.symbol.clone(),
        opened_at: fill.filled_at,
        closed_at: fill.filled_at,
        avg_entry_price: 0.0,
        avg_exit_price: fill.price,
        qty: fill.qty,
        realized_pnl_inr: None,
        fees_inr: Some(fee_inr(fill)),
        unknown_basis: true,
        product: fill
            .product
            .clone()
            .unwrap_or_else(|| "MIS".to_string())
            .to_ascii_uppercase(),
    }
}

/// Prefer broker fill fee; else statutory cash leg (STT + NSE txn + SEBI + stamp on buy).
fn fee_inr(fill: &BrokerFill) -> f64 {
    if let Some(fee) = fill.fee_amount {
        if fee.is_finite() && fee >= 0.0 {
            return fee;
        }
    }
    statutory_leg_charges(fill)
}

fn statutory_leg_charges(fill: &BrokerFill) -> f64 {
    let t = fill.qty * fill.price;
    let product = fill
        .product
        .as_deref()
        .unwrap_or("MIS")
        .to_ascii_uppercase();
    let side = fill.side.to_ascii_uppercase();
    let mut stt = 0.0;
    let mut stamp = 0.0;
    let brokerage = if product == "CNC" {
        0.0
    } else {
        (t * 0.0003).min(20.0)
    };
    if product == "CNC" {
        stt = t * 0.001;
        if side == "BUY" {
            stamp = t * 0.00015;
        }
    } else {
        // MIS
        if side == "SELL" {
            stt = t * 0.00025;
        }
        if side == "BUY" {
            stamp = t * 0.00003;
        }
    }
    let exchange = t * NSE_EQ_TXN;
    let sebi = t * SEBI_RATE;
    let gst = (brokerage + exchange + sebi) * 0.18;
    brokerage + stt + exchange + sebi + stamp + gst
}

fn is_buy(side: &str) -> bool {
    side.eq_ignore_ascii_case("BUY")
}

fn is_sell(side: &str) -> bool {
    side.eq_ignore_ascii_case("SELL")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fill(
        id: &str,
        symbol: &str,
        side: &str,
        qty: f64,
        price: f64,
        at: DateTime<Utc>,
        product: &str,
    ) -> BrokerFill {
        BrokerFill {
            fill_id: id.to_string(),
            trade_id: format!("trade-{id}"),
            symbol: symbol.to_string(),
            side: side.to_string(),
            qty,
            price,
            filled_at: at,
            broker: "kotak_neo".to_string(),
            fee_amount: Some(0.0),
            fee_asset: Some("INR".to_string()),
            currency: Some("INR".to_string()),
            product: Some(product.to_string()),
            exchange_segment: Some("nse_cm".to_string()),
            instrument_type: Some("EQ".to_string()),
            lot: Some(1),
            ..Default::default()
        }
    }

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 22, h, m, 0).unwrap()
    }

    #[test]
    fn owner_path_is_not_round_trip_engine() {
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert_eq!(BOOK_ID, "kotak-nse-bse-cash");
    }

    #[test]
    fn wac_qty_gt_1_cash_round_trip() {
        let engine = InrCashWacEngine::new();
        let result = engine.reconstruct(vec![
            fill("b1", "RELIANCE", "BUY", 2.0, 1000.0, t(10, 0), "CNC"),
            fill("s1", "RELIANCE", "SELL", 2.0, 1100.0, t(11, 0), "CNC"),
        ]);
        assert_eq!(result.round_trips.len(), 1);
        let rt = &result.round_trips[0];
        assert!((rt.qty - 2.0).abs() < 1e-9);
        // fees forced 0 → net = 2 * (1100 - 1000) = 200
        assert!((rt.realized_pnl_inr.unwrap() - 200.0).abs() < 1e-6);
        assert!(rt.realized_pnl_inr.unwrap().is_finite());
    }

    #[test]
    fn refuses_nrml_and_lot_not_1() {
        let engine = InrCashWacEngine::new();
        let mut nrml = fill("n1", "RELIANCE", "BUY", 1.0, 100.0, t(10, 0), "NRML");
        let mut bad_lot = fill("l1", "RELIANCE", "BUY", 1.0, 100.0, t(10, 1), "CNC");
        bad_lot.lot = Some(25);
        let result = engine.reconstruct(vec![nrml, bad_lot]);
        assert!(result.round_trips.is_empty());
        assert_eq!(result.refused.len(), 2);
        assert_eq!(result.refused[0].reason, "product_refused");
        assert_eq!(result.refused[1].reason, "lot_not_1");
    }

    #[test]
    fn accepts_kite_nse_bse_exchange_segment() {
        let engine = InrCashWacEngine::new();
        let mut buy = fill("b1", "RELIANCE", "BUY", 1.0, 1000.0, t(10, 0), "CNC");
        buy.exchange_segment = Some("NSE".to_string());
        buy.currency = Some("INR".to_string());
        let mut sell = fill("s1", "RELIANCE", "SELL", 1.0, 1100.0, t(11, 0), "CNC");
        sell.exchange_segment = Some("NSE".to_string());
        sell.currency = Some("INR".to_string());
        let result = engine.reconstruct(vec![buy, sell]);
        assert_eq!(result.round_trips.len(), 1);
        assert!(result.refused.is_empty());
    }

    #[test]
    fn refuses_fo_instrument() {
        let engine = InrCashWacEngine::new();
        let mut fo = fill("f1", "NIFTY", "BUY", 1.0, 100.0, t(10, 0), "MIS");
        fo.instrument_type = Some("CE".to_string());
        fo.exchange_segment = Some("nse_fo".to_string());
        let result = engine.reconstruct(vec![fo]);
        assert!(result.refused.iter().any(|r| r.reason == "segment_refused"
            || r.reason == "instrument_fo_refused"));
    }

    #[test]
    fn dual_no_blend_struct_has_no_fx_blend_fields() {
        // Field names on InrCashRoundTrip — DualNoBlend forbids FX/USD money fields.
        let src = include_str!("inr_cash_wac.rs");
        let struct_start = src
            .find("pub struct InrCashRoundTrip")
            .expect("InrCashRoundTrip");
        let after = &src[struct_start..];
        let struct_end = after.find('}').expect("struct close");
        let body = &after[..struct_end];
        assert!(
            !body.contains("exchange_rate"),
            "InrCashRoundTrip must not carry FX blend fields"
        );
        assert!(!body.contains("realized_pnl_usd"));
        assert!(body.contains("realized_pnl_inr"));
    }

    #[test]
    fn round_trip_engine_source_must_not_claim_this_book() {
        let rte = include_str!("round_trip_engine.rs");
        assert!(
            !rte.contains("kotak-nse-bse-cash"),
            "COM USD engine must not claim cash book ownership"
        );
    }
}
