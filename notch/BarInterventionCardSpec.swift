import Foundation

// MARK: - #122 — intervention ladder + card chrome (unified notch reference §BarPlanStateView)

/// Priority for Notch intervention cards — **lower** `sortRank` = surfaces **first** (more urgent).
/// Aligned with `2026-05-16-notch-unified-build-reference.md`:
/// `token_expired → kill_switch → sync_stale → naked_window → post_loss_macro → composite_red → exit_gate_micro`,
/// plus `bar_*` wire kinds from live-state projection.
enum BarInterventionCardSpec {
    enum PrimaryAccessory: Equatable {
        case none
        /// Kill / enforcement — web Bar until Notch has full override UX (#122).
        case manageInWebBar
    }

    struct Chrome: Equatable {
        let borderColorHex: String
        let borderWidth: Double
        let backgroundHex: String
        let backgroundOpacity: Double
        /// Animated dot for naked-window emphasis when `isPrimary` (honor Reduce Motion outside).
        let showNakedPulseDot: Bool
    }

    static func sortRank(interventionType: String) -> Int {
        rank(for: canonical(interventionType))
    }

    static func sortedInterventions(_ items: [ActiveIntervention]) -> [ActiveIntervention] {
        items.sorted { a, b in
            let ra = sortRank(interventionType: a.interventionType)
            let rb = sortRank(interventionType: b.interventionType)
            if ra != rb { return ra < rb }
            return a.interventionType.lowercased() < b.interventionType.lowercased()
        }
    }

    static func primaryAccessory(interventionType: String) -> PrimaryAccessory {
        switch canonical(interventionType) {
        case "bar_protective_sl", "naked_window":
            // T5 K3 — intervention must not re-open Notch `place_sl`.
            return .none
        case "bar_kill_switch", "kill_switch":
            return .manageInWebBar
        default:
            return .none
        }
    }

    static func chrome(interventionType: String, isPrimary: Bool) -> Chrome {
        let base = Chrome(
            borderColorHex: "#F5A524",
            borderWidth: isPrimary ? 2 : 0,
            backgroundHex: "#F5A524",
            backgroundOpacity: 0.08,
            showNakedPulseDot: false,
        )
        guard isPrimary else { return base }

        switch canonical(interventionType) {
        case "token_expired", "bar_broker_session":
            return Chrome(
                borderColorHex: "#FF9500",
                borderWidth: 2,
                backgroundHex: "#FF9500",
                backgroundOpacity: 0.10,
                showNakedPulseDot: false,
            )
        case "kill_switch", "bar_kill_switch", "bar_stop_me":
            return Chrome(
                borderColorHex: "#FF3B30",
                borderWidth: 2,
                backgroundHex: "#FF3B30",
                backgroundOpacity: 0.10,
                showNakedPulseDot: false,
            )
        case "sync_stale", "bar_stream_health":
            return Chrome(
                borderColorHex: "#8E8E93",
                borderWidth: 2,
                backgroundHex: "#8E8E93",
                backgroundOpacity: 0.10,
                showNakedPulseDot: false,
            )
        case "naked_window", "bar_protective_sl":
            return Chrome(
                borderColorHex: "#F5A524",
                borderWidth: 2,
                backgroundHex: "#F5A524",
                backgroundOpacity: 0.12,
                showNakedPulseDot: true,
            )
        case "post_loss_macro":
            return Chrome(
                borderColorHex: "#F5A524",
                borderWidth: 2,
                backgroundHex: "#F5A524",
                backgroundOpacity: 0.10,
                showNakedPulseDot: false,
            )
        case "composite_red", "bar_composite_red":
            return Chrome(
                borderColorHex: "#FF3B30",
                borderWidth: 2,
                backgroundHex: "#FF3B30",
                backgroundOpacity: 0.10,
                showNakedPulseDot: false,
            )
        case "exit_gate_micro":
            return Chrome(
                borderColorHex: "#AF52DE",
                borderWidth: 2,
                backgroundHex: "#AF52DE",
                backgroundOpacity: 0.10,
                showNakedPulseDot: false,
            )
        default:
            return base
        }
    }

    private static func canonical(_ raw: String) -> String {
        raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }

    private static func legacyAlias(_ k: String) -> String? {
        switch k {
        case "soft_block", "hard_block":
            return "bar_composite_amber"
        default:
            return nil
        }
    }

    /// Ladder ranks — **ascending** urgency (lower index = show first).
    private static func rank(for key: String) -> Int {
        let k = legacyAlias(key) ?? key
        switch k {
        case "token_expired", "bar_broker_session":
            return 0
        case "kill_switch", "bar_kill_switch":
            return 10
        case "bar_stop_me":
            return 12
        case "sync_stale", "bar_stream_health":
            return 20
        case "naked_window", "bar_protective_sl":
            return 30
        case "post_loss_macro":
            return 40
        case "composite_red", "bar_composite_red":
            return 50
        case "exit_gate_micro":
            return 60
        case "bar_composite_amber":
            return 70
        default:
            return 1000
        }
    }
}
