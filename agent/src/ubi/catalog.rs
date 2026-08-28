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
    /// Versioned SourceManifest id. Capabilities come from the manifest, not the slug.
    pub manifest_id: String,
    /// Lock book id (locks/binance-com-spot.md, locks/kotak-nse-bse-cash.md; fetch 2026-08-22 IST).
    /// Slug stays Wasm/Keychain/Start (`binance_com` / `kotak_neo`).
    pub book_id: String,
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

/// Catalog v1 — live first pair only (dogfood).
///
/// **Named next broker (T2.1):** `zerodha_kite` — B6 sheet stub only
/// (`issues/brokers/sheets/zerodha_kite.md`). Not in this catalog until SIGNED.
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
            manifest_id: "binance_com.s1.v1".into(),
            book_id: "binance-com-spot".into(),
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
            manifest_id: "kotak_neo.s1k.v1".into(),
            book_id: "kotak-nse-bse-cash".into(),
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
    fn catalog_v1_first_pair_only_enabled() {
        let cat = catalog_v1();
        assert_eq!(cat.len(), 2);
        let com = cat.iter().find(|d| d.slug == "binance_com").unwrap();
        let kotak = cat.iter().find(|d| d.slug == "kotak_neo").unwrap();
        assert_eq!(com.availability, BrokerAvailability::Enabled);
        assert_eq!(com.auth_scheme, AuthScheme::HmacApiKeySecret);
        assert_eq!(com.origin, AdapterOrigin::FirstParty);
        assert_eq!(com.manifest_id, "binance_com.s1.v1");
        assert_eq!(com.book_id, "binance-com-spot");
        assert_eq!(kotak.availability, BrokerAvailability::Enabled);
        assert_eq!(kotak.auth_scheme, AuthScheme::KotakNeoTotpSession);
        assert_eq!(kotak.quote_currency, "INR");
        assert_eq!(kotak.manifest_id, "kotak_neo.s1k.v1");
        assert_eq!(kotak.book_id, "kotak-nse-bse-cash");
        assert!(cat.iter().all(|d| d.origin == AdapterOrigin::FirstParty));
        assert!(descriptor_for_slug("binance_us").is_none());
        assert!(descriptor_for_slug("binance_com_usdm").is_none());
        assert!(descriptor_for_slug("zerodha_kite").is_none());
        assert!(descriptor_for_slug("interactive_brokers").is_none());
    }

    #[test]
    fn catalog_capabilities_come_from_manifest_not_slug() {
        use crate::data::{manifest_for_slug, obtain, ObtainStatus};
        for descriptor in catalog_v1() {
            let manifest =
                manifest_for_slug(&descriptor.slug).expect("enabled slug has a manifest");
            assert_eq!(manifest.manifest_id, descriptor.manifest_id);
            assert!(manifest.implemented.iter().any(|op| op == "quotes"));
            assert_eq!(
                obtain(&manifest, "quotes").status,
                ObtainStatus::Unavailable
            );
        }
    }

    /// B6 gate: only signed first-pair slugs may be Enabled. Named next
    /// (`zerodha_kite`) lives in issues sheets only until SIGNED — not here.
    #[test]
    fn b6_gate_only_signed_first_pair_is_enabled() {
        let enabled: Vec<_> = catalog_v1()
            .into_iter()
            .filter(|d| d.availability == BrokerAvailability::Enabled)
            .map(|d| d.slug)
            .collect();
        assert_eq!(
            enabled,
            vec!["binance_com".to_string(), "kotak_neo".to_string()]
        );
        assert!(catalog_v1()
            .iter()
            .all(|d| d.availability == BrokerAvailability::Enabled));
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
