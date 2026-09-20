import Foundation

/// Hybrid armed is no longer the source of truth (N1 LiveBook). Kept so leftover
/// UserDefaults keys can be cleared. Age / poll-failure must not un-arm.
enum BarOptimisticArmedReconcilePolicy {
    static let maxAgeSeconds: TimeInterval = 45
    static let maxPollFailures = 3
    static let confirmWarningMessage =
        "Could not confirm declaration on Station."

    static func shouldClearOptimistic(
        snapshot: BarOptimisticArmedSnapshot,
        pollFailures: Int,
        now: Date = Date()
    ) -> Bool {
        _ = snapshot
        _ = pollFailures
        _ = now
        return false
    }
}
