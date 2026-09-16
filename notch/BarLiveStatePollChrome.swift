import Foundation

/// PLAN live-state poll chrome. Transient misses never insert a body strip.
/// Separate from [`BarOptimisticArmedReconcilePolicy`] — that counter clears hybrid armed.
enum BarLiveStatePollChrome {
    /// Chip goes amber after this many seconds without a successful poll.
    static let staleAfterSeconds: TimeInterval = 10

    /// Never a spinner strip — first open shows `Waiting` on the freshness chip.
    static func showsLoading(hasLiveState: Bool, isInFlight: Bool) -> Bool {
        _ = hasLiveState
        _ = isInFlight
        return false
    }

    /// Device-login is sticky and user-actionable. Transient 502 / network never publish `barStateError`.
    static func shouldPublishError(
        hasLiveState: Bool,
        consecutiveFailures: Int,
        isDeviceLogin: Bool
    ) -> Bool {
        _ = hasLiveState
        guard consecutiveFailures > 0 else { return false }
        return isDeviceLogin
    }

    static func nextConsecutiveFailures(previous: Int, succeeded: Bool) -> Int {
        succeeded ? 0 : previous + 1
    }

    /// `Waiting` · `9s ago` · `2m ago`
    static func freshnessLabel(lastFetched: Date?, now: Date) -> String {
        guard let lastFetched else { return "Waiting" }
        let secs = max(0, Int(now.timeIntervalSince(lastFetched).rounded(.down)))
        if secs < 60 { return "\(secs)s ago" }
        return "\(secs / 60)m ago"
    }

    static func isStale(lastFetched: Date?, now: Date) -> Bool {
        guard let lastFetched else { return false }
        return now.timeIntervalSince(lastFetched) >= staleAfterSeconds
    }
}
