import Foundation

/// ADR 0019 — Tier I connect while catalog stays **Planned**; Tier II flips **Enabled** after signed dogfood (step 6).
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

    public static let p4PlannedSlugs: Set<String> = [
        "bybit",
        "okx_com",
        "kraken",
        "coinbase_advanced",
    ]

    /// Slugs that may Connect + Start sync while `Planned` (Station UI + offline drills).
    public static let connectWhilePlannedSlugs: Set<String> = tierIIndiaCashSlugs
        .union(p4PlannedSlugs)

    public static func allowsConnectWhilePlanned(slug: String) -> Bool {
        connectWhilePlannedSlugs.contains(slug)
    }

    public static func offlineDrills(for slug: String) -> [P4OfflineDrill] {
        guard p4PlannedSlugs.contains(slug) else { return [] }
        return P4OfflineDrill.allCases
    }
}
