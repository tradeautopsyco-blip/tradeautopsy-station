//! Per-venue policy. Everything in this file is either quoted from a docs lock in
//! `docs/reference/` or explicitly marked as a chosen default. Nothing is blended
//! across venues.

use super::meter::Budget;

/// A venue. Ban scope is the slot: one IP, one freeze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotPolicy {
    pub slot_id: &'static str,
    /// Statuses that freeze one meter.
    pub rate_limit_statuses: &'static [u16],
    /// Statuses that ban the whole slot. Empty when the lock documents none.
    pub ban_statuses: &'static [u16],
    /// Used when a 429 carries no `Retry-After`.
    pub default_backoff_ms: i64,
    /// First local ban length when a 418 carries no parseable expiry.
    ///
    /// NOT from the lock — `spot/REST.md:48` says "HTTP 418 = IP banned" and gives
    /// no duration. Chosen to be recoverable rather than correct, and escalated on
    /// repeat (see `ban_escalation_ms`) so a wrong guess self-corrects instead of
    /// re-banning the IP.
    pub default_ban_ms: i64,
    /// Local ban length after N consecutive bans, capped at the last entry.
    pub ban_escalation_ms: &'static [i64],
    /// Meter that connect-time auth traffic is charged to. Auth endpoints sit
    /// outside the R0 *data* fence by design, but they reach the same IP, so they
    /// must still be metered and must still respect a ban.
    pub auth_meter_id: &'static str,
}

pub const BINANCE_COM: SlotPolicy = SlotPolicy {
    slot_id: "binance_com",
    // spot/REST.md:48 — "HTTP 429 = rate limit hit."
    rate_limit_statuses: &[429],
    // spot/REST.md:48 — "HTTP 418 = IP banned."
    ban_statuses: &[418],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000, 900_000, 2_700_000, 10_800_000],
    auth_meter_id: "binance_com:api_weight",
};

/// Zerodha Kite — B6 row 2: 10 rps on most endpoints, 1 rps on quote; numeric window
/// not modeled as a hard ledger here (same pattern as Kotak NotSpecified).
pub const ZERODHA_KITE: SlotPolicy = SlotPolicy {
    slot_id: "zerodha_kite",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "zerodha_kite:requests",
};

/// Upstox — numeric window NOT SPECIFIED on the B6 lock; pace/freeze only (429).
pub const UPSTOX: SlotPolicy = SlotPolicy {
    slot_id: "upstox",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "upstox:requests",
};

/// Fyers — same NotSpecified pattern as other India OAuth brokers.
pub const FYERS: SlotPolicy = SlotPolicy {
    slot_id: "fyers",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "fyers:requests",
};

/// Dhan — REST v2 reads + consent auth host (B6 row 4); one meter per slot.
pub const DHAN: SlotPolicy = SlotPolicy {
    slot_id: "dhan",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "dhan:requests",
};

/// Groww — REST + assets CSV; one meter per slot (unknown bucket sharing).
pub const GROWW: SlotPolicy = SlotPolicy {
    slot_id: "groww",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "groww:requests",
};

/// Bybit v5 — B6 row 4: 429 / retCode 10006; no documented 418 on this lock.
pub const BYBIT: SlotPolicy = SlotPolicy {
    slot_id: "bybit",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "bybit:requests",
};

/// OKX v5 global — B6 row 4: code 50011 rate limit; no 418 on this lock.
pub const OKX_COM: SlotPolicy = SlotPolicy {
    slot_id: "okx_com",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "okx_com:requests",
};

/// Kraken spot REST — B6 row 4: EAPI:Rate limit exceeded; no 418 on this lock.
pub const KRAKEN: SlotPolicy = SlotPolicy {
    slot_id: "kraken",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "kraken:requests",
};

/// Coinbase Advanced Trade — B6 row 4: 429 documented; no 418 on this lock.
pub const COINBASE_ADVANCED: SlotPolicy = SlotPolicy {
    slot_id: "coinbase_advanced",
    rate_limit_statuses: &[429],
    ban_statuses: &[],
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "coinbase_advanced:requests",
};

pub const KOTAK_NEO: SlotPolicy = SlotPolicy {
    slot_id: "kotak_neo",
    // india/kotak-neo/REST.md:226 — 429 "Too many requests to the API".
    rate_limit_statuses: &[429],
    // No IP-ban status documented for Kotak. Do not invent one.
    ban_statuses: &[],
    // Matches the existing `BrokerSyncConfig::rate_limit_default_backoff_ms`.
    default_backoff_ms: 60_000,
    default_ban_ms: 300_000,
    ban_escalation_ms: &[300_000],
    auth_meter_id: "kotak_neo:requests",
};

pub const SLOTS: &[SlotPolicy] = &[
    BINANCE_COM,
    KOTAK_NEO,
    ZERODHA_KITE,
    UPSTOX,
    FYERS,
    GROWW,
    DHAN,
    BYBIT,
    OKX_COM,
    KRAKEN,
    COINBASE_ADVANCED,
];

/// One metered budget inside a slot.
#[derive(Debug, Clone, Copy)]
pub struct MeterPolicy {
    pub meter_id: &'static str,
    pub slot_id: &'static str,
    pub budget: Budget,
    pub max_concurrency: u32,
}

/// `api.binance.com` REQUEST_WEIGHT — spot/REST.md:40-46, 6000 per 1 minute.
/// Headroom 85% and a 70% market-data floor are chosen, not published.
pub const BINANCE_COM_API_WEIGHT: MeterPolicy = MeterPolicy {
    meter_id: "binance_com:api_weight",
    slot_id: "binance_com",
    budget: Budget::Specified {
        limit: 6000,
        window_ms: 60_000,
        headroom_pct: 85,
        market_data_pct: 70,
    },
    max_concurrency: 8,
};

/// `eapi.binance.com` — `binance-global/options/REST.md` has **no rate-limit
/// section**. No ledger may be invented; this meter paces and freezes only. It
/// shares the `binance_com` IP ban with the spot meter but not its counter.
pub const BINANCE_COM_EAPI: MeterPolicy = MeterPolicy {
    meter_id: "binance_com:eapi",
    slot_id: "binance_com",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

/// `fapi.binance.com` — numeric REQUEST_WEIGHT window **NOT SPECIFIED** on the
/// USDM lock this slice. Pace/freeze only. Shares `binance_com` IP ban. Not the spot meter.
pub const BINANCE_COM_FAPI: MeterPolicy = MeterPolicy {
    meter_id: "binance_com:fapi",
    slot_id: "binance_com",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

/// `dapi.binance.com` — Coin-M. Same NotSpecified budget rule as eapi/fapi.
pub const BINANCE_COM_DAPI: MeterPolicy = MeterPolicy {
    meter_id: "binance_com:dapi",
    slot_id: "binance_com",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

/// Kotak Neo — `india/kotak-neo/REST.md:22,260` records HTTP 429 as real and the
/// numeric window as NOT SPECIFIED. REST.md:290 further records that whether
/// quotes, file-paths, and the trade book share one bucket is unknown. Unknown
/// sharing is resolved the safe way: **one meter for the whole slot**, so cash and
/// NFO pace against each other rather than each assuming it has the venue to itself.
pub const KOTAK_NEO_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "kotak_neo:requests",
    slot_id: "kotak_neo",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const ZERODHA_KITE_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "zerodha_kite:requests",
    slot_id: "zerodha_kite",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const UPSTOX_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "upstox:requests",
    slot_id: "upstox",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const FYERS_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "fyers:requests",
    slot_id: "fyers",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const GROWW_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "groww:requests",
    slot_id: "groww",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const DHAN_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "dhan:requests",
    slot_id: "dhan",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const BYBIT_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "bybit:requests",
    slot_id: "bybit",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const OKX_COM_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "okx_com:requests",
    slot_id: "okx_com",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const KRAKEN_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "kraken:requests",
    slot_id: "kraken",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const COINBASE_ADVANCED_REQUESTS: MeterPolicy = MeterPolicy {
    meter_id: "coinbase_advanced:requests",
    slot_id: "coinbase_advanced",
    budget: Budget::NotSpecified,
    max_concurrency: 4,
};

pub const METERS: &[MeterPolicy] = &[
    BINANCE_COM_API_WEIGHT,
    BINANCE_COM_EAPI,
    BINANCE_COM_FAPI,
    BINANCE_COM_DAPI,
    KOTAK_NEO_REQUESTS,
    ZERODHA_KITE_REQUESTS,
    UPSTOX_REQUESTS,
    FYERS_REQUESTS,
    GROWW_REQUESTS,
    DHAN_REQUESTS,
    BYBIT_REQUESTS,
    OKX_COM_REQUESTS,
    KRAKEN_REQUESTS,
    COINBASE_ADVANCED_REQUESTS,
];

/// Book id → (slot, meter). Book ids are the same strings the R0 fence keys on
/// (`data::host_policy::authorize_book_fence`), so a book that the fence does not
/// know cannot reach a meter either.
pub fn route_book(book_id: &str) -> Option<&'static MeterPolicy> {
    match book_id.trim() {
        "binance-com-spot" => Some(&BINANCE_COM_API_WEIGHT),
        "binance-com-options" => Some(&BINANCE_COM_EAPI),
        "binance-com-usdm" => Some(&BINANCE_COM_FAPI),
        "binance-com-coinm" => Some(&BINANCE_COM_DAPI),
        "kotak-nse-bse-cash" | "kotak-nse-nfo" | "kotak-nse-cds" | "kotak-mcx-future" => {
            Some(&KOTAK_NEO_REQUESTS)
        }
        crate::ubi::ZERODHA_KITE_BOOK_ID | crate::ubi::ZERODHA_KITE_NFO_BOOK_ID => {
            Some(&ZERODHA_KITE_REQUESTS)
        }
        crate::ubi::UPSTOX_BOOK_ID | crate::ubi::UPSTOX_NFO_BOOK_ID => Some(&UPSTOX_REQUESTS),
        crate::ubi::FYERS_BOOK_ID | crate::ubi::FYERS_NFO_BOOK_ID => Some(&FYERS_REQUESTS),
        crate::ubi::GROWW_BOOK_ID => Some(&GROWW_REQUESTS),
        crate::ubi::DHAN_BOOK_ID => Some(&DHAN_REQUESTS),
        crate::ubi::bybit_session::BYBIT_BOOK_ID => Some(&BYBIT_REQUESTS),
        crate::ubi::OKX_COM_SPOT_BOOK_ID => Some(&OKX_COM_REQUESTS),
        crate::ubi::KRAKEN_BOOK_ID => Some(&KRAKEN_REQUESTS),
        crate::ubi::coinbase_session::COINBASE_BOOK_ID => Some(&COINBASE_ADVANCED_REQUESTS),
        _ => None,
    }
}

pub fn meter_policy(meter_id: &str) -> Option<&'static MeterPolicy> {
    METERS.iter().find(|m| m.meter_id == meter_id)
}

/// Which venue owns a host. Reuses `dns_block::hosts_for_broker`, the same map
/// Kill DNS sinkholes, so a host can never be metered under one venue and
/// sinkholed under another.
pub fn slot_for_host(host: &str) -> Option<&'static SlotPolicy> {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    SLOTS.iter().find(|slot| {
        crate::dns_block::hosts_for_broker(slot.slot_id)
            .iter()
            .any(|h| *h == host)
    })
}

/// First `limit=` pair parsed as `u32`. Missing or unparsable → `None`.
fn query_limit(query: &str) -> Option<u32> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "limit").then_some(v)?.parse().ok()
    })
}

/// `GET /api/v3/depth` REQUEST_WEIGHT from rest-api.md Order book table
/// (REST.md, fetched 2026-08-30). Missing / unparsable limit → official default
/// 100 → 5. Out of range → 250 (fail closed).
fn depth_weight(query: &str) -> u32 {
    match query_limit(query) {
        None => 5,
        Some(n) if (1..=100).contains(&n) => 5,
        Some(n) if (101..=500).contains(&n) => 25,
        Some(n) if (501..=1000).contains(&n) => 50,
        Some(n) if (1001..=5000).contains(&n) => 250,
        Some(_) => 250,
    }
}

/// Endpoint weight forecast for `api.binance.com`, from spot/REST.md.
///
/// Depth uses the published limit table (`depth_weight`). Other endpoints that
/// still publish a range without a table forecast the maximum. Over-forecasting
/// can only cost throughput; under-forecasting costs the IP. The
/// `X-MBX-USED-WEIGHT-1M` header corrects the figure on the next response.
pub fn binance_spot_weight(path: &str, query: &str) -> u32 {
    let path = path.split('?').next().unwrap_or(path).trim_end_matches('/');
    let has = |k: &str| {
        query
            .split('&')
            .any(|kv| kv.split('=').next().is_some_and(|n| n == k))
    };
    match path {
        // REST.md:214-215
        "/api/v3/ping" | "/api/v3/time" => 1,
        // REST.md:149 / klines section
        "/api/v3/klines" | "/api/v3/uiKlines" => 2,
        // REST.md:106
        "/api/v3/order" => 4,
        // REST.md:68
        "/api/v3/exchangeInfo" => {
            if has("symbol") || has("symbols") {
                4
            } else {
                20
            }
        }
        // REST.md:76 / :83 / :99
        "/api/v3/account" | "/api/v3/myTrades" | "/api/v3/allOrders" => 20,
        // REST.md:92
        "/api/v3/openOrders" => {
            if has("symbol") {
                6
            } else {
                40
            }
        }
        // REST.md:113 — "80 (no symbol) / 2 (1 symbol) / varies (list)".
        // A list is a range with no published table: forecast the maximum.
        "/api/v3/ticker/24hr" => {
            if has("symbol") {
                2
            } else {
                80
            }
        }
        // REST.md:119
        "/api/v3/ticker/price" => {
            if has("symbol") {
                2
            } else {
                4
            }
        }
        // REST.md Order book table (fetched 2026-08-30).
        "/api/v3/depth" => depth_weight(query),
        // REST.md:132
        "/api/v3/trades" => 25,
        // Unknown path: the fence already allowlists paths, so this is a path we
        // are permitted to call but have not yet priced. Forecast at the heaviest
        // routinely-used private read rather than free.
        _ => 20,
    }
}

/// Weight forecast for a meter. Meters with no published budget cost nothing,
/// because there is no ledger for a cost to land in.
pub fn forecast_weight(meter: &MeterPolicy, path: &str, query: &str) -> u32 {
    match meter.budget {
        Budget::NotSpecified => 0,
        Budget::Specified { .. } if meter.meter_id == BINANCE_COM_API_WEIGHT.meter_id => {
            binance_spot_weight(path, query)
        }
        Budget::Specified { .. } => 1,
    }
}

/// The venue's own used-weight header for a meter, if it publishes one.
/// `ubi::http::RESPONSE_HEADER_ALLOWLIST` already permits these through.
pub fn used_weight_header(meter: &MeterPolicy) -> Option<&'static str> {
    if meter.meter_id == BINANCE_COM_API_WEIGHT.meter_id {
        Some("x-mbx-used-weight-1m")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn books_route_to_the_right_slot() {
        assert_eq!(
            route_book("binance-com-spot").unwrap().slot_id,
            "binance_com"
        );
        assert_eq!(
            route_book("binance-com-options").unwrap().slot_id,
            "binance_com"
        );
        assert_eq!(
            route_book("kotak-nse-bse-cash").unwrap().slot_id,
            "kotak_neo"
        );
        assert_eq!(route_book("kotak-nse-nfo").unwrap().slot_id, "kotak_neo");
        assert_eq!(
            route_book(crate::ubi::ZERODHA_KITE_NFO_BOOK_ID).unwrap().slot_id,
            "zerodha_kite"
        );
        assert_eq!(
            route_book(crate::ubi::ZERODHA_KITE_BOOK_ID).unwrap().slot_id,
            "zerodha_kite"
        );
        assert_eq!(route_book(crate::ubi::UPSTOX_BOOK_ID).unwrap().slot_id, "upstox");
        assert_eq!(route_book(crate::ubi::FYERS_BOOK_ID).unwrap().slot_id, "fyers");
        assert_eq!(route_book(crate::ubi::GROWW_BOOK_ID).unwrap().slot_id, "groww");
        assert_eq!(
            route_book("binance-com-usdm").unwrap().meter_id,
            "binance_com:fapi"
        );
        assert_eq!(
            route_book("binance-com-coinm").unwrap().meter_id,
            "binance_com:dapi"
        );
        assert_ne!(
            route_book("binance-com-usdm").unwrap().meter_id,
            route_book("binance-com-spot").unwrap().meter_id
        );
        assert!(route_book("binance-com-stocks").is_none());
        assert!(route_book("").is_none());
    }

    #[test]
    fn binance_spot_and_options_share_a_slot_but_not_a_meter() {
        let spot = route_book("binance-com-spot").unwrap();
        let opts = route_book("binance-com-options").unwrap();
        assert_eq!(spot.slot_id, opts.slot_id);
        assert_ne!(spot.meter_id, opts.meter_id);
    }

    #[test]
    fn kotak_books_share_one_meter_because_sharing_is_unknown() {
        let cash = route_book("kotak-nse-bse-cash").unwrap();
        let nfo = route_book("kotak-nse-nfo").unwrap();
        assert_eq!(cash.meter_id, nfo.meter_id);
    }

    #[test]
    fn unpublished_budgets_stay_unpublished() {
        assert_eq!(BINANCE_COM_EAPI.budget, Budget::NotSpecified);
        assert_eq!(BINANCE_COM_FAPI.budget, Budget::NotSpecified);
        assert_eq!(BINANCE_COM_DAPI.budget, Budget::NotSpecified);
        assert_eq!(KOTAK_NEO_REQUESTS.budget, Budget::NotSpecified);
        assert_eq!(ZERODHA_KITE_REQUESTS.budget, Budget::NotSpecified);
        assert_eq!(UPSTOX_REQUESTS.budget, Budget::NotSpecified);
        assert_eq!(FYERS_REQUESTS.budget, Budget::NotSpecified);
        assert_eq!(GROWW_REQUESTS.budget, Budget::NotSpecified);
        assert_eq!(forecast_weight(&BINANCE_COM_EAPI, "/eapi/v1/ticker", ""), 0);
        assert_eq!(used_weight_header(&BINANCE_COM_EAPI), None);
    }

    #[test]
    fn every_slot_host_is_also_a_kill_dns_host() {
        // If these drift, a venue could be metered here and unreachable there.
        for slot in SLOTS {
            assert!(
                !crate::dns_block::hosts_for_broker(slot.slot_id).is_empty(),
                "slot {} has no Kill DNS host set",
                slot.slot_id
            );
        }
    }

    #[test]
    fn auth_hosts_route_to_their_venue() {
        // Kotak login host — outside the R0 data fence, inside the venue.
        assert_eq!(
            slot_for_host("mis.kotaksecurities.com").unwrap().slot_id,
            "kotak_neo"
        );
        assert_eq!(
            slot_for_host("api.binance.com").unwrap().slot_id,
            "binance_com"
        );
        assert_eq!(
            slot_for_host("eapi.binance.com").unwrap().slot_id,
            "binance_com"
        );
        assert_eq!(
            slot_for_host("stream.binance.com").unwrap().slot_id,
            "binance_com"
        );
        assert!(slot_for_host("evil.example.com").is_none());
        // binance.us has no slot and must not be adopted by binance_com.
        assert!(slot_for_host("api.binance.us").is_none());
    }

    #[test]
    fn every_slot_names_a_real_auth_meter() {
        for slot in SLOTS {
            assert!(
                meter_policy(slot.auth_meter_id).is_some(),
                "slot {} names auth meter {} which does not exist",
                slot.slot_id,
                slot.auth_meter_id
            );
        }
    }

    #[test]
    fn kotak_has_no_documented_ip_ban() {
        assert!(KOTAK_NEO.ban_statuses.is_empty());
        assert_eq!(BINANCE_COM.ban_statuses, &[418]);
    }

    /// Every shipping book in `catalog_books()` must egress-route or Wasm sync gets `wrong_book`.
    #[test]
    fn every_catalog_book_routes_to_egress() {
        let mut seen = std::collections::BTreeSet::new();
        for descriptor in crate::ubi::catalog_books() {
            if !seen.insert(descriptor.book_id.clone()) {
                continue;
            }
            assert!(
                route_book(&descriptor.book_id).is_some(),
                "book {} (slug {}) missing route_book row",
                descriptor.book_id,
                descriptor.slug
            );
        }
    }

    #[test]
    fn india_broker_api_hosts_map_to_slots() {
        assert_eq!(slot_for_host("api.kite.trade").unwrap().slot_id, "zerodha_kite");
        assert_eq!(slot_for_host("api.upstox.com").unwrap().slot_id, "upstox");
        assert_eq!(
            slot_for_host(crate::ubi::FYERS_API_HOST).unwrap().slot_id,
            "fyers"
        );
        assert_eq!(slot_for_host("api.groww.in").unwrap().slot_id, "groww");
        assert_eq!(slot_for_host("api.dhan.co").unwrap().slot_id, "dhan");
        assert_eq!(slot_for_host("auth.dhan.co").unwrap().slot_id, "dhan");
    }

    #[test]
    fn depth_weight_follows_the_official_limit_table() {
        // rest-api.md Order book table (fetched 2026-08-30):
        // 1–100 → 5, 101–500 → 25, 501–1000 → 50, 1001–5000 → 250.
        // Default limit is 100. Missing / unparsable → that default, not max 250.
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=5"),
            5
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=100"),
            5
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=101"),
            25
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=500"),
            25
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=501"),
            50
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=1000"),
            50
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=1001"),
            250
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=5000"),
            250
        );
        assert_eq!(binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT"), 5);
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=nope"),
            5
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=0"),
            250
        );
        assert_eq!(
            binance_spot_weight("/api/v3/depth", "symbol=BTCUSDT&limit=5001"),
            250
        );
    }

    #[test]
    fn symbol_scoped_calls_are_cheaper_per_the_lock() {
        assert_eq!(binance_spot_weight("/api/v3/exchangeInfo", ""), 20);
        assert_eq!(
            binance_spot_weight("/api/v3/exchangeInfo", "symbol=BTCUSDT"),
            4
        );
        assert_eq!(binance_spot_weight("/api/v3/openOrders", ""), 40);
        assert_eq!(
            binance_spot_weight("/api/v3/openOrders", "symbol=BTCUSDT"),
            6
        );
        // `ticker/price` is the desk's prime call. `symbol=` keeps it at 2;
        // the no-symbol form is 4 and must never be built (see
        // `data::binance_spot_ticker`). `24hr` is the expensive neighbour.
        assert_eq!(
            binance_spot_weight("/api/v3/ticker/price", "symbol=ETHUSDT"),
            2
        );
        assert_eq!(binance_spot_weight("/api/v3/ticker/price", ""), 4);
        assert_eq!(binance_spot_weight("/api/v3/ticker/24hr", ""), 80);
        assert_eq!(
            binance_spot_weight("/api/v3/ticker/24hr", "symbol=BTCUSDT"),
            2
        );
    }

    #[test]
    fn klines_are_cheap_and_a_query_string_never_confuses_the_path() {
        assert_eq!(binance_spot_weight("/api/v3/klines", "symbol=BTCUSDT"), 2);
        assert_eq!(binance_spot_weight("/api/v3/klines?symbol=BTCUSDT", ""), 2);
    }

    #[test]
    fn an_unpriced_path_is_never_free() {
        assert!(binance_spot_weight("/api/v3/somethingNew", "") > 0);
    }
}
