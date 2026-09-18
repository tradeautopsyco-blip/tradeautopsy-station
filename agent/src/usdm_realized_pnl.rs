//! USDM realized-PnL owner. Not spot WAC. DualNoBlend vs INR NFO and Coin-M.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md` (fetch 2026-09-15 IST;
//! path oracle 2026-09-19 IST).
//! Identity: `GET /fapi/v1/income?incomeType=REALIZED_PNL` (SDK AccountApi
//! `get_income_history`, cite only — no cargo-depend). Venue `unRealizedProfit`
//! on positionRisk stays a published display string, not a second realized book.
//! COMMISSION is not this sum (fees later, same owner file). Force-order is
//! ineligible. TRADE is not this identity. This file owns the refusal to let
//! `round_trip_engine.rs` eat USDM.

use serde::Deserialize;
use serde_json::Value;

pub const OWNER_PATH: &str = "agent/src/usdm_realized_pnl.rs";

/// SDK `GetIncomeHistoryResponseInner` (cite). `income` is a string amount.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct UsdmIncomeRow {
    #[serde(rename = "incomeType", default)]
    pub income_type: Option<String>,
    #[serde(default)]
    pub income: Option<String>,
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub time: Option<i64>,
}

/// HMAC USER_DATA call this identity copies. Query key is `incomeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsdmIncomeCall {
    pub book_id: &'static str,
    pub host: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub query_income_type: &'static str,
}

pub fn usdm_income_call() -> UsdmIncomeCall {
    UsdmIncomeCall {
        book_id: "binance-com-usdm",
        host: "fapi.binance.com",
        method: "GET",
        path: "/fapi/v1/income",
        query_income_type: "REALIZED_PNL",
    }
}

/// Parsed venue realized. `None` means no REALIZED_PNL rows — do not claim 0.
#[derive(Debug, Clone, PartialEq)]
pub struct UsdmRealizedSlot {
    pub as_of_ms: i64,
    pub provenance_path: String,
    pub realized_pnl_usd: Option<f64>,
}

impl UsdmRealizedSlot {
    pub fn from_income_json(body: &str, as_of_ms: i64) -> Self {
        let call = usdm_income_call();
        Self {
            as_of_ms,
            provenance_path: call.path.to_string(),
            realized_pnl_usd: income_realized_from_json(body),
        }
    }

    pub fn published_realized_pnl_usd(&self) -> Option<f64> {
        self.realized_pnl_usd
    }
}

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

/// Sum `income` where `incomeType == "REALIZED_PNL"`. Other types are skipped.
/// Empty / no matching rows → `None` (honesty: do not claim 0 realized).
pub fn realized_pnl_usd(rows: &[UsdmIncomeRow]) -> Option<f64> {
    let mut sum = 0.0;
    let mut any = false;
    for row in rows {
        if row.income_type.as_deref() != Some("REALIZED_PNL") {
            continue;
        }
        let Some(raw) = row.income.as_deref() else {
            continue;
        };
        let Ok(amount) = raw.parse::<f64>() else {
            continue;
        };
        if !amount.is_finite() {
            continue;
        }
        sum += amount;
        any = true;
    }
    any.then_some(sum)
}

/// Parse a venue income JSON array. Invalid JSON or no REALIZED_PNL rows → `None`.
pub fn income_realized_from_json(body: &str) -> Option<f64> {
    let rows: Vec<UsdmIncomeRow> = serde_json::from_str(body).ok()?;
    realized_pnl_usd(&rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TWO_REALIZED: &str = r#"[
        {"symbol":"BTCUSDT","incomeType":"REALIZED_PNL","income":"10.5","asset":"USDT","time":1625474304765},
        {"symbol":"ETHUSDT","incomeType":"REALIZED_PNL","income":"-3.25","asset":"USDT","time":1625474304766}
    ]"#;

    const COMMISSION_ONLY: &str = r#"[
        {"symbol":"BTCUSDT","incomeType":"COMMISSION","income":"-0.0015","asset":"USDT","time":1625474304765}
    ]"#;

    const MIXED: &str = r#"[
        {"symbol":"BTCUSDT","incomeType":"REALIZED_PNL","income":"10.5","asset":"USDT","time":1},
        {"symbol":"BTCUSDT","incomeType":"COMMISSION","income":"-1.00","asset":"USDT","time":2},
        {"symbol":"ETHUSDT","incomeType":"FUNDING_FEE","income":"0.25","asset":"USDT","time":3},
        {"symbol":"ETHUSDT","incomeType":"REALIZED_PNL","income":"-3.25","asset":"USDT","time":4}
    ]"#;

    #[test]
    fn owner_is_not_spot_wac() {
        assert_eq!(OWNER_PATH, "agent/src/usdm_realized_pnl.rs");
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert!(realized_pnl_usd(&[]).is_none());
    }

    #[test]
    fn two_realized_pnl_rows_sum_to_7_25() {
        assert_eq!(income_realized_from_json(TWO_REALIZED), Some(7.25));
        let rows: Vec<UsdmIncomeRow> = serde_json::from_str(TWO_REALIZED).expect("fixture");
        assert_eq!(realized_pnl_usd(&rows), Some(7.25));
    }

    #[test]
    fn commission_only_is_not_realized() {
        assert!(income_realized_from_json(COMMISSION_ONLY).is_none());
        assert_ne!(income_realized_from_json(COMMISSION_ONLY), Some(-0.0015));
    }

    #[test]
    fn mixed_rows_copy_realized_pnl_only() {
        assert_eq!(income_realized_from_json(MIXED), Some(7.25));
        assert_ne!(income_realized_from_json(MIXED), Some(6.25));
        assert_ne!(income_realized_from_json(MIXED), Some(7.50));
    }

    #[test]
    fn empty_array_is_none_not_claimed_zero() {
        assert!(income_realized_from_json("[]").is_none());
        assert_ne!(income_realized_from_json("[]"), Some(0.0));
    }

    #[test]
    fn dual_no_blend_vs_spot_wac_and_inr() {
        let usd = income_realized_from_json(TWO_REALIZED).expect("realized");
        assert_eq!(usd, 7.25);
        assert_ne!(usd, 7.25 * 83.0);
        let call = usdm_income_call();
        assert_ne!(call.book_id, "binance-com-spot");
        assert_ne!(call.book_id, "kotak-nse-nfo");
        assert_ne!(call.host, "api.binance.com");
        assert_ne!(call.host, "dapi.binance.com");
        assert_ne!(call.host, "eapi.binance.com");
    }

    #[test]
    fn income_call_is_fapi_realized_pnl() {
        let call = usdm_income_call();
        assert_eq!(call.method, "GET");
        assert_eq!(call.path, "/fapi/v1/income");
        assert_eq!(call.query_income_type, "REALIZED_PNL");
        assert_eq!(call.host, "fapi.binance.com");
        assert_eq!(call.book_id, "binance-com-usdm");
        assert_ne!(call.path, "/fapi/v1/order");
        assert_ne!(call.path, "/fapi/v1/forceOrders");
        assert_ne!(call.query_income_type, "COMMISSION");
        assert_ne!(call.query_income_type, "TRADE");
    }

    #[test]
    fn unrealized_profit_and_force_order_are_not_realized() {
        let position_risk = r#"[{"symbol":"BTCUSDT","positionAmt":"2","unRealizedProfit":"99.5"}]"#;
        assert!(income_realized_from_json(position_risk).is_none());
        let force_order = r#"[{"symbol":"BTCUSDT","side":"SELL"}]"#;
        assert!(income_realized_from_json(force_order).is_none());
    }

    #[test]
    fn slot_reads_owner_sum_not_a_second_book() {
        let slot = UsdmRealizedSlot::from_income_json(TWO_REALIZED, 1);
        assert_eq!(slot.provenance_path, "/fapi/v1/income");
        assert_eq!(slot.published_realized_pnl_usd(), Some(7.25));
        let empty = UsdmRealizedSlot::from_income_json("[]", 1);
        assert!(empty.published_realized_pnl_usd().is_none());
    }

    #[test]
    fn round_trip_engine_does_not_ingest_usdm_book() {
        let src = include_str!("round_trip_engine.rs");
        assert!(!src.contains("binance-com-usdm"));
        assert!(!src.contains("usdm_realized"));
        assert!(!src.contains("/fapi/v1/income"));
        assert!(!src.contains("REALIZED_PNL"));
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
