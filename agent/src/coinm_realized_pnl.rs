//! Coin-M realized-PnL owner. Third identity — not USDM, not spot WAC.
//!
//! Lock: `issues/compliance/locks/binance-com-coinm.md` (fetch 2026-09-15 IST).

use serde_json::Value;

pub const OWNER_PATH: &str = "agent/src/coinm_realized_pnl.rs";

pub fn tick_from_exchange_info_filters(filters: &[Value]) -> Option<String> {
    crate::usdm_realized_pnl::tick_from_exchange_info_filters(filters)
}

pub fn parse_position_amt(raw: &str) -> Option<f64> {
    crate::usdm_realized_pnl::parse_position_amt(raw)
}

pub fn realized_pnl(_fills: &[()]) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn owner_is_third_identity() {
        assert_eq!(OWNER_PATH, "agent/src/coinm_realized_pnl.rs");
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert_ne!(OWNER_PATH, crate::usdm_realized_pnl::OWNER_PATH);
        assert!(realized_pnl(&[]).is_none());
    }

    #[test]
    fn tick_size_not_price_precision() {
        let filters = [json!({"filterType": "PRICE_FILTER", "tickSize": "0.001"})];
        assert_eq!(
            tick_from_exchange_info_filters(&filters).as_deref(),
            Some("0.001")
        );
    }

    #[test]
    fn qty_gt_1_position_amt() {
        assert_eq!(parse_position_amt("3"), Some(3.0));
        assert_ne!(parse_position_amt("3"), Some(1.0));
    }
}
