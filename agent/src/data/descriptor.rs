//! One adapter binding descriptor (#363). Load-time shape only.

use super::identity::{CapabilityId, Family, Identity, Physics};
use super::rights::Rights;
use serde::{Deserialize, Serialize};

/// Delay class declared on the binding. Not a freshness extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelayClass {
    Realtime,
    Delayed,
    Eod,
    Unknown,
}

/// Station quota shape. Object required; fields may be omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Limits {
    #[serde(default)]
    pub requests_per_window: Option<u32>,
    #[serde(default)]
    pub window_secs: Option<u32>,
}

/// Tenant + broker account required in the contract when family is `account` (#362).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBinding {
    pub tenant_id: String,
    pub broker_account_id: String,
}

impl AccountBinding {
    pub fn is_complete(&self) -> bool {
        !self.tenant_id.trim().is_empty() && !self.broker_account_id.trim().is_empty()
    }
}

/// One installed binding. Rights may be absent on the wire; [`super::Registry::load`] refuses that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor {
    pub adapter_id: String,
    pub identity: Identity,
    /// Required at registration. `None` after JSON omit/`null` → [`super::Reject::RightsMissing`].
    #[serde(default)]
    pub rights: Option<Rights>,
    pub delay_class: DelayClass,
    pub limits: Limits,
    #[serde(default)]
    pub account: Option<AccountBinding>,
    /// Optional installed-source label. OpenBB / ODP here is forbidden (#365).
    #[serde(default)]
    pub installed_source: Option<String>,
}

pub const BINANCE_COM_ADAPTER_ID: &str = "binance_com";
pub const KOTAK_NEO_ADAPTER_ID: &str = "kotak_neo";
pub const ZERODHA_KITE_ADAPTER_ID: &str = "zerodha_kite";
/// Shipping TickBook slot for `binance_com` (locks/binance-com-spot.md).
pub const BINANCE_COM_SPOT_BOOK_ID: &str = "binance-com-spot";
/// Named options book (slice 1 last). Same `binance_com` slug; Start still ships spot.
pub const BINANCE_COM_OPTIONS_BOOK_ID: &str = "binance-com-options";
/// Shipping TickBook slot for `kotak_neo` (locks/kotak-nse-bse-cash.md).
pub const KOTAK_NSE_BSE_CASH_BOOK_ID: &str = "kotak-nse-bse-cash";
/// Named NFO book (Gate 0). Same `kotak_neo` slug; Start still ships cash.
pub const KOTAK_NSE_NFO_BOOK_ID: &str = "kotak-nse-nfo";
pub const KOTAK_NSE_CDS_BOOK_ID: &str = "kotak-nse-cds";
pub const KOTAK_MCX_FUTURE_BOOK_ID: &str = "kotak-mcx-future";
/// Shipping TickBook slot for `zerodha_kite` (B6 row 21 — founder may rename at Z9).
pub const ZERODHA_NSE_BSE_CASH_BOOK_ID: &str = "zerodha-nse-bse-cash";
/// Named NFO book on slug `zerodha_kite` (locks/zerodha-nse-nfo.md).
pub const ZERODHA_NSE_NFO_BOOK_ID: &str = "zerodha-nse-nfo";
pub const UPSTOX_NSE_BSE_CASH_BOOK_ID: &str = "upstox-nse-bse-cash";
/// Named NFO book on slug `upstox` (locks/upstox-nse-nfo.md).
pub const UPSTOX_NSE_NFO_BOOK_ID: &str = "upstox-nse-nfo";
pub const FYERS_NSE_BSE_CASH_BOOK_ID: &str = "fyers-nse-bse-cash";
/// Named NFO book on slug `fyers` (locks/fyers-nse-nfo.md).
pub const FYERS_NSE_NFO_BOOK_ID: &str = "fyers-nse-nfo";
/// Named USDM book. Same `binance_com` slug; Start still ships spot.
pub const BINANCE_COM_USDM_BOOK_ID: &str = "binance-com-usdm";
/// Named Coin-M book. Third identity. Same `binance_com` slug.
pub const BINANCE_COM_COINM_BOOK_ID: &str = "binance-com-coinm";

fn market_quote_latest_state() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("quote").expect("canonical quote id"),
        Physics::LatestState,
    )
}

/// Fixture `market/quote/latest_state` — no network, no vendor session.
pub fn fixture_quote_descriptor() -> Descriptor {
    Descriptor {
        adapter_id: "fixture_equity_quote".to_string(),
        identity: market_quote_latest_state(),
        rights: Some(Rights::research_fetch_only()),
        delay_class: DelayClass::Realtime,
        limits: Limits::default(),
        account: None,
        installed_source: None,
    }
}

/// Connected-broker last-price binding. Capability id stays `quote` (vendor is not an ID).
pub fn binance_com_quote_descriptor() -> Descriptor {
    Descriptor {
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        identity: market_quote_latest_state(),
        rights: Some(Rights::research_fetch_only()),
        delay_class: DelayClass::Realtime,
        limits: Limits::default(),
        account: None,
        installed_source: None,
    }
}

/// Kotak Neo last-price binding. Same `quote` / `latest_state` identity; PrivateRead fetch.
pub fn kotak_neo_quote_descriptor() -> Descriptor {
    Descriptor {
        adapter_id: KOTAK_NEO_ADAPTER_ID.to_string(),
        identity: market_quote_latest_state(),
        rights: Some(Rights::research_fetch_only()),
        delay_class: DelayClass::Realtime,
        limits: Limits::default(),
        account: None,
        installed_source: None,
    }
}

/// Fixture `account/funds/bounded_snapshot` with placeholder tenant + broker account.
#[cfg(test)]
pub fn fixture_account_descriptor() -> Descriptor {
    Descriptor {
        adapter_id: "fixture_broker_account".to_string(),
        identity: Identity::new(
            Family::Account,
            CapabilityId::new("funds").expect("canonical funds id"),
            Physics::BoundedSnapshot,
        ),
        rights: Some(Rights::research_fetch_only()),
        delay_class: DelayClass::Unknown,
        limits: Limits::default(),
        account: Some(AccountBinding {
            tenant_id: "fixture-tenant".to_string(),
            broker_account_id: "fixture-broker-account".to_string(),
        }),
        installed_source: None,
    }
}

pub(crate) fn forbids_openbb_source(adapter_id: &str, installed_source: Option<&str>) -> bool {
    forbidden_openbb_label(adapter_id)
        || installed_source
            .map(forbidden_openbb_label)
            .unwrap_or(false)
}

fn forbidden_openbb_label(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("openbb") {
        return true;
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|tok| tok == "odp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;

    #[test]
    fn openbb_equity_adapter_id_is_forbidden() {
        assert!(forbids_openbb_source("openbb_equity", None));
        assert!(forbids_openbb_source("fixture_quote", Some("ODP")));
        assert!(!forbids_openbb_source("fixture_equity_quote", None));
        assert!(!forbids_openbb_source(BINANCE_COM_ADAPTER_ID, None));
    }

    #[test]
    fn binance_com_quote_descriptor_loads() {
        let descriptor = binance_com_quote_descriptor();
        assert_eq!(descriptor.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(descriptor.identity.capability_id.as_str(), "quote");
        crate::data::Registry::load(&[descriptor]).expect("binance_com quote must register");
    }

    #[test]
    fn kotak_neo_quote_descriptor_loads() {
        let descriptor = kotak_neo_quote_descriptor();
        assert_eq!(descriptor.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_eq!(descriptor.identity.capability_id.as_str(), "quote");
        assert_eq!(descriptor.identity.physics, Physics::LatestState);
        crate::data::Registry::load(&[descriptor]).expect("kotak_neo quote must register");
        crate::data::Registry::load(&[
            binance_com_quote_descriptor(),
            kotak_neo_quote_descriptor(),
        ])
        .expect("binance and kotak quote bindings coexist");
    }
}
