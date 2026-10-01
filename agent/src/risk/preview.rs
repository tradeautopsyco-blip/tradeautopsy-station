//! Plan risk envelope. Quantity uses Nautilus fixed-risk when the profile is locked
//! and instrument fields are present. Fees stay at commission 0.
//! See `docs/reference/behavioral-science/POSITION-SIZING.md`.

use crate::risk::fixed_risk::{fixed_risk_quantity, margin_required};
use crate::ubi::{catalog_books, BrokerDescriptor};
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
    /// Venue tick. Required to run the Nautilus chain.
    pub price_increment: Option<f64>,
    /// Contract multiplier. Cash / spot / USDM lock uses 1 when omitted.
    pub multiplier: Option<f64>,
    /// Floor step. `None` selects the profile default (cash and NFO lots: 1, crypto: 0).
    pub unit_batch_size: Option<f64>,
    /// `cash` | `spot` | `usdm` | `coinm` | `nfo_future` | `option`.
    pub instrument_role: String,
    /// Venue leverage readout. Margin gate only. Never a sizing multiplier.
    pub leverage: Option<f64>,
    pub symbol: String,
}

impl Default for RiskPreviewInput {
    fn default() -> Self {
        Self {
            book_id: String::new(),
            side: "BUY".into(),
            budget_mode: "risk_percent".into(),
            budget_value: None,
            entry: None,
            stop: None,
            target: None,
            override_qty: None,
            funds_lit: false,
            funds_balance: None,
            price_increment: None,
            multiplier: None,
            unit_batch_size: None,
            instrument_role: String::new(),
            leverage: None,
            symbol: String::new(),
        }
    }
}

fn side_buy(side: &str) -> bool {
    !side.trim().eq_ignore_ascii_case("SELL")
}

fn catalog_row(book_id: &str) -> Option<BrokerDescriptor> {
    let needle = book_id.trim();
    catalog_books().into_iter().find(|b| b.book_id == needle)
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

fn positive(v: Option<f64>) -> Option<f64> {
    v.filter(|n| n.is_finite() && *n > 0.0)
}

/// Profiles the founder locked to the Nautilus identity (C1, C2, C7, C8).
fn locked_profile(profile_id: &str, role: &str) -> Result<&'static str, &'static str> {
    match profile_id {
        "equities_inr_cash" => Ok("cash"),
        "crypto_spot_usd" => Ok("spot"),
        "crypto_usdm_usd" => Ok("usdm"),
        "equities_inr_nfo" => match role {
            "nfo_future" => Ok("nfo_future"),
            "option" => Err("option_premium_unspecified"),
            _ => Err("instrument_role_required"),
        },
        "crypto_coinm_usd" => Err("coinm_identity_unspecified"),
        "crypto_options_usd" => Err("option_premium_unspecified"),
        "fx_cds_inr" | "commodity_inr_mcx" => Err("profile_not_locked"),
        _ => Err("profile_not_locked"),
    }
}

fn resolved_role<'a>(profile_id: &str, requested: &'a str) -> &'a str {
    if !requested.is_empty() {
        return requested;
    }
    match profile_id {
        "equities_inr_cash" => "cash",
        "crypto_spot_usd" => "spot",
        "crypto_usdm_usd" => "usdm",
        "crypto_coinm_usd" => "coinm",
        "crypto_options_usd" => "option",
        _ => "",
    }
}

struct Authored {
    qty: Option<f64>,
    reason: &'static str,
    risk_money: Option<f64>,
    multiplier: Option<f64>,
    price_increment: Option<f64>,
    resolved_quantity: Option<f64>,
    margin_gate: &'static str,
}

fn author_fixed_risk(input: &RiskPreviewInput, profile: &str, entry: f64, stop: f64) -> Authored {
    let dash = |reason: &'static str| Authored {
        qty: None,
        reason,
        risk_money: None,
        multiplier: positive(input.multiplier),
        price_increment: positive(input.price_increment),
        resolved_quantity: None,
        margin_gate: "not_applicable",
    };

    if input.book_id.trim().is_empty() {
        return dash("book_id_required");
    }
    if !input.funds_lit || positive(input.funds_balance).is_none() {
        return dash("funds_dark");
    }
    let role = resolved_role(profile, input.instrument_role.trim());
    if role == "option" && input.side.trim().eq_ignore_ascii_case("SELL") {
        return dash("option_short_max_loss_unspecified");
    }
    let kind = match locked_profile(profile, role) {
        Ok(kind) => kind,
        Err(reason) => return dash(reason),
    };
    if input.budget_mode.trim() != "risk_percent" {
        return dash("fixed_money_unspecified");
    }
    let Some(pct) = positive(input.budget_value) else {
        return dash("risk_pct_required");
    };
    let Some(tick) = positive(input.price_increment) else {
        return dash("price_increment_unspecified");
    };
    let multiplier = match kind {
        "cash" | "spot" | "usdm" => positive(input.multiplier).unwrap_or(1.0),
        "nfo_future" => match positive(input.multiplier) {
            Some(lot) => lot,
            None => return dash("multiplier_unspecified"),
        },
        _ => return dash("profile_not_locked"),
    };
    let equity = positive(input.funds_balance).expect("checked");
    let batch = input.unit_batch_size.unwrap_or(match kind {
        "spot" | "usdm" => 0.0,
        _ => 1.0,
    });
    let Some(sized) = fixed_risk_quantity(
        equity,
        pct / 100.0,
        entry,
        stop,
        tick,
        multiplier,
        1.0,
        batch,
        1,
    ) else {
        return dash("fixed_risk_inputs_rejected");
    };
    if sized.reason == "below_unit_batch" || sized.quantity <= 0.0 {
        return Authored {
            qty: Some(0.0),
            reason: if sized.reason == "ok" {
                "no_stop_distance"
            } else {
                sized.reason
            },
            risk_money: Some(sized.risk_money),
            multiplier: Some(multiplier),
            price_increment: Some(tick),
            resolved_quantity: None,
            margin_gate: "not_applicable",
        };
    }

    // India: Nautilus quantity is lots. OpenAlgo `resolve_quantity` is lots × lot_size
    // after the size exists. Margin uses that share quantity.
    let (resolved, qty_for_margin) = if kind == "nfo_future" {
        let shares = sized.quantity * multiplier;
        (Some(shares), shares)
    } else {
        (None, sized.quantity)
    };
    let india = kind == "nfo_future" || kind == "cash";
    let margin_gate = if !india {
        "not_applicable"
    } else {
        match positive(input.leverage).and_then(|lev| margin_required(qty_for_margin, entry, lev)) {
            None => "leverage_unspecified",
            Some(required) if required > equity => {
                return Authored {
                    qty: None,
                    reason: "margin_gate_exceeded",
                    risk_money: Some(sized.risk_money),
                    multiplier: Some(multiplier),
                    price_increment: Some(tick),
                    resolved_quantity: resolved,
                    margin_gate: "exceeded",
                };
            }
            Some(_) => "ok",
        }
    };

    Authored {
        qty: Some(sized.quantity),
        reason: "ok",
        risk_money: Some(sized.risk_money),
        multiplier: Some(multiplier),
        price_increment: Some(tick),
        resolved_quantity: resolved,
        margin_gate,
    }
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
    let side_buy = side_buy(&input.side);

    let typed_qty = input.override_qty.filter(|q| q.is_finite() && *q > 0.0);
    let ladder_risk =
        typed_qty.and_then(|units| max_planned_loss(rung1(units, entry?, stop?, side_buy)));

    let rr_ex_fees = match (entry, stop, target) {
        (Some(e), Some(s), Some(t)) => risk_reward(e, s, t, side_buy),
        _ => None,
    };

    let authored = match (entry, stop) {
        (Some(e), Some(s)) => author_fixed_risk(&input, &calc_profile, e, s),
        _ => Authored {
            qty: None,
            reason: if input.book_id.trim().is_empty() {
                "book_id_required"
            } else if !input.funds_lit {
                "funds_dark"
            } else {
                "no_stop_or_entry"
            },
            risk_money: None,
            multiplier: None,
            price_increment: None,
            resolved_quantity: None,
            margin_gate: "not_applicable",
        },
    };

    let risk_money = ladder_risk.or(authored.risk_money);

    json!({
        "quote_currency": quote_currency,
        "calc_profile_id": calc_profile,
        "qty_unit": qty_unit,
        "authored_qty": authored.qty,
        "authored_qty_reason": authored.reason,
        "reason": authored.reason,
        "budget_mode": input.budget_mode,
        "budget_value": input.budget_value,
        "funds_lit": input.funds_lit,
        "funds_balance": input.funds_balance,
        "risk_money": risk_money,
        "fixed_risk_money": authored.risk_money,
        "commission_rate": 0.0,
        "exchange_rate": 1.0,
        "fees_money": Value::Null,
        "fees_reason": "fee_schedule_unspecified",
        "risk_plus_fees": Value::Null,
        "reward_minus_fees": Value::Null,
        "rr_ex_fees": rr_ex_fees,
        "rr_in_fees": Value::Null,
        "resolved_quantity": authored.resolved_quantity,
        "margin_gate": authored.margin_gate,
        "leverage": input.leverage,
        "contract": {
            "tick": authored.price_increment,
            "step": input.unit_batch_size,
            "multiplier": authored.multiplier,
            "multiplier_reason": if authored.multiplier.is_some() {
                "fixed_risk_instrument"
            } else {
                "unspecified"
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_without_tick_stays_dashed() {
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
            ..RiskPreviewInput::default()
        });
        assert!(out["authored_qty"].is_null());
        assert_eq!(
            out["authored_qty_reason"].as_str(),
            Some("price_increment_unspecified")
        );
    }

    #[test]
    fn spot_fixed_risk_authors_base_qty() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "binance-com-spot".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(100.0),
            stop: Some(90.0),
            funds_lit: true,
            funds_balance: Some(10_000.0),
            price_increment: Some(0.01),
            ..RiskPreviewInput::default()
        });
        assert!((out["authored_qty"].as_f64().unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(out["commission_rate"].as_f64(), Some(0.0));
        assert_eq!(out["qty_unit"].as_str(), Some("shares"));
    }

    #[test]
    fn coinm_and_option_short_stay_unspecified() {
        let coinm = compute_preview(RiskPreviewInput {
            book_id: "binance-com-coinm".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(100.0),
            stop: Some(90.0),
            funds_lit: true,
            funds_balance: Some(1.0),
            price_increment: Some(0.1),
            multiplier: Some(100.0),
            ..RiskPreviewInput::default()
        });
        assert!(coinm["authored_qty"].is_null());
        assert_eq!(
            coinm["authored_qty_reason"].as_str(),
            Some("coinm_identity_unspecified")
        );

        let short_opt = compute_preview(RiskPreviewInput {
            book_id: "binance-com-options".into(),
            side: "SELL".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(0.02),
            stop: Some(0.04),
            funds_lit: true,
            funds_balance: Some(1_000.0),
            price_increment: Some(0.0001),
            instrument_role: "option".into(),
            ..RiskPreviewInput::default()
        });
        assert_eq!(
            short_opt["authored_qty_reason"].as_str(),
            Some("option_short_max_loss_unspecified")
        );
    }

    #[test]
    fn nfo_future_uses_lot_then_margin_gate() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "kotak-nse-nfo".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(100.0),
            stop: Some(90.0),
            funds_lit: true,
            funds_balance: Some(500_000.0),
            price_increment: Some(0.05),
            multiplier: Some(50.0),
            instrument_role: "nfo_future".into(),
            leverage: Some(5.0),
            ..RiskPreviewInput::default()
        });
        assert!((out["authored_qty"].as_f64().unwrap() - 10.0).abs() < 1e-6);
        assert!((out["resolved_quantity"].as_f64().unwrap() - 500.0).abs() < 1e-6);
        assert_eq!(out["margin_gate"].as_str(), Some("ok"));
        assert_eq!(out["qty_unit"].as_str(), Some("lots"));
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
            ..RiskPreviewInput::default()
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
            ..RiskPreviewInput::default()
        });
        assert_eq!(out["reason"].as_str(), Some("funds_dark"));
    }

    #[test]
    fn missing_stop_keeps_authored_null() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "binance-com-spot".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(100.0),
            stop: None,
            funds_lit: true,
            funds_balance: Some(10_000.0),
            price_increment: Some(0.01),
            ..RiskPreviewInput::default()
        });
        assert!(out["authored_qty"].is_null());
        assert_eq!(out["reason"].as_str(), Some("no_stop_or_entry"));
    }

    #[test]
    fn nfo_future_without_lot_stays_dashed() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "kotak-nse-nfo".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(1.0),
            entry: Some(100.0),
            stop: Some(90.0),
            funds_lit: true,
            funds_balance: Some(500_000.0),
            price_increment: Some(0.05),
            instrument_role: "nfo_future".into(),
            ..RiskPreviewInput::default()
        });
        assert!(out["authored_qty"].is_null());
        assert_eq!(
            out["authored_qty_reason"].as_str(),
            Some("multiplier_unspecified")
        );
    }

    #[test]
    fn margin_gate_exceeded_clears_qty() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "kotak-nse-bse-cash".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(50.0),
            entry: Some(100.0),
            stop: Some(99.0),
            funds_lit: true,
            funds_balance: Some(1_000.0),
            price_increment: Some(0.05),
            instrument_role: "cash".into(),
            leverage: Some(2.0),
            ..RiskPreviewInput::default()
        });
        assert!(out["authored_qty"].is_null());
        assert_eq!(
            out["authored_qty_reason"].as_str(),
            Some("margin_gate_exceeded")
        );
        assert_eq!(out["margin_gate"].as_str(), Some("exceeded"));
    }

    #[test]
    fn fixed_risk_money_matches_equity_times_percent() {
        let out = compute_preview(RiskPreviewInput {
            book_id: "binance-com-spot".into(),
            budget_mode: "risk_percent".into(),
            budget_value: Some(2.0),
            entry: Some(100.0),
            stop: Some(90.0),
            funds_lit: true,
            funds_balance: Some(50_000.0),
            price_increment: Some(0.01),
            ..RiskPreviewInput::default()
        });
        assert_eq!(out["fixed_risk_money"].as_f64(), Some(1_000.0));
        assert_eq!(out["commission_rate"].as_f64(), Some(0.0));
    }
}
