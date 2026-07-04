//! Spot round-trip reconstruction + WAC realized P&L (Today v1, Slice 1).
//!
//! Spec: `docs/reference/crypto/binance-us/spot/MECHANICS.md` §2–5

use crate::broker::BrokerFill;
use crate::exchange_info::{
    is_usd_pegged_stablecoin, is_usd_quoted_symbol, resolve_symbol_assets, ExchangeInfoSymbolCache,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

const EPS: f64 = 1e-12;

/// Closed round-trip row — matches MECHANICS.md §11 field set (+ `fee_unhandled`).
#[derive(Debug, Clone, PartialEq)]
pub struct RoundTrip {
    pub symbol: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
    pub avg_entry_price: f64,
    pub avg_exit_price: f64,
    pub qty: f64,
    /// `None` when `unknown_basis`, `fee_unhandled`, or `quote_not_usd` — UI shows `—`, not zero.
    pub realized_pnl_usd: Option<f64>,
    /// `None` when `quote_not_usd` — quote-asset fee totals must not wear a USD label.
    pub fees_usd: Option<f64>,
    pub unknown_basis: bool,
    pub fee_unhandled: bool,
    pub quote_not_usd: bool,
}

/// Fee charged in an asset the engine cannot convert to USD at fill time.
#[derive(Debug, Clone, PartialEq)]
pub struct UnhandledFee {
    pub fill_id: String,
    pub fee_asset: String,
    pub fee_amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconstructResult {
    pub round_trips: Vec<RoundTrip>,
    pub unhandled_fees: Vec<UnhandledFee>,
}

/// Fill-time USD price for fee normalization (MECHANICS.md §4).
pub trait FillTimeFeePriceLookup: Send + Sync {
    fn fee_usd(
        &self,
        fill: &BrokerFill,
        fee_amount: f64,
        fee_asset: &str,
    ) -> Result<f64, UnhandledFee>;

    /// Whether the pair's quote asset is USD-pegged (MECHANICS.md §3).
    fn is_usd_quoted_symbol(&self, symbol: &str) -> bool;
}

/// Fee lookup using exchangeInfo `baseAsset`/`quoteAsset` when cached; optional heuristic fallback.
#[derive(Debug, Clone)]
pub struct PairAssetFeeLookup {
    cache: ExchangeInfoSymbolCache,
    allow_heuristic_fallback: bool,
}

impl PairAssetFeeLookup {
    pub fn new(cache: ExchangeInfoSymbolCache, allow_heuristic_fallback: bool) -> Self {
        Self {
            cache,
            allow_heuristic_fallback,
        }
    }
}

impl Default for PairAssetFeeLookup {
    fn default() -> Self {
        Self::new(ExchangeInfoSymbolCache::empty(), true)
    }
}

impl FillTimeFeePriceLookup for PairAssetFeeLookup {
    fn fee_usd(
        &self,
        fill: &BrokerFill,
        fee_amount: f64,
        fee_asset: &str,
    ) -> Result<f64, UnhandledFee> {
        if fee_amount <= EPS {
            return Ok(0.0);
        }
        let asset = fee_asset.to_ascii_uppercase();
        if is_usd_pegged_stablecoin(&asset) {
            return Ok(fee_amount);
        }

        let (base, quote, used_heuristic) =
            resolve_symbol_assets(&self.cache, &fill.symbol, self.allow_heuristic_fallback)
                .ok_or_else(|| UnhandledFee {
                    fill_id: fill.fill_id.clone(),
                    fee_asset: fee_asset.to_string(),
                    fee_amount,
                })?;

        if used_heuristic {
            tracing::warn!(
                symbol = %fill.symbol,
                "symbol base/quote resolved via heuristic fallback — populate ExchangeInfoSymbolCache from GET /api/v3/exchangeInfo"
            );
        }

        if asset == base {
            return Ok(fee_amount * fill.price);
        }
        if asset == quote {
            return Ok(fee_amount);
        }

        Err(UnhandledFee {
            fill_id: fill.fill_id.clone(),
            fee_asset: fee_asset.to_string(),
            fee_amount,
        })
    }

    fn is_usd_quoted_symbol(&self, symbol: &str) -> bool {
        is_usd_quoted_symbol(&self.cache, symbol, self.allow_heuristic_fallback)
    }
}

/// Back-compat alias — prefer [`PairAssetFeeLookup`].
pub type StablecoinAndBaseAssetFeeLookup = PairAssetFeeLookup;

#[derive(Debug, Clone)]
pub struct RoundTripEngine<L: FillTimeFeePriceLookup = PairAssetFeeLookup> {
    fee_lookup: L,
}

impl RoundTripEngine<PairAssetFeeLookup> {
    pub fn new() -> Self {
        Self::with_exchange_info(ExchangeInfoSymbolCache::empty(), true)
    }

    pub fn with_exchange_info(
        cache: ExchangeInfoSymbolCache,
        allow_heuristic_fallback: bool,
    ) -> Self {
        Self {
            fee_lookup: PairAssetFeeLookup::new(cache, allow_heuristic_fallback),
        }
    }
}

impl<L: FillTimeFeePriceLookup> RoundTripEngine<L> {
    pub fn with_fee_lookup(fee_lookup: L) -> Self {
        Self { fee_lookup }
    }

    /// Reconstruct round-trips from a chronological fill stream (all symbols).
    pub fn reconstruct(&self, mut fills: Vec<BrokerFill>) -> ReconstructResult {
        fills.sort_by(|a, b| {
            a.filled_at
                .cmp(&b.filled_at)
                .then_with(|| a.fill_id.cmp(&b.fill_id))
        });

        let mut by_symbol: BTreeMap<String, SymbolLedger> = BTreeMap::new();
        let mut unhandled_fees = Vec::new();
        let mut round_trips = Vec::new();

        for fill in fills {
            let fee_outcome = normalize_fill_fee(&self.fee_lookup, &fill, &mut unhandled_fees);

            let ledger = by_symbol
                .entry(fill.symbol.clone())
                .or_insert_with(SymbolLedger::default);

            if is_buy(&fill.side) {
                ledger.apply_buy(&fill, fee_outcome);
            } else if is_sell(&fill.side) {
                let quote_not_usd = !self.fee_lookup.is_usd_quoted_symbol(&fill.symbol);
                ledger.apply_sell(&fill, fee_outcome, quote_not_usd, &mut round_trips);
            }
        }

        round_trips.sort_by(|a, b| a.closed_at.cmp(&b.closed_at));
        ReconstructResult {
            round_trips,
            unhandled_fees,
        }
    }
}

impl Default for RoundTripEngine<PairAssetFeeLookup> {
    fn default() -> Self {
        Self::new()
    }
}

/// Round-trips eligible for hero aggregate P&L (excludes honesty-flagged rows).
pub fn is_aggregate_eligible(rt: &RoundTrip) -> bool {
    !rt.unknown_basis && !rt.fee_unhandled && !rt.quote_not_usd
}

/// Sum realized P&L for aggregate-eligible round-trips only.
pub fn aggregate_known_pnl(round_trips: &[RoundTrip]) -> f64 {
    round_trips
        .iter()
        .filter(|rt| is_aggregate_eligible(rt))
        .filter_map(|rt| rt.realized_pnl_usd)
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FeeOutcome {
    Handled(f64),
    Unhandled,
}

#[derive(Debug, Default)]
struct SymbolLedger {
    position_qty: f64,
    wac: f64,
    trip_buy_qty: f64,
    trip_buy_notional: f64,
    trip_sell_qty: f64,
    trip_sell_notional: f64,
    trip_fees_usd: f64,
    trip_fee_unhandled: bool,
    trip_opened_at: Option<DateTime<Utc>>,
}

impl SymbolLedger {
    fn apply_buy(&mut self, fill: &BrokerFill, fee: FeeOutcome) {
        if self.position_qty <= EPS {
            self.reset_trip();
            self.trip_opened_at = Some(fill.filled_at);
        }
        self.record_fee(fee);

        let new_qty = self.position_qty + fill.qty;
        self.wac = (self.position_qty * self.wac + fill.qty * fill.price) / new_qty;
        self.position_qty = new_qty;
        self.trip_buy_qty += fill.qty;
        self.trip_buy_notional += fill.qty * fill.price;
    }

    fn apply_sell(
        &mut self,
        fill: &BrokerFill,
        fee: FeeOutcome,
        quote_not_usd: bool,
        out: &mut Vec<RoundTrip>,
    ) {
        if self.position_qty <= EPS {
            out.push(unknown_basis_round_trip(fill, fee, quote_not_usd));
            return;
        }

        self.record_fee(fee);
        let sell_qty = fill.qty.min(self.position_qty);
        self.trip_sell_qty += sell_qty;
        self.trip_sell_notional += sell_qty * fill.price;
        self.position_qty -= sell_qty;

        if self.position_qty <= EPS {
            self.position_qty = 0.0;
            out.push(self.close_round_trip(&fill.symbol, fill.filled_at, quote_not_usd));
            self.wac = 0.0;
        }
    }

    fn record_fee(&mut self, fee: FeeOutcome) {
        match fee {
            FeeOutcome::Handled(usd) => self.trip_fees_usd += usd,
            FeeOutcome::Unhandled => self.trip_fee_unhandled = true,
        }
    }

    fn reset_trip(&mut self) {
        self.trip_buy_qty = 0.0;
        self.trip_buy_notional = 0.0;
        self.trip_sell_qty = 0.0;
        self.trip_sell_notional = 0.0;
        self.trip_fees_usd = 0.0;
        self.trip_fee_unhandled = false;
        self.trip_opened_at = None;
    }

    fn close_round_trip(
        &mut self,
        symbol: &str,
        closed_at: DateTime<Utc>,
        quote_not_usd: bool,
    ) -> RoundTrip {
        let avg_entry = self.trip_buy_notional / self.trip_buy_qty;
        let avg_exit = self.trip_sell_notional / self.trip_sell_qty;
        let qty = self.trip_sell_qty;
        let fees_usd = self.trip_fees_usd;
        let fee_unhandled = self.trip_fee_unhandled;
        // MECHANICS.md §3: Realized PnL = sell_qty × (avg_sell − avg_buy), net of fees.
        let gross = qty * (avg_exit - avg_entry);
        let net = gross - fees_usd;
        let opened_at = self.trip_opened_at.unwrap_or(closed_at);

        self.reset_trip();

        RoundTrip {
            symbol: symbol.to_string(),
            opened_at,
            closed_at,
            avg_entry_price: avg_entry,
            avg_exit_price: avg_exit,
            qty,
            realized_pnl_usd: if fee_unhandled || quote_not_usd {
                None
            } else {
                Some(net)
            },
            fees_usd: if quote_not_usd { None } else { Some(fees_usd) },
            unknown_basis: false,
            fee_unhandled,
            quote_not_usd,
        }
    }
}

fn unknown_basis_round_trip(fill: &BrokerFill, fee: FeeOutcome, quote_not_usd: bool) -> RoundTrip {
    let (trip_fees_usd, fee_unhandled) = match fee {
        FeeOutcome::Handled(usd) => (usd, false),
        FeeOutcome::Unhandled => (0.0, true),
    };
    RoundTrip {
        symbol: fill.symbol.clone(),
        opened_at: fill.filled_at,
        closed_at: fill.filled_at,
        avg_entry_price: 0.0,
        avg_exit_price: fill.price,
        qty: fill.qty,
        realized_pnl_usd: None,
        fees_usd: if quote_not_usd {
            None
        } else {
            Some(trip_fees_usd)
        },
        unknown_basis: true,
        fee_unhandled,
        quote_not_usd,
    }
}

fn normalize_fill_fee<L: FillTimeFeePriceLookup>(
    lookup: &L,
    fill: &BrokerFill,
    unhandled: &mut Vec<UnhandledFee>,
) -> FeeOutcome {
    let (amount, asset) = match (fill.fee_amount, fill.fee_asset.as_deref()) {
        (Some(a), Some(asset)) if a > EPS => (a, asset),
        _ => return FeeOutcome::Handled(0.0),
    };
    match lookup.fee_usd(fill, amount, asset) {
        Ok(usd) => FeeOutcome::Handled(usd),
        Err(err) => {
            unhandled.push(err);
            FeeOutcome::Unhandled
        }
    }
}

fn is_buy(side: &str) -> bool {
    side.eq_ignore_ascii_case("BUY")
}

fn is_sell(side: &str) -> bool {
    side.eq_ignore_ascii_case("SELL")
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use chrono::TimeZone;

    fn fill(
        id: &str,
        symbol: &str,
        side: &str,
        qty: f64,
        price: f64,
        at: DateTime<Utc>,
        fee_amount: Option<f64>,
        fee_asset: Option<&str>,
    ) -> BrokerFill {
        BrokerFill {
            fill_id: id.to_string(),
            trade_id: format!("trade-{id}"),
            symbol: symbol.to_string(),
            side: side.to_string(),
            qty,
            price,
            filled_at: at,
            broker: "test".to_string(),
            fee_amount,
            fee_asset: fee_asset.map(str::to_string),
        }
    }

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 2, h, m, 0).unwrap()
    }

    #[test]
    fn wac_formula_matches_mechanics_section_3() {
        let engine = RoundTripEngine::new();
        let fills = vec![
            fill("b1", "BTCUSDT", "BUY", 1.0, 50_000.0, t(10, 0), None, None),
            fill("b2", "BTCUSDT", "BUY", 1.0, 60_000.0, t(10, 30), None, None),
            fill("s1", "BTCUSDT", "SELL", 2.0, 70_000.0, t(11, 0), None, None),
        ];
        let result = engine.reconstruct(fills);
        assert_eq!(result.round_trips.len(), 1);
        let rt = &result.round_trips[0];
        assert!((rt.avg_entry_price - 55_000.0).abs() < 1e-6);
        assert!((rt.avg_exit_price - 70_000.0).abs() < 1e-6);
        assert!((rt.qty - 2.0).abs() < 1e-6);
        assert!(!rt.fee_unhandled);
        // 2 × (70k − 55k) = 30k
        assert!((rt.realized_pnl_usd.unwrap() - 30_000.0).abs() < 1e-6);
    }
}
