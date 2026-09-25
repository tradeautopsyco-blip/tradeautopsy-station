//! UBI broker catalog + calc/compliance profile stubs (R7, Phase 2; ADR 0004 taxonomy).
//!
//! Two catalogs, two keys:
//! - `catalog_v1()` — slug-keyed first pair (Binance.com spot, Kotak cash). Kept for
//!   slug-keyed callers (`desk.rs`, Swift session payloads). Unchanged shape in P1
//!   except the axes are typed (ADR 0004) instead of free strings.
//! - `catalog_books()` — book-keyed (ADR 0004). One row per SHIPPING book; the host
//!   stamps fill axes from the connection's book row, never from the slug row. This
//!   is what lifts the F3 second-book STOP: an NFO fill inherits nothing from cash.

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
    KiteChecksumSession,
    UpstoxOAuthBearerSession,
    FyersOAuthJsonAppIdHashSession,
    GrowwChecksumSession,
    DhanConsentSession,
    OkxPassphraseSession,
    KrakenSpotNonceSession,
    CoinbaseJwtEs256Session,
}

/// Closed asset axis (ADR 0004, Nautilus-aligned). Mirrors the WIT
/// `tradeautopsy:ubi-data/types.asset-class` enum; wire form is snake_case.
/// Unknown value = reject (`parse` returns `None`), never default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    Fx,
    Equity,
    Commodity,
    Debt,
    Index,
    Cryptocurrency,
    Alternative,
}

impl AssetClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fx => "fx",
            Self::Equity => "equity",
            Self::Commodity => "commodity",
            Self::Debt => "debt",
            Self::Index => "index",
            Self::Cryptocurrency => "cryptocurrency",
            Self::Alternative => "alternative",
        }
    }

    /// Parse a wire string. There is deliberately NO legacy mapping:
    /// `crypto_spot`, `equities`, `crypto_options`, `crypto_usdm`, `crypto_coinm`,
    /// `nfo`, `crypto` are all rejected (return `None`). Pre-bump components are
    /// rejected after migration, never silently translated (P1 law).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "fx" => Some(Self::Fx),
            "equity" => Some(Self::Equity),
            "commodity" => Some(Self::Commodity),
            "debt" => Some(Self::Debt),
            "index" => Some(Self::Index),
            "cryptocurrency" => Some(Self::Cryptocurrency),
            "alternative" => Some(Self::Alternative),
            _ => None,
        }
    }
}

impl std::fmt::Display for AssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Closed instrument axis (ADR 0004, Nautilus-aligned). Mirrors the WIT
/// `tradeautopsy:ubi-data/types.instrument-class` enum; wire form is snake_case
/// (`futures_spread`, `option_spread`, `sports_betting`, `binary_option`).
/// `is_inverse` (Coin-M) is a bool on the fill, never a member here. Unknown
/// value = reject (`parse` returns `None`), never default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentClass {
    Spot,
    Swap,
    Future,
    FuturesSpread,
    Forward,
    Cfd,
    Bond,
    Option,
    OptionSpread,
    Warrant,
    SportsBetting,
    BinaryOption,
}

impl InstrumentClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Swap => "swap",
            Self::Future => "future",
            Self::FuturesSpread => "futures_spread",
            Self::Forward => "forward",
            Self::Cfd => "cfd",
            Self::Bond => "bond",
            Self::Option => "option",
            Self::OptionSpread => "option_spread",
            Self::Warrant => "warrant",
            Self::SportsBetting => "sports_betting",
            Self::BinaryOption => "binary_option",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "spot" => Some(Self::Spot),
            "swap" => Some(Self::Swap),
            "future" => Some(Self::Future),
            "futures_spread" => Some(Self::FuturesSpread),
            "forward" => Some(Self::Forward),
            "cfd" => Some(Self::Cfd),
            "bond" => Some(Self::Bond),
            "option" => Some(Self::Option),
            "option_spread" => Some(Self::OptionSpread),
            "warrant" => Some(Self::Warrant),
            "sports_betting" => Some(Self::SportsBetting),
            "binary_option" => Some(Self::BinaryOption),
            _ => None,
        }
    }
}

impl std::fmt::Display for InstrumentClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerDescriptor {
    pub slug: String,
    pub display_name: String,
    /// Closed asset axis (ADR 0004). Was a free string in catalog v1.
    pub asset_class: AssetClass,
    /// Closed instrument axis (ADR 0004). Book-level stamp; see the NFO
    /// per-fill limitation in ADR 0004 §"NFO book stamp".
    pub instrument_class: InstrumentClass,
    /// Coin-M margining. A bool, never a class string (P1 law).
    pub is_inverse: bool,
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
    pub asset_class: AssetClass,
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
            asset_class: AssetClass::Cryptocurrency,
        }),
        "equities_inr_cash" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "INR".into(),
            asset_class: AssetClass::Equity,
        }),
        // ADR 0004: descriptive (quote, asset) rows for the second books. A calc
        // profile is NOT a money owner — NFO/options stay fills-display-only and
        // USDM/coin-M keep their income-file owners. No profile invents a rate.
        "crypto_options_usd" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "USD".into(),
            asset_class: AssetClass::Cryptocurrency,
        }),
        "crypto_usdm_usd" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "USD".into(),
            asset_class: AssetClass::Cryptocurrency,
        }),
        "crypto_coinm_usd" => Some(CalcProfile {
            id: id.into(),
            // Coin-M contracts are quoted in USD (BTCUSD_PERP) and margined in the
            // base asset; `is_inverse` on the descriptor carries the margining leg.
            quote_currency: "USD".into(),
            asset_class: AssetClass::Cryptocurrency,
        }),
        "equities_inr_nfo" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "INR".into(),
            asset_class: AssetClass::Equity,
        }),
        "fx_cds_inr" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "INR".into(),
            asset_class: AssetClass::Fx,
        }),
        "commodity_inr_mcx" => Some(CalcProfile {
            id: id.into(),
            quote_currency: "INR".into(),
            asset_class: AssetClass::Commodity,
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
        "zerodha_kite_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: false,
        }),
        "upstox_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: false,
        }),
        "fyers_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: false,
        }),
        "groww_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: false,
        }),
        "dhan_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: false,
        }),
        "bybit_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: true,
        }),
        "okx_com_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: true,
        }),
        "kraken_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: true,
        }),
        "coinbase_advanced_compliance" => Some(ComplianceProfile {
            id: id.into(),
            block_on_withdraw: true,
        }),
        _ => None,
    }
}

/// Catalog v1 — live first pair only (dogfood).
///
/// **Named next broker (T2.1):** `zerodha_kite` — B6 sheet stub only
/// (`issues/brokers/sheets/zerodha_kite.md`). Not in this catalog until SIGNED.
///
/// Slug-keyed on purpose: `desk.rs`, the Swift session payload, and the B6 gate
/// still resolve by slug. Book-keyed callers use [`catalog_books`] /
/// [`descriptor_for_book_id`].
pub fn catalog_v1() -> Vec<BrokerDescriptor> {
    vec![
        BrokerDescriptor {
            slug: "binance_com".into(),
            display_name: "Binance.com".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
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
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
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
    if let Some(d) = catalog_v1().into_iter().find(|d| d.slug == slug) {
        return Some(d);
    }
    catalog_books()
        .into_iter()
        .find(|d| {
            d.slug == slug
                && matches!(
                    d.availability,
                    BrokerAvailability::Planned | BrokerAvailability::Enabled
                )
        })
}

/// Book-keyed catalog (ADR 0004) — one row per SHIPPING book (CLAIM-REGISTRY.md:
/// exactly the 6 live books). The host stamps fill axes from the connection's
/// book row. Second books inherit nothing from the slug's first pair.
///
/// Mapping (asset, instrument, is_inverse) per P1 law:
/// - `binance-com-spot` → (cryptocurrency, spot, false)
/// - `binance-com-options` → (cryptocurrency, option, false)
/// - `binance-com-usdm` → (cryptocurrency, future, false)
/// - `binance-com-coinm` → (cryptocurrency, future, true)
/// - `kotak-nse-bse-cash` → (equity, spot, false)
/// - `kotak-nse-nfo` / `zerodha-nse-nfo` / `upstox-nse-nfo` / `fyers-nse-nfo` → (equity, option, false) + documented FUT limitation
///   (ADR 0004 §"NFO book stamp": the book is mixed CE/PE/FUT; per-fill precision
///   lives in `BrokerFill.instrument_type`, never in this stamp).
pub fn catalog_books() -> Vec<BrokerDescriptor> {
    vec![
        BrokerDescriptor {
            slug: "binance_com".into(),
            display_name: "Binance.com".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
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
            slug: "binance_com".into(),
            display_name: "Binance.com Options".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "crypto_options_usd".into(),
            compliance_profile_id: "binance_com_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "binance_com.options.v1".into(),
            book_id: "binance-com-options".into(),
        },
        BrokerDescriptor {
            slug: "binance_com".into(),
            display_name: "Binance.com USD-M".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Future,
            is_inverse: false,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "crypto_usdm_usd".into(),
            compliance_profile_id: "binance_com_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "binance_com.usdm.v1".into(),
            book_id: "binance-com-usdm".into(),
        },
        BrokerDescriptor {
            slug: "binance_com".into(),
            display_name: "Binance.com Coin-M".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Future,
            is_inverse: true,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "crypto_coinm_usd".into(),
            compliance_profile_id: "binance_com_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "binance_com.coinm.v1".into(),
            book_id: "binance-com-coinm".into(),
        },
        BrokerDescriptor {
            slug: "kotak_neo".into(),
            display_name: "Kotak Neo".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KotakNeoTotpSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "kotak_neo_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "kotak_neo.s1k.v1".into(),
            book_id: "kotak-nse-bse-cash".into(),
        },
        BrokerDescriptor {
            slug: "kotak_neo".into(),
            display_name: "Kotak Neo NFO".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KotakNeoTotpSession,
            calc_profile_id: "equities_inr_nfo".into(),
            compliance_profile_id: "kotak_neo_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "kotak_neo.nfo.v1".into(),
            book_id: "kotak-nse-nfo".into(),
        },
        BrokerDescriptor {
            slug: "kotak_neo".into(),
            display_name: "Kotak Neo CDS".into(),
            asset_class: AssetClass::Fx,
            instrument_class: InstrumentClass::Future,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KotakNeoTotpSession,
            calc_profile_id: "fx_cds_inr".into(),
            compliance_profile_id: "kotak_neo_compliance".into(),
            availability: BrokerAvailability::Planned,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "kotak_neo.cds.v1".into(),
            book_id: "kotak-nse-cds".into(),
        },
        BrokerDescriptor {
            slug: "kotak_neo".into(),
            display_name: "Kotak Neo MCX".into(),
            asset_class: AssetClass::Commodity,
            instrument_class: InstrumentClass::Future,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KotakNeoTotpSession,
            calc_profile_id: "commodity_inr_mcx".into(),
            compliance_profile_id: "kotak_neo_compliance".into(),
            availability: BrokerAvailability::Planned,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "kotak_neo.mcx.v1".into(),
            book_id: "kotak-mcx-future".into(),
        },
        BrokerDescriptor {
            slug: "zerodha_kite".into(),
            display_name: "Zerodha Kite".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KiteChecksumSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "zerodha_kite_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:zerodha-kite-cash@0.1.0".into(),
            book_id: "zerodha-nse-bse-cash".into(),
        },
        BrokerDescriptor {
            slug: "zerodha_kite".into(),
            display_name: "Zerodha Kite NFO".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::KiteChecksumSession,
            calc_profile_id: "equities_inr_nfo".into(),
            compliance_profile_id: "zerodha_kite_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:zerodha-kite-nfo@0.1.0".into(),
            book_id: "zerodha-nse-nfo".into(),
        },
        BrokerDescriptor {
            slug: "upstox".into(),
            display_name: "Upstox".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::UpstoxOAuthBearerSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "upstox_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:upstox-cash@0.1.0".into(),
            book_id: "upstox-nse-bse-cash".into(),
        },
        BrokerDescriptor {
            slug: "upstox".into(),
            display_name: "Upstox NFO".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::UpstoxOAuthBearerSession,
            calc_profile_id: "equities_inr_nfo".into(),
            compliance_profile_id: "upstox_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:upstox-nfo@0.1.0".into(),
            book_id: "upstox-nse-nfo".into(),
        },
        BrokerDescriptor {
            slug: "fyers".into(),
            display_name: "Fyers".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::FyersOAuthJsonAppIdHashSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "fyers_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:fyers-cash@0.1.0".into(),
            book_id: "fyers-nse-bse-cash".into(),
        },
        BrokerDescriptor {
            slug: "fyers".into(),
            display_name: "Fyers NFO".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::FyersOAuthJsonAppIdHashSession,
            calc_profile_id: "equities_inr_nfo".into(),
            compliance_profile_id: "fyers_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:fyers-nfo@0.1.0".into(),
            book_id: "fyers-nse-nfo".into(),
        },
        BrokerDescriptor {
            slug: "groww".into(),
            display_name: "Groww".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::GrowwChecksumSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "groww_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:groww-cash@0.1.0".into(),
            book_id: "groww-nse-bse-cash".into(),
        },
        BrokerDescriptor {
            slug: "dhan".into(),
            display_name: "Dhan".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::DhanConsentSession,
            calc_profile_id: "equities_inr_cash".into(),
            compliance_profile_id: "dhan_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:dhan-cash@0.1.0".into(),
            book_id: "dhan-nse-bse-cash".into(),
        },
        BrokerDescriptor {
            slug: "dhan".into(),
            display_name: "Dhan NFO".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::DhanConsentSession,
            calc_profile_id: "equities_inr_nfo".into(),
            compliance_profile_id: "dhan_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:dhan-nfo@0.1.0".into(),
            book_id: "dhan-nse-nfo".into(),
        },
        BrokerDescriptor {
            slug: "groww".into(),
            display_name: "Groww NFO".into(),
            asset_class: AssetClass::Equity,
            instrument_class: InstrumentClass::Option,
            is_inverse: false,
            quote_currency: "INR".into(),
            auth_scheme: AuthScheme::GrowwChecksumSession,
            calc_profile_id: "equities_inr_nfo".into(),
            compliance_profile_id: "groww_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:groww-nfo@0.1.0".into(),
            book_id: "groww-nse-nfo".into(),
        },
        BrokerDescriptor {
            slug: "bybit".into(),
            display_name: "Bybit".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::HmacApiKeySecret,
            calc_profile_id: "crypto_spot_usd".into(),
            compliance_profile_id: "bybit_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "bybit.spot.v1".into(),
            book_id: "bybit-com-spot".into(),
        },
        BrokerDescriptor {
            slug: "okx_com".into(),
            display_name: "OKX".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::OkxPassphraseSession,
            calc_profile_id: "crypto_spot_usd".into(),
            compliance_profile_id: "okx_com_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "okx_com.s1.v1".into(),
            book_id: "okx-com-spot".into(),
        },
        BrokerDescriptor {
            slug: "kraken".into(),
            display_name: "Kraken".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::KrakenSpotNonceSession,
            calc_profile_id: "crypto_spot_usd".into(),
            compliance_profile_id: "kraken_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "kraken.spot.v1".into(),
            book_id: "kraken-com-spot".into(),
        },
        BrokerDescriptor {
            slug: "coinbase_advanced".into(),
            display_name: "Coinbase Advanced".into(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            quote_currency: "USD".into(),
            auth_scheme: AuthScheme::CoinbaseJwtEs256Session,
            calc_profile_id: "crypto_spot_usd".into(),
            compliance_profile_id: "coinbase_advanced_compliance".into(),
            availability: BrokerAvailability::Enabled,
            origin: AdapterOrigin::FirstParty,
            manifest_id: "tradeautopsy:coinbase-advanced-spot@0.1.0".into(),
            book_id: "coinbase-advanced-spot".into(),
        },
    ]
}

/// Book-keyed lookup (ADR 0004). Unknown book = `None` (fail closed), never a
/// slug fallback: falling back to the slug's first pair would reintroduce F3.
pub fn descriptor_for_book_id(book_id: &str) -> Option<BrokerDescriptor> {
    let book_id = book_id.trim();
    if book_id.is_empty() {
        return None;
    }
    catalog_books().into_iter().find(|d| d.book_id == book_id)
}

/// SHIPPING book for the active connection desk (slug + calc profile). Used by M1 share-up.
pub fn book_id_for_slug_calc_profile(slug: &str, calc_profile_id: &str) -> Option<String> {
    let slug = slug.trim();
    let calc_profile_id = calc_profile_id.trim();
    if slug.is_empty() || calc_profile_id.is_empty() {
        return None;
    }
    catalog_books()
        .into_iter()
        .find(|d| d.slug == slug && d.calc_profile_id == calc_profile_id)
        .map(|d| d.book_id)
        .or_else(|| {
            catalog_v1()
                .into_iter()
                .find(|d| d.slug == slug && d.calc_profile_id == calc_profile_id)
                .map(|d| d.book_id)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_id_for_slug_calc_profile_resolves_second_books() {
        assert_eq!(
            book_id_for_slug_calc_profile("dhan", "equities_inr_nfo").as_deref(),
            Some("dhan-nse-nfo")
        );
        assert_eq!(
            book_id_for_slug_calc_profile("kotak_neo", "equities_inr_cash").as_deref(),
            Some("kotak-nse-bse-cash")
        );
        assert_eq!(
            book_id_for_slug_calc_profile("binance_com", "crypto_spot_usd").as_deref(),
            Some("binance-com-spot")
        );
    }

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
        let kite = descriptor_for_slug("zerodha_kite").expect("zerodha slug");
        assert_eq!(kite.availability, BrokerAvailability::Enabled);
        assert_eq!(kite.book_id, "zerodha-nse-bse-cash");
        assert_eq!(kite.auth_scheme, AuthScheme::KiteChecksumSession);
        let groww = descriptor_for_slug("groww").expect("groww slug");
        assert_eq!(groww.availability, BrokerAvailability::Enabled);
        assert_eq!(groww.book_id, "groww-nse-bse-cash");
        assert_eq!(groww.auth_scheme, AuthScheme::GrowwChecksumSession);
        let dhan = descriptor_for_slug("dhan").expect("dhan slug");
        assert_eq!(dhan.availability, BrokerAvailability::Enabled);
        assert_eq!(dhan.book_id, "dhan-nse-bse-cash");
        assert_eq!(dhan.auth_scheme, AuthScheme::DhanConsentSession);
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
    ///
    /// W0.8 (F6): Enabled ⇒ sheet SIGNED. Status lines are read at test time
    /// from the canonical sheets dir via a relative path (no prod-code coupling:
    /// this test breaks if an Enabled slug has no sheet or a non-SIGNED sheet).
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
        // W0.8a: Enabled is a subset of the signed first pair, and every
        // Enabled slug's canonical sheet carries a SIGNED Status line.
        // (Backticked `SIGNED` required: a RESEARCH sheet saying "not SIGNED"
        // must NOT satisfy this gate.)
        let signed_pair = ["binance_com", "kotak_neo"];
        assert!(
            enabled.iter().all(|s| signed_pair.contains(&s.as_str())),
            "Enabled slug outside the signed first pair: {enabled:?}"
        );
        let sheets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../issues/brokers/sheets");
        for slug in &enabled {
            let text = std::fs::read_to_string(sheets_dir.join(format!("{slug}.md")))
                .unwrap_or_else(|_| panic!("B6 sheet missing for Enabled slug {slug}"));
            let status = text
                .lines()
                .find(|l| l.contains("**Status:**"))
                .unwrap_or_else(|| panic!("{slug}.md has no Status line"));
            assert!(
                status.contains("`SIGNED`") && !status.contains("`RESEARCH`"),
                "Enabled slug {slug} sheet is not SIGNED: {status}"
            );
        }
    }

    /// ADR 0019: Connect-beta slugs — Enabled on book rows; not in `catalog_v1()` desk-live pair.
    #[test]
    fn adr_0019_connect_beta_slugs_enabled_on_book_rows() {
        const CONNECT_BETA: &[&str] = &[
            "zerodha_kite",
            "upstox",
            "fyers",
            "groww",
            "dhan",
            "bybit",
            "okx_com",
            "kraken",
            "coinbase_advanced",
        ];
        for slug in CONNECT_BETA {
            let d = descriptor_for_slug(slug).expect("connect beta slug");
            assert_eq!(
                d.availability,
                BrokerAvailability::Enabled,
                "connect beta slug must be Enabled on book row: {slug}"
            );
        }
        let v1_slugs: std::collections::BTreeSet<_> =
            catalog_v1().into_iter().map(|d| d.slug).collect();
        assert_eq!(v1_slugs.len(), 2);
        for slug in CONNECT_BETA {
            assert!(
                !v1_slugs.contains(*slug),
                "connect beta {slug} must not be catalog_v1 desk-live yet"
            );
        }
    }

    #[test]
    fn p4_crypto_spot_slugs_are_enabled_tier_ii() {
        for slug in ["bybit", "okx_com", "kraken", "coinbase_advanced"] {
            let d = descriptor_for_slug(slug).expect("p4 slug");
            assert_eq!(
                d.availability,
                BrokerAvailability::Enabled,
                "p4 slug must be Enabled: {slug}"
            );
        }
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

    // ---- ADR 0004 taxonomy + book-keyed catalog ----

    #[test]
    fn asset_class_wire_is_snake_case_closed() {
        assert_eq!(AssetClass::Cryptocurrency.as_str(), "cryptocurrency");
        assert_eq!(AssetClass::Equity.as_str(), "equity");
        assert_eq!(AssetClass::parse("cryptocurrency"), Some(AssetClass::Cryptocurrency));
        assert_eq!(AssetClass::parse("fx"), Some(AssetClass::Fx));
        assert_eq!(AssetClass::parse("alternative"), Some(AssetClass::Alternative));
    }

    #[test]
    fn instrument_class_wire_is_snake_case_closed() {
        assert_eq!(InstrumentClass::FuturesSpread.as_str(), "futures_spread");
        assert_eq!(InstrumentClass::OptionSpread.as_str(), "option_spread");
        assert_eq!(InstrumentClass::SportsBetting.as_str(), "sports_betting");
        assert_eq!(InstrumentClass::BinaryOption.as_str(), "binary_option");
        assert_eq!(
            InstrumentClass::parse("futures_spread"),
            Some(InstrumentClass::FuturesSpread)
        );
        assert_eq!(InstrumentClass::parse("option"), Some(InstrumentClass::Option));
    }

    #[test]
    fn legacy_strings_are_rejected_never_mapped() {
        // P1 law: no silent legacy-string mapping. Every pre-bump noun dies here.
        for legacy in [
            "crypto_spot",
            "equities",
            "crypto_options",
            "crypto_usdm",
            "crypto_coinm",
            "nfo",
            "crypto",
        ] {
            assert_eq!(AssetClass::parse(legacy), None, "legacy {legacy} must reject");
            assert_eq!(
                InstrumentClass::parse(legacy),
                None,
                "legacy {legacy} must reject"
            );
        }
        assert_eq!(AssetClass::parse(""), None);
        assert_eq!(AssetClass::parse("CryptoCurrency"), None);
        assert_eq!(InstrumentClass::parse(""), None);
        assert_eq!(InstrumentClass::parse("FUT"), None);
    }

    #[test]
    /// ADR 0019 Tier II: only first-pair + Kotak/Binance multi-book rows are Enabled.
    /// Tier I Wave 2/3 tracers stay Planned until signed dogfood (step 6).
    fn book_catalog_covers_shipping_books_adr_0019_tiers() {
        let books = catalog_books();
        assert_eq!(books.len(), 22);
        let ids: Vec<&str> = books.iter().map(|d| d.book_id.as_str()).collect();
        for expected in [
            "binance-com-spot",
            "binance-com-options",
            "binance-com-usdm",
            "binance-com-coinm",
            "kotak-nse-bse-cash",
            "kotak-nse-nfo",
            "kotak-nse-cds",
            "kotak-mcx-future",
            "zerodha-nse-bse-cash",
            "zerodha-nse-nfo",
            "upstox-nse-bse-cash",
            "upstox-nse-nfo",
            "fyers-nse-bse-cash",
            "fyers-nse-nfo",
            "groww-nse-bse-cash",
            "groww-nse-nfo",
            "dhan-nse-bse-cash",
            "dhan-nse-nfo",
            "bybit-com-spot",
            "okx-com-spot",
            "kraken-com-spot",
            "coinbase-advanced-spot",
        ] {
            assert!(ids.contains(&expected), "missing book row {expected}");
        }
        let tier_ii: Vec<_> = books
            .iter()
            .filter(|d| d.availability == BrokerAvailability::Enabled)
            .collect();
        assert_eq!(
            tier_ii.len(),
            20,
            "binance×4 + kotak×2 + p4×4 + wave2×5 cash + NFO×5 (incl. dhan/groww P6-W2)"
        );
        for (book, scheme) in [
            ("bybit-com-spot", AuthScheme::HmacApiKeySecret),
            ("okx-com-spot", AuthScheme::OkxPassphraseSession),
            ("kraken-com-spot", AuthScheme::KrakenSpotNonceSession),
            (
                "coinbase-advanced-spot",
                AuthScheme::CoinbaseJwtEs256Session,
            ),
        ] {
            let row = descriptor_for_book_id(book).expect("p4 crypto book");
            assert_eq!(row.availability, BrokerAvailability::Enabled);
            assert_eq!(row.auth_scheme, scheme);
            assert_eq!(row.calc_profile_id, "crypto_spot_usd");
        }
        let zerodha = descriptor_for_book_id("zerodha-nse-bse-cash").expect("zerodha book");
        assert_eq!(zerodha.availability, BrokerAvailability::Enabled);
        let upstox = descriptor_for_book_id("upstox-nse-bse-cash").expect("upstox book");
        assert_eq!(upstox.availability, BrokerAvailability::Enabled);
        assert_eq!(upstox.auth_scheme, AuthScheme::UpstoxOAuthBearerSession);
        let fyers = descriptor_for_book_id("fyers-nse-bse-cash").expect("fyers book");
        assert_eq!(fyers.availability, BrokerAvailability::Enabled);
        assert_eq!(
            fyers.auth_scheme,
            AuthScheme::FyersOAuthJsonAppIdHashSession
        );
        let groww = descriptor_for_book_id("groww-nse-bse-cash").expect("groww book");
        assert_eq!(groww.availability, BrokerAvailability::Enabled);
        assert_eq!(groww.slug, "groww");
        assert_eq!(groww.display_name, "Groww");
        assert_eq!(groww.auth_scheme, AuthScheme::GrowwChecksumSession);
        assert_eq!(groww.quote_currency, "INR");
        assert_eq!(groww.calc_profile_id, "equities_inr_cash");
        assert_eq!(groww.manifest_id, "tradeautopsy:groww-cash@0.1.0");
        let dhan = descriptor_for_book_id("dhan-nse-bse-cash").expect("dhan book");
        assert_eq!(dhan.availability, BrokerAvailability::Enabled);
        assert_eq!(dhan.slug, "dhan");
        let cds = descriptor_for_book_id("kotak-nse-cds").expect("kotak cds book");
        assert_eq!(cds.availability, BrokerAvailability::Planned);
        assert_eq!(cds.asset_class, AssetClass::Fx);
        assert_eq!(cds.instrument_class, InstrumentClass::Future);
        assert_eq!(cds.calc_profile_id, "fx_cds_inr");
        assert_eq!(dhan.auth_scheme, AuthScheme::DhanConsentSession);
        // Every book row's profiles resolve; every row is first-party.
        for d in &books {
            assert!(
                calc_profile(&d.calc_profile_id).is_some(),
                "book {} calc profile {} must resolve",
                d.book_id,
                d.calc_profile_id
            );
            assert!(
                compliance_profile(&d.compliance_profile_id).is_some(),
                "book {} compliance profile {} must resolve",
                d.book_id,
                d.compliance_profile_id
            );
            assert_eq!(d.origin, AdapterOrigin::FirstParty);
        }
    }

    #[test]
    fn book_rows_stamp_p1_mapping_intent() {
        let stamp = |book: &str| {
            let d = descriptor_for_book_id(book).expect("book row");
            (d.asset_class, d.instrument_class, d.is_inverse)
        };
        assert_eq!(
            stamp("binance-com-spot"),
            (
                AssetClass::Cryptocurrency,
                InstrumentClass::Spot,
                false
            )
        );
        assert_eq!(
            stamp("binance-com-options"),
            (
                AssetClass::Cryptocurrency,
                InstrumentClass::Option,
                false
            )
        );
        assert_eq!(
            stamp("binance-com-usdm"),
            (
                AssetClass::Cryptocurrency,
                InstrumentClass::Future,
                false
            )
        );
        assert_eq!(
            stamp("binance-com-coinm"),
            (AssetClass::Cryptocurrency, InstrumentClass::Future, true)
        );
        assert_eq!(
            stamp("kotak-nse-bse-cash"),
            (AssetClass::Equity, InstrumentClass::Spot, false)
        );
        // NFO stamps NFO axes, not the slug's cash/equities pair — the F3 flip at
        // catalog level. FUT-leg limitation documented in ADR 0004 §"NFO book stamp".
        assert_eq!(
            stamp("kotak-nse-nfo"),
            (AssetClass::Equity, InstrumentClass::Option, false)
        );
        assert_eq!(
            stamp("kotak-nse-cds"),
            (AssetClass::Fx, InstrumentClass::Future, false)
        );
        assert_eq!(
            stamp("kotak-mcx-future"),
            (AssetClass::Commodity, InstrumentClass::Future, false)
        );
    }

    #[test]
    fn book_lookup_fails_closed_without_slug_fallback() {
        assert!(descriptor_for_book_id("").is_none());
        assert!(descriptor_for_book_id("   ").is_none());
        assert!(descriptor_for_book_id("binance_com").is_none());
        assert!(descriptor_for_book_id("kotak_neo").is_none());
        assert!(descriptor_for_book_id("binance-com-stocks").is_none());
        assert!(descriptor_for_book_id("crypto_spot").is_none());
        // No slug fallback: looking up a slug must not return the first pair.
        assert!(descriptor_for_book_id("binance-com-spot-extra").is_none());
    }

    #[test]
    fn book_rows_agree_with_source_manifests() {
        use crate::data::manifest_for_book_id;
        for descriptor in catalog_books() {
            if descriptor.availability == BrokerAvailability::Planned {
                continue;
            }
            let manifest = manifest_for_book_id(&descriptor.book_id)
                .unwrap_or_else(|| panic!("book {} has no manifest", descriptor.book_id));
            assert_eq!(
                manifest.manifest_id, descriptor.manifest_id,
                "book {} manifest drift",
                descriptor.book_id
            );
            assert_eq!(
                manifest.adapter_id, descriptor.slug,
                "book {} adapter drift",
                descriptor.book_id
            );
        }
    }

    #[test]
    fn slug_catalog_still_resolves_first_pair_for_unmigrated_callers() {
        // desk.rs + Swift session payloads stay slug-keyed in P1 (ADR 0004).
        let com = descriptor_for_slug("binance_com").expect("com");
        assert_eq!(com.book_id, "binance-com-spot");
        assert_eq!(com.asset_class, AssetClass::Cryptocurrency);
        let kotak = descriptor_for_slug("kotak_neo").expect("kotak");
        assert_eq!(kotak.book_id, "kotak-nse-bse-cash");
        assert_eq!(kotak.asset_class, AssetClass::Equity);
        assert_eq!(kotak.instrument_class, InstrumentClass::Spot);
    }
}
