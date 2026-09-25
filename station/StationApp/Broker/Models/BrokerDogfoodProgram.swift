import Foundation

/// ADR 0019 — **Connect beta**: book-row **Enabled** for sync; badge until step-6 desk-live.
/// `catalog_v1()` Enabled remains Kotak + Binance only (Today hero / registry live slugs).
public enum BrokerDogfoodProgram {
    /// Phase 4 crypto spot — offline / deferred-live drills (`plans/dogfood-*-2026-09-24.md`).
    public enum P4OfflineDrill: String, CaseIterable, Sendable {
        case connectAndPoll = "connect_and_poll"
        case hostFenceHonesty = "host_fence_honesty"
        case quoteFilterHonesty = "quote_filter_honesty"
        case killSwitch = "kill_switch"
        case wasmCredentialShapeGate = "wasm_credential_shape_gate"
    }

    /// Wave 2 India cash tracers (integrator-complete, dogfood open).
    public static let tierIIndiaCashSlugs: Set<String> = [
        "zerodha_kite",
        "upstox",
        "fyers",
        "groww",
        "dhan",
    ]

    public static let p4CryptoSpotSlugs: Set<String> = [
        "bybit",
        "okx_com",
        "kraken",
        "coinbase_advanced",
    ]

    /// Wave 2 + P4: Enabled in `BrokerCatalog` but not desk-live Tier II — show **Connect beta**.
    public static let connectBetaSlugs: Set<String> = tierIIndiaCashSlugs
        .union(p4CryptoSpotSlugs)

    /// Legacy: Planned rows that may still Connect (empty once all tracers are Enabled).
    public static let connectWhilePlannedSlugs: Set<String> = []

    public static func allowsConnectWhilePlanned(slug: String) -> Bool {
        connectWhilePlannedSlugs.contains(slug)
    }

    public static func showsConnectBetaBadge(slug: String) -> Bool {
        connectBetaSlugs.contains(slug)
    }

    public static func offlineDrills(for slug: String) -> [P4OfflineDrill] {
        guard p4CryptoSpotSlugs.contains(slug) else { return [] }
        return P4OfflineDrill.allCases
    }
}
