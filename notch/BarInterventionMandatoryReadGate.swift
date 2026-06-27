import Foundation

// MARK: - #3 kill / composite intervention — minimum read window (pure, testable)

enum BarInterventionMandatoryReadGate {
    static let defaultSeconds: Int = 15

    /// Seconds remaining until controls may unlock (0 = unlocked). Clamped at 0.
    static func secondsRemaining(
        anchorStartedAt: Date?,
        now: Date,
        minimumSeconds: Int = defaultSeconds,
    ) -> Int {
        guard let start = anchorStartedAt else { return minimumSeconds }
        let elapsed = Int(now.timeIntervalSince(start).rounded(.down))
        return max(0, minimumSeconds - elapsed)
    }

    static func isReadingComplete(
        anchorStartedAt: Date?,
        now: Date,
        minimumSeconds: Int = defaultSeconds,
    ) -> Bool {
        secondsRemaining(anchorStartedAt: anchorStartedAt, now: now, minimumSeconds: minimumSeconds) == 0
    }
}

extension BarInterventionCardSpec {
    /// Types that require a mandatory read window before primary actions (override / web Bar).
    static func requiresMandatoryRead(interventionType: String) -> Bool {
        let k = interventionType.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch k {
        case "kill_switch", "bar_kill_switch":
            return true
        case "composite_red", "bar_composite_red":
            return true
        default:
            return false
        }
    }
}
