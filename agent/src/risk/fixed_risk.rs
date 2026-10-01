//! Fixed-risk quantity from a stop.
//!
//! Identity copied from NautilusTrader `calculate_fixed_risk_position_size`
//! (`crates/risk/src/sizing.rs` on `develop`, last change
//! `fcd3c5e0f09b849ef665b2ac8eadb75db22c18ff`). Station does not link that crate.
//! `commission_rate` is 0 until a manual Station fee exists.
//! See `docs/reference/behavioral-science/POSITION-SIZING.md`.

/// Result of the Nautilus division chain. `None` means the inputs cannot run it.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedRiskQty {
    pub quantity: f64,
    pub risk_money: f64,
    /// `ok` or `below_unit_batch` when the floor is zero.
    pub reason: &'static str,
}

/// `risk` is a fraction (0.01 = 1%), matching Nautilus `Decimal` tests.
/// `exchange_rate` is 1 for a DualNoBlend book whose free figure is already the quote.
/// `unit_batch_size` of 0 skips the floor, as in `sizing.rs`.
pub fn fixed_risk_quantity(
    equity: f64,
    risk: f64,
    entry: f64,
    stop: f64,
    price_increment: f64,
    multiplier: f64,
    exchange_rate: f64,
    unit_batch_size: f64,
    units: u32,
) -> Option<FixedRiskQty> {
    if !risk.is_finite() || risk <= 0.0 {
        return None;
    }
    if !exchange_rate.is_finite() || exchange_rate < 0.0 {
        return None;
    }
    if !price_increment.is_finite() || price_increment <= 0.0 {
        return None;
    }
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return None;
    }
    if !unit_batch_size.is_finite() || unit_batch_size < 0.0 {
        return None;
    }
    if units == 0 {
        return None;
    }
    if !entry.is_finite() || !stop.is_finite() {
        return None;
    }

    // Source: exchange_rate == 0 → quantity zero.
    if exchange_rate == 0.0 {
        return Some(FixedRiskQty {
            quantity: 0.0,
            risk_money: 0.0,
            reason: "exchange_rate_zero",
        });
    }

    let risk_points = (entry - stop).abs() / price_increment;
    if !risk_points.is_finite() {
        return None;
    }
    // Source: risk_points <= 0 → quantity zero.
    if risk_points <= 0.0 {
        return Some(FixedRiskQty {
            quantity: 0.0,
            risk_money: 0.0,
            reason: "no_stop_distance",
        });
    }

    // Source `calculate_riskable_money`, commission_rate fixed at 0.
    let risk_money = if equity.is_finite() && equity > 0.0 {
        let gross = equity * risk;
        let commission = gross * 0.0 * 2.0;
        gross - commission
    } else {
        0.0
    };
    if !risk_money.is_finite() || risk_money < 0.0 {
        return None;
    }

    let position_size = risk_money / exchange_rate / risk_points / price_increment / multiplier;
    if !position_size.is_finite() || position_size < 0.0 {
        return None;
    }

    let mut batched = position_size / f64::from(units);
    if batched < 0.0 {
        batched = 0.0;
    }
    if unit_batch_size > 0.0 {
        batched = (batched / unit_batch_size).floor() * unit_batch_size;
    }
    if !batched.is_finite() || batched < 0.0 {
        return None;
    }

    let reason = if batched == 0.0 && position_size > 0.0 {
        "below_unit_batch"
    } else {
        "ok"
    };
    Some(FixedRiskQty {
        quantity: batched,
        risk_money,
        reason,
    })
}

/// OpenAlgo sandbox check after a size exists: `qty * price / leverage`.
/// Not a sizer. `None` leverage means the check does not run.
pub fn margin_required(quantity_for_margin: f64, price: f64, leverage: f64) -> Option<f64> {
    if quantity_for_margin < 0.0
        || !quantity_for_margin.is_finite()
        || !price.is_finite()
        || price <= 0.0
        || !leverage.is_finite()
        || leverage <= 0.0
    {
        return None;
    }
    let required = quantity_for_margin * price / leverage;
    required.is_finite().then_some(required)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplier_scales_like_nautilus_es_case() {
        // equity 1_000_000 × 0.01 = 10_000; $1 stop; multiplier 1000 → 10.
        let qty = fixed_risk_quantity(1_000_000.0, 0.01, 100.0, 99.0, 1.0, 1000.0, 1.0, 1.0, 1)
            .expect("qty");
        assert!((qty.quantity - 10.0).abs() < 1e-9);
        assert!((qty.risk_money - 10_000.0).abs() < 1e-6);
        assert_eq!(qty.reason, "ok");
    }

    #[test]
    fn price_increment_cancels_in_the_division_chain() {
        let a = fixed_risk_quantity(10_000.0, 0.01, 100.0, 90.0, 0.01, 1.0, 1.0, 0.0, 1).unwrap();
        let b = fixed_risk_quantity(10_000.0, 0.01, 100.0, 90.0, 0.05, 1.0, 1.0, 0.0, 1).unwrap();
        assert!((a.quantity - 10.0).abs() < 1e-9);
        assert!((b.quantity - 10.0).abs() < 1e-9);
    }

    #[test]
    fn commission_zero_does_not_reduce_risk_money() {
        let qty = fixed_risk_quantity(50_000.0, 0.02, 10.0, 9.0, 0.01, 1.0, 1.0, 1.0, 1).unwrap();
        assert!((qty.risk_money - 1_000.0).abs() < 1e-6);
    }

    #[test]
    fn batch_floor_zero_is_explicit() {
        let qty = fixed_risk_quantity(100.0, 0.01, 100.0, 90.0, 0.05, 1.0, 1.0, 1.0, 1).unwrap();
        assert_eq!(qty.quantity, 0.0);
        assert_eq!(qty.reason, "below_unit_batch");
    }

    #[test]
    fn margin_check_is_notional_over_leverage() {
        let required = margin_required(500.0, 100.0, 5.0).unwrap();
        assert!((required - 10_000.0).abs() < 1e-6);
        assert!(margin_required(10.0, 100.0, 0.0).is_none());
    }
}
