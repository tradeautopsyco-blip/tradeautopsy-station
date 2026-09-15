//! USDM realized-PnL owner. Not spot WAC. DualNoBlend vs INR NFO and Coin-M.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md` (fetch 2026-09-15 IST).
//! Venue `unRealizedProfit` on positionRisk may be copied for display. Realized
//! identity from `GET /fapi/v1/income` `REALIZED_PNL` stays later. This file owns
//! the refusal to let `round_trip_engine.rs` eat USDM.

use serde_json::Value;

pub const OWNER_PATH: &str = "agent/src/usdm_realized_pnl.rs";

/// Tick from `PRICE_FILTER.tickSize`. Never `pricePrecision`.
pub fn tick_from_exchange_info_filters(filters: &[Value]) -> Option<String> {
    for filter in filters {
        let filter_type = filter.get("filterType").and_then(Value::as_str);
        if filter_type == Some("PRICE_FILTER") {
            return filter
                .get("tickSize")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if filter.get("tickSize").and_then(Value::as_str).is_some() && filter_type.is_none() {
            return filter
                .get("tickSize")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
    }
    None
}

pub fn parse_position_amt(raw: &str) -> Option<f64> {
    let qty: f64 = raw.parse().ok()?;
    if qty.abs() <= f64::EPSILON {
        None
    } else {
        Some(qty)
    }
}

/// Desk realized PnL for USDM is not spot WAC. Until income `REALIZED_PNL` is
/// the locked identity, this owner returns none.
pub fn realized_pnl_usd(_fills: &[()]) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn owner_is_not_spot_wac() {
        assert_eq!(OWNER_PATH, "agent/src/usdm_realized_pnl.rs");
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert!(realized_pnl_usd(&[]).is_none());
    }

    #[test]
    fn tick_size_not_price_precision() {
        let filters = [
            json!({"filterType": "PRICE_FILTER", "tickSize": "0.10", "minPrice": "0.10"}),
            json!({"pricePrecision": 8}),
        ];
        assert_eq!(
            tick_from_exchange_info_filters(&filters).as_deref(),
            Some("0.10")
        );
        assert_ne!(
            tick_from_exchange_info_filters(&filters).as_deref(),
            Some("8")
        );
    }

    #[test]
    fn qty_gt_1_position_amt_does_not_collapse() {
        assert_eq!(parse_position_amt("2"), Some(2.0));
        assert_ne!(parse_position_amt("2"), Some(1.0));
        assert!(parse_position_amt("0").is_none());
    }
}
