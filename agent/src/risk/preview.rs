//! Honest risk envelope for Plan — cites `docs/reference/behavioral-science/POSITION-SIZING.md` gaps.

use crate::ubi::catalog::{catalog_books, BrokerDescriptor};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct RiskPreviewInput {
    pub book_id: String,
    pub side: String,
    pub budget_mode: String,
    pub budget_value: Option<f64>,
    pub entry: Option<f64>,
    pub stop: Option<f64>,
    pub target: Option<f64>,
    pub override_qty: Option<f64>,
    pub funds_lit: bool,
    pub funds_balance: Option<f64>,
}

fn side_buy(side: &str) -> bool {
    !side.trim().eq_ignore_ascii_case("SELL")
}

fn catalog_row(book_id: &str) -> Option<BrokerDescriptor> {
    let needle = book_id.trim();
    catalog_books()
        .into_iter()
        .find(|b| b.book_id == needle)
}

fn qty_unit_for(profile_id: &str) -> &'static str {
    match profile_id {
        "equities_inr_nfo" | "commodity_inr_mcx" | "fx_cds_inr" => "lots",
        "crypto_usdm_usd" | "crypto_coinm_usd" => "contracts",
        "crypto_options_usd" => "contracts",
        _ => "shares",
    }
}

/// `units × (stop − entry) × sign`; sign +1 BUY, −1 SELL (matches Notch `BarPlanLadder`).
fn rung1(units: f64, entry: f64, stop: f64, side_buy: bool) -> Option<f64> {
    if units <= 0.0 || !entry.is_finite() || !stop.is_finite() {
        return None;
    }
    let sign = if side_buy { 1.0 } else { -1.0 };
    Some(units * (stop - entry) * sign)
}

fn max_planned_loss(rung1: Option<f64>) -> Option<f64> {
    let r = rung1?;
    if r >= 0.0 {
        return None;
    }
    Some(r.abs())
}

fn risk_reward(entry: f64, stop: f64, target: f64, side_buy: bool) -> Option<f64> {
    let risk = (entry - stop).abs();
    let reward = if side_buy {
        target - entry
    } else {
        entry - target
    };
    if risk <= 0.0 || reward <= 0.0 || !reward.is_finite() {
        return None;
    }
    Some(reward / risk)
}

fn primary_reason(input: &RiskPreviewInput, has_geometry: bool) -> &'static str {
    if input.book_id.trim().is_empty() {
        return "book_id_required";
    }
    if !input.funds_lit {
        return "funds_dark";
    }
    if !has_geometry {
        return "no_stop_or_entry";
    }
    "position_sizing_unspecified"
}

/// Build the preview JSON envelope (no HTTP).
pub fn compute_preview(input: RiskPreviewInput) -> Value {
    let row = catalog_row(&input.book_id);
    let quote_currency = row
        .as_ref()
        .map(|b| b.quote_currency.clone())
        .unwrap_or_default();
    let calc_profile = row
        .as_ref()
        .map(|b| b.calc_profile_id.clone())
        .unwrap_or_else(|| "unknown".into());
    let qty_unit = qty_unit_for(calc_profile.as_str());

    let entry = input.entry.filter(|v| v.is_finite() && *v > 0.0);
    let stop = input.stop.filter(|v| v.is_finite() && *v > 0.0);
    let target = input.target.filter(|v| v.is_finite() && *v > 0.0);
    let has_geometry = entry.is_some() && stop.is_some();
    let side_buy = side_buy(&input.side);

    let typed_qty = input
        .override_qty
        .filter(|q| q.is_finite() && *q > 0.0);
    let risk_money = typed_qty.and_then(|units| {
        max_planned_loss(rung1(
            units,
            entry?,
            stop?,
            side_buy,
        ))
    });

    let rr_ex_fees = match (entry, stop, target) {
        (Some(e), Some(s), Some(t)) => risk_reward(e, s, t, side_buy),
        _ => None,
    };

    let reason = primary_reason(&input, has_geometry);

    json!({
        "quote_currency": quote_currency,
        "calc_profile_id": calc_profile,
        "qty_unit": qty_unit,
        "authored_qty": Value::Null,
        "authored_qty_reason": "position_sizing_unspecified",
        "reason": reason,
        "budget_mode": input.budget_mode,
        "budget_value": input.budget_value,
        "funds_lit": input.funds_lit,
        "funds_balance": input.funds_balance,
        "risk_money": risk_money,
        "fees_money": Value::Null,
        "fees_reason": "fee_schedule_unspecified",
        "risk_plus_fees": Value::Null,
        "reward_minus_fees": Value::Null,
        "rr_ex_fees": rr_ex_fees,
        "rr_in_fees": Value::Null,
        "leverage": Value::Null,
        "contract": {
            "tick": Value::Null,
            "step": Value::Null,
            "multiplier": Value::Null,
            "multiplier_reason": "contract_spec_from_quote_envelope"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_qty_stays_null_without_reference_formula() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "binance-com-spot".into(),
            side: "BUY".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(100.0),
            stop: Some(90.0),
            target: Some(120.0),
            override_qty: None,
            funds_lit: true,
            funds_balance: Some(10_000.0),
        });
        assert!(out["authored_qty"].is_null());
        assert_eq!(
            out["authored_qty_reason"].as_str(),
            Some("position_sizing_unspecified")
        );
    }

    #[test]
    fn typed_qty_measures_risk_via_ladder() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "kotak-nse-bse-cash".into(),
            side: "BUY".into(),
            budget_mode: "fixed_money".into(),
            budget_value: Some(500.0),
            entry: Some(100.0),
            stop: Some(90.0),
            target: Some(120.0),
            override_qty: Some(10.0),
            funds_lit: true,
            funds_balance: Some(50_000.0),
        });
        assert_eq!(out["risk_money"].as_f64(), Some(100.0));
        assert_eq!(out["qty_unit"].as_str(), Some("shares"));
        assert!(out["rr_ex_fees"].as_f64().unwrap() > 1.9);
    }

    #[test]
    fn dark_funds_reason() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "binance-com-spot".into(),
            side: "BUY".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(2.0),
            entry: Some(1.0),
            stop: Some(0.9),
            target: None,
            override_qty: None,
            funds_lit: false,
            funds_balance: None,
        });
        assert_eq!(out["reason"].as_str(), Some("funds_dark"));
    }
}
