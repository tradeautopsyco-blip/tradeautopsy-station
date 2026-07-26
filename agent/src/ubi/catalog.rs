//! UBI broker catalog + calc/compliance profile stubs (R7, Phase 2).

use serde::{Deserialize, Serialize};

/// Provenance only — never an execution privilege (ADR 0001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterOrigin {
    FirstParty,
    CommunityReviewed,
    CommunityUnreviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerAvailability {
    Enabled,
    Parked,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthScheme {
    HmacApiKeySecret,
    KotakNeoTotpSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerDescriptor {
    pub slug: String,
    pub display_name: String,
    pub asset_class: String,
    pub quote_currency: String,
    pub auth_scheme: AuthScheme,
    pub calc_profile_id: String,
    pub compliance_profile_id: String,
    pub availability: BrokerAvailability,
    pub origin: AdapterOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalcProfile {
    pub id: String,
    pub quote_currency: String,
    pub asset_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceProfile {
    pub id: String,
    /// Host-side connect refusal when withdraw permission is detected.
    pub block_on_withdraw: bool,
}

pub fn calc_profile(id: &str) -> Option<CalcProfile> {
    match id {
        "crypto_spot_usd" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "USD".into(),
            asset_class: "crypto_spot".into(),
        }),
        "equities_inr_cash" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "INR".into(),
            asset_class: "equities".into(),
        }),
        _ => None,
    }
}

pub fn compliance_profile(id: &str) -> Option<ComplianceProfile> {
    match id {
        "binance_com_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: true,
        }),
        "kotak_neo_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: false,
        }),
        _ => None,
    }
}

/// Catalog v1 — first pair enabled; binance_us parked; no community entries.
///
/// **Named next broker (T2.1):** `zerodha_kite` — B6 sheet stub only
/// (`issues/brokers/sheets/zerodha_kite.md`). Must stay `Planned` until that
/// sheet is `SIGNED`. Do not invent `community_reviewed`.
pub fn catalog_v1() -> Vec<BrokerDescriptor> {
    vec![
        BrokerDescriptor {
            slug: "binance_com".into(),
            display_name: "Binance.com".into(),
            asset_class: "crypto_spot".into(),
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "crypto_spot_usd".into(),
            compliance_profile_id: "binance_com_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
        },
        BrokerDescriptor {
            slug: "kotak_neo".into(),
            display_name: "Kotak Neo".into(),
            asset_class: "equities".into(),
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KotakNeoTotpSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "kotak_neo_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
        },
        BrokerDescriptor {
            slug: "binance_us".into(),
            display_name: "Binance.US".into(),
            asset_class: "crypto_spot".into(),
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "crypto_spot_usd".into(),
            compliance_profile_id: "binance_us_compliance".into(),
            availability: BrokerAvailability::Parked,
            origin: AdapterOrigin::FirstParty,
        },
        BrokerDescriptor {
            slug: "interactive_brokers".into(),
            display_name: "Interactive Brokers".into(),
            asset_class: "equities".into(),
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "equities_usd".into(),
            compliance_profile_id: String::new(),
            availability: BrokerAvailability::Planned,
            origin: AdapterOrigin::FirstParty,
        },
        BrokerDescriptor {
            slug: "zerodha_kite".into(),
            display_name: "Zerodha Kite".into(),
            asset_class: "equities".into(),
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: String::new(),
            availability: BrokerAvailability::Planned,
            origin: AdapterOrigin::FirstParty,
        },
    ]
}

pub fn descriptor_for_slug(slug: &str) -> Option<BrokerDescriptor> {
    catalog_v1().into_iter().find(|d| d.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_v1_first_pair_enabled_binance_us_parked() {
        let cat = catalog_v1();
        let com = cat.iter().find(|d| d.slug == "binance_com").unwrap();
        let kotak = cat.iter().find(|d| d.slug == "kotak_neo").unwrap();
        let us = cat.iter().find(|d| d.slug == "binance_us").unwrap();
        assert_eq!(com.availability, BrokerAvailability::Enabled);
        assert_eq!(com.auth_scheme, AuthScheme::HmacApiKeySecret);
        assert_eq!(com.origin, AdapterOrigin::FirstParty);
        assert_eq!(kotak.availability, BrokerAvailability::Enabled);
        assert_eq!(kotak.auth_scheme, AuthScheme::KotakNeoTotpSession);
        assert_eq!(kotak.quote_currency, "INR");
        assert_eq!(us.availability, BrokerAvailability::Parked);
        assert!(cat.iter().all(|d| d.origin == AdapterOrigin::FirstParty));
    }

    /// B6 gate: only signed first-pair slugs may be Enabled. Named next
    /// (`zerodha_kite`) and all other stubs stay Planned/Parked until a sheet
    /// is SIGNED — no Wasm ship for them.
    #[test]
    fn b6_gate_only_signed_first_pair_is_enabled() {
        let enabled: Vec<_> = catalog_v1()
            .into_iter()
            .filter(|d| d.availability == BrokerAvailability::Enabled)
            .map(|d| d.slug)
            .collect();
        assert_eq!(enabled, vec!["binance_com".to_string(), "kotak_neo".to_string()]);

        let zerodha = descriptor_for_slug("zerodha_kite").expect("named next in catalog");
        assert_eq!(zerodha.availability, BrokerAvailability::Planned);
        let ib = descriptor_for_slug("interactive_brokers").unwrap();
        assert_eq!(ib.availability, BrokerAvailability::Planned);
    }

    #[test]
    fn calc_profiles_cover_dual_desk() {
        assert_eq!(
            calc_profile("crypto_spot_usd").unwrap().quote_currency,
            "USD"
        );
        assert_eq!(
            calc_profile("equities_inr_cash").unwrap().quote_currency,
            "INR"
        );
    }
}
