//! Declared non-broker vendor bindings (S7 machine).
//!
//! One row is one B6 package. Adding a vendor is a sheet + a row here — not a
//! `pick_route` rewrite, not OpenBB/ODP, not a freetext host.

use super::amfi::{AMFI_ADAPTER_ID, AMFI_NAV_BOOK_ID};
use super::source_route::{
    LICENSED_HISTORY_ADAPTER_ID, LICENSED_HISTORY_BOOK_ID, PRODUCT_USE_LABS,
};

/// Published fixture meter. User may lower; PUT clamps above this.
pub const LICENSED_HISTORY_PUBLISHED_BUDGET: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorRole {
    ExplicitGap,
    SpecializedNonBroker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorBindingSpec {
    pub adapter_id: &'static str,
    pub book_id: &'static str,
    pub role: VendorRole,
    pub requires_key: bool,
    pub published_budget: u32,
    pub what: &'static str,
    pub obtain_noun: &'static str,
    pub product_use: &'static str,
}

pub const LICENSED_HISTORY_BINDING: VendorBindingSpec = VendorBindingSpec {
    adapter_id: LICENSED_HISTORY_ADAPTER_ID,
    book_id: LICENSED_HISTORY_BOOK_ID,
    role: VendorRole::ExplicitGap,
    requires_key: true,
    published_budget: LICENSED_HISTORY_PUBLISHED_BUDGET,
    what: "licensed_history India ohlcv (Kotak has none)",
    obtain_noun: "history",
    product_use: PRODUCT_USE_LABS,
};

pub const AMFI_BINDING: VendorBindingSpec = VendorBindingSpec {
    adapter_id: AMFI_ADAPTER_ID,
    book_id: AMFI_NAV_BOOK_ID,
    role: VendorRole::SpecializedNonBroker,
    requires_key: false,
    published_budget: 0,
    what: "AMFI NAV (labs)",
    obtain_noun: "amfi_nav",
    product_use: PRODUCT_USE_LABS,
};

/// Shipping declared bindings. Yahoo / OpenBB / Finnhub are not rows.
pub fn declared_vendor_bindings() -> &'static [VendorBindingSpec] {
    &[LICENSED_HISTORY_BINDING, AMFI_BINDING]
}

pub fn binding_for(adapter_id: &str) -> Option<&'static VendorBindingSpec> {
    declared_vendor_bindings()
        .iter()
        .find(|row| row.adapter_id == adapter_id)
}

pub fn clamp_budget(spec: &VendorBindingSpec, requested: Option<u32>) -> u32 {
    if spec.published_budget == 0 {
        return 0;
    }
    requested
        .unwrap_or(spec.published_budget)
        .min(spec.published_budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_licensed_history_and_amfi_only() {
        let ids: Vec<&str> = declared_vendor_bindings()
            .iter()
            .map(|row| row.adapter_id)
            .collect();
        assert_eq!(ids, vec!["licensed_history", "amfi"]);
        assert!(binding_for("yahoo").is_none());
        assert!(binding_for("openbb").is_none());
        assert!(binding_for("finnhub").is_none());
        assert!(binding_for("licensed_history").unwrap().requires_key);
        assert!(!binding_for("amfi").unwrap().requires_key);
    }

    #[test]
    fn published_budget_is_a_ceiling() {
        assert_eq!(clamp_budget(&LICENSED_HISTORY_BINDING, None), 60);
        assert_eq!(clamp_budget(&LICENSED_HISTORY_BINDING, Some(5)), 5);
        assert_eq!(clamp_budget(&LICENSED_HISTORY_BINDING, Some(9_000)), 60);
        assert_eq!(clamp_budget(&AMFI_BINDING, Some(99)), 0);
    }
}
