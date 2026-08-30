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

/// How the desk should present money when one or more connections are considered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeskHonesty {
    /// Single active (or homogeneous) currency — safe to format one hero number.
    Single(DeskProfile),
    /// Mixed USD+INR (or other) — do **not** blend; surface per-connection honesty.
    DualNoBlend { profiles: Vec<DeskProfile> },
    /// No usable catalog connection.
    None,
}

/// Resolve desk profile from a catalog slug (active sync).
pub fn desk_profile_for_slug(slug: Option<&str>) -> Option<DeskProfile> {
    let slug = slug?.trim();
    if slug.is_empty() {
        return None;
    }
    descriptor_for_slug(slug).map(|d| DeskProfile::from_descriptor(&d))
}

/// First-pair dogfood gate: COM → USD / crypto_spot_usd; Kotak → INR / equities_inr_cash.
pub fn desk_honesty_for_active_slugs(slugs: &[&str]) -> DeskHonesty {
    let mut profiles: Vec<DeskProfile> = slugs
        .iter()
        .filter_map(|s| desk_profile_for_slug(Some(s)))
        .collect();
    profiles.sort_by(|a, b| a.broker_slug.cmp(&b.broker_slug));
    profiles.dedup_by(|a, b| a.broker_slug == b.broker_slug);

    if profiles.is_empty() {
        return DeskHonesty::None;
    }
    if profiles.len() == 1 {
        return DeskHonesty::Single(profiles.remove(0));
    }
    let currencies: std::collections::BTreeSet<_> =
        profiles.iter().map(|p| p.quote_currency.as_str()).collect();
    if currencies.len() == 1 {
        // Homogeneous multi-connection — still one currency; use first for formatting.
        return DeskHonesty::Single(profiles.remove(0));
    }
    DeskHonesty::DualNoBlend { profiles }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binance_com_desk_is_usd_crypto_spot() {
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
        match desk_honesty_for_active_slugs(&["binance_com", "kotak_neo"]) {
            DeskHonesty::DualNoBlend { profiles } => {
                assert_eq!(profiles.len(), 2);
                let currencies: std::collections::BTreeSet<_> =
                    profiles.iter().map(|p| p.quote_currency.as_str()).collect();
                assert_eq!(currencies.len(), 2);
            }
            other => panic!("expected DualNoBlend, got {other:?}"),
        }
    }

    #[test]
    fn single_active_slug_is_honest_single() {
        match desk_honesty_for_active_slugs(&["binance_com"]) {
            DeskHonesty::Single(p) => assert_eq!(p.quote_currency, "USD"),
            other => panic!("expected Single, got {other:?}"),
        }
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
