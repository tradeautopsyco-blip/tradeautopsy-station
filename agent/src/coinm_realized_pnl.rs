//! Coin-M realized-PnL owner. Third identity — not USDM, not spot WAC.
//!
//! Lock: `issues/compliance/locks/binance-com-coinm.md` (fetch 2026-09-15 IST).
//! Identity: `GET /dapi/v1/income?incomeType=REALIZED_PNL` (SDK cite only).

use serde::Deserialize;
use serde_json::Value;

pub const OWNER_PATH: &str = "agent/src/coinm_realized_pnl.rs";

/// SDK `GetIncomeHistoryResponseInner` (cite). `income` is a string amount.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct CoinmIncomeRow {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinmIncomeCall {
    pub book_id: &'static str,
    pub host: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub query_income_type: &'static str,
}

pub fn coinm_income_call() -> CoinmIncomeCall {
    CoinmIncomeCall {
        book_id: "binance-com-coinm",
        host: "dapi.binance.com",
        method: "GET",
        path: "/dapi/v1/income",
        query_income_type: "REALIZED_PNL",
    }
}

pub fn tick_from_exchange_info_filters(filters: &[Value]) -> Option<String> {
    crate::usdm_realized_pnl::tick_from_exchange_info_filters(filters)
}

pub fn parse_position_amt(raw: &str) -> Option<f64> {
    crate::usdm_realized_pnl::parse_position_amt(raw)
}

pub fn realized_pnl_usd(rows: &[CoinmIncomeRow]) -> Option<f64> {
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

pub fn income_realized_from_json(body: &str) -> Option<f64> {
    let rows: Vec<CoinmIncomeRow> = serde_json::from_str(body).ok()?;
    realized_pnl_usd(&rows)
}

/// Parsed venue realized. `None` means no REALIZED_PNL rows — do not claim 0.
#[derive(Debug, Clone, PartialEq)]
pub struct CoinmRealizedSlot {
    pub as_of_ms: i64,
    pub provenance_path: String,
    pub realized_pnl_usd: Option<f64>,
    pub income_rows: Vec<CoinmIncomeRow>,
}

impl CoinmRealizedSlot {
    pub fn from_income_json(body: &str, as_of_ms: i64) -> Self {
        let call = coinm_income_call();
        let income_rows: Vec<CoinmIncomeRow> = serde_json::from_str(body).unwrap_or_default();
        Self {
            as_of_ms,
            provenance_path: call.path.to_string(),
            realized_pnl_usd: realized_pnl_usd(&income_rows),
            income_rows,
        }
    }

    pub fn published_realized_pnl_usd(&self) -> Option<f64> {
        self.realized_pnl_usd
    }
}

pub fn realized_pnl_rows_local_today(rows: &[CoinmIncomeRow]) -> Vec<CoinmIncomeRow> {
    use chrono::{Local, TimeZone, Utc};
    let today = Local::now().date_naive();
    rows.iter()
        .filter(|row| row.income_type.as_deref() == Some("REALIZED_PNL"))
        .filter(|row| {
            row.time
                .and_then(|ms| Utc.timestamp_millis_opt(ms).single())
                .map(|dt| dt.with_timezone(&Local).date_naive() == today)
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

pub fn aggregate_realized_pnl_usd_today(rows: &[CoinmIncomeRow]) -> Option<f64> {
    let today = realized_pnl_rows_local_today(rows);
    realized_pnl_usd(&today)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TWO_REALIZED: &str = r#"[
        {"symbol":"BTCUSD_PERP","incomeType":"REALIZED_PNL","income":"4.5","asset":"BTC","time":1},
        {"symbol":"ETHUSD_PERP","incomeType":"REALIZED_PNL","income":"-1.25","asset":"ETH","time":2}
    ]"#;

    const COMMISSION_ONLY: &str = r#"[
        {"symbol":"BTCUSD_PERP","incomeType":"COMMISSION","income":"-0.001","asset":"BTC","time":1}
    ]"#;

    #[test]
    fn owner_is_third_identity() {
        assert_eq!(OWNER_PATH, "agent/src/coinm_realized_pnl.rs");
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert_ne!(OWNER_PATH, crate::usdm_realized_pnl::OWNER_PATH);
        assert!(realized_pnl_usd(&[]).is_none());
        let rte = include_str!("round_trip_engine.rs");
        assert!(!rte.contains("binance-com-coinm"));
        assert!(!rte.contains("coinm_realized"));
        assert!(!rte.contains("/dapi/v1/income"));
    }

    #[test]
    fn two_realized_rows_sum() {
        assert_eq!(income_realized_from_json(TWO_REALIZED), Some(3.25));
    }

    #[test]
    fn commission_only_is_not_realized() {
        assert!(income_realized_from_json(COMMISSION_ONLY).is_none());
    }

    #[test]
    fn income_call_is_dapi_realized_pnl() {
        let call = coinm_income_call();
        assert_eq!(call.path, "/dapi/v1/income");
        assert_eq!(call.host, "dapi.binance.com");
        assert_eq!(call.book_id, "binance-com-coinm");
        assert_ne!(call.host, "fapi.binance.com");
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

    #[test]
    fn slot_reads_owner_sum() {
        let slot = CoinmRealizedSlot::from_income_json(TWO_REALIZED, 1);
        assert_eq!(slot.provenance_path, "/dapi/v1/income");
        assert_eq!(slot.published_realized_pnl_usd(), Some(3.25));
        assert_eq!(slot.income_rows.len(), 2);
    }
}
