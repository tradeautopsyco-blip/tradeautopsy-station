//! Dual-desk honesty helpers (R7 Phase 5).
//!
//! Format / calc profile follow the **active connection**. Dual sync with mixed
//! quote currencies must never FX-blend into one number.

use super::catalog::{descriptor_for_slug, BrokerDescriptor};

/// Resolved desk profile for a single active connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeskProfile {
    pub broker_slug: String,
    pub quote_currency: String,
    pub calc_profile_id: String,
}

impl DeskProfile {
    pub fn from_descriptor(d: &BrokerDescriptor) -> Self {
        Self {
            broker_slug: d.slug.clone(),
            quote_currency: d.quote_currency.clone(),
            calc_profile_id: d.calc_profile_id.clone(),
        }
    }
}

/// Resolve desk profile from a catalog slug (active sync).
pub fn desk_profile_for_slug(slug: Option<&str>) -> Option<DeskProfile> {
    let slug = slug?.trim();
    if slug.is_empty() {
        return None;
    }
    descriptor_for_slug(slug).map(|d| DeskProfile::from_descriptor(&d))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DualNoBlend lives in Swift `DeskHonesty`. This crate only supplies per-slug
    /// profiles; the test locks that mixed USD+INR slugs stay two currencies.
    fn dual_no_blend_currencies(slugs: &[&str]) -> std::collections::BTreeSet<String> {
        slugs
            .iter()
            .filter_map(|s| desk_profile_for_slug(Some(s)))
            .map(|p| p.quote_currency)
            .collect()
    }

    #[test]
    fn binance_com_desk_is_usd_spot() {
        let p = desk_profile_for_slug(Some("binance_com")).expect("com");
        assert_eq!(p.quote_currency, "USD");
        assert_eq!(p.calc_profile_id, "crypto_spot_usd");
    }

    #[test]
    fn kotak_neo_desk_is_inr_equities() {
        let p = desk_profile_for_slug(Some("kotak_neo")).expect("kotak");
        assert_eq!(p.quote_currency, "INR");
        assert_eq!(p.calc_profile_id, "equities_inr_cash");
    }

    #[test]
    fn dual_com_plus_kotak_never_blends() {
        let currencies = dual_no_blend_currencies(&["binance_com", "kotak_neo"]);
        assert_eq!(currencies.len(), 2);
        assert!(currencies.contains("USD"));
        assert!(currencies.contains("INR"));
    }

    #[test]
    fn single_active_slug_is_honest_single() {
        let currencies = dual_no_blend_currencies(&["binance_com"]);
        assert_eq!(currencies.len(), 1);
        assert!(currencies.contains("USD"));
    }

    #[test]
    fn first_pair_kill_and_desk_dogfood_ready() {
        // Automated dogfood gate: both venues have catalog desk + Kill hosts.
        for slug in ["binance_com", "kotak_neo"] {
            let desk = desk_profile_for_slug(Some(slug)).expect("desk");
            let hosts = crate::dns_block::hosts_for_broker(slug);
            assert!(!hosts.is_empty(), "{slug} Kill hosts missing");
            assert!(!desk.quote_currency.is_empty());
            assert!(!desk.calc_profile_id.is_empty());
            match slug {
                "binance_com" => {
                    assert_eq!(desk.quote_currency, "USD");
                    assert!(hosts.contains(&"api.binance.com"));
                }
                "kotak_neo" => {
                    assert_eq!(desk.quote_currency, "INR");
                    assert!(hosts.contains(&"mis.kotaksecurities.com"));
                }
                _ => unreachable!(),
            }
        }
    }
}
