import Foundation

/// When PLAN live-state poll chrome (spinner / red strip) may paint.
/// Separate from [`BarOptimisticArmedReconcilePolicy`] — that counter clears hybrid armed.
enum BarLiveStatePollChrome {
    /// Transient 502 / network: two consecutive misses (~4s at the 2s poll) before the strip.
    static let stickyFailureCount = 2

    /// Spinner only on a cold fetch — never over last-good PLAN content.
    static func showsLoading(hasLiveState: Bool, isInFlight: Bool) -> Bool {
        isInFlight && !hasLiveState
    }

    /// Device-login is user-actionable and shows immediately. Transient misses keep last-good
    /// until `stickyFailureCount`. First open with no last-good shows the strip on the first miss.
    static func shouldPublishError(
        hasLiveState: Bool,
        consecutiveFailures: Int,
        isDeviceLogin: Bool
    ) -> Bool {
        guard consecutiveFailures > 0 else { return false }
        if isDeviceLogin { return true }
        if !hasLiveState { return true }
        return consecutiveFailures >= stickyFailureCount
    }

    static func nextConsecutiveFailures(previous: Int, succeeded: Bool) -> Int {
        succeeded ? 0 : previous + 1
    }
}
