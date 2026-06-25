import Foundation

/// Hybrid armed reconcile — 45s timeout · ≥3 failed live-state polls (#183 / CONTEXT.md).
enum BarOptimisticArmedReconcilePolicy {
    static let maxAgeSeconds: TimeInterval = 45
    static let maxPollFailures = 3
    static let confirmWarningMessage =
        "Could not confirm declaration — check web Bar."

    static func shouldClearOptimistic(
        snapshot: BarOptimisticArmedSnapshot,
        pollFailures: Int,
        now: Date = Date()
    ) -> Bool {
        let age = now.timeIntervalSince1970 - snapshot.submittedAt
        if age > maxAgeSeconds { return true }
        if pollFailures >= maxPollFailures { return true }
        return false
    }
}
