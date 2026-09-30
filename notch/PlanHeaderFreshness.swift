import Foundation

/// Plan top bar: broker connection age vs quote lane — never one clock for both.
enum PlanHeaderFreshness {
    static func connectionChipLabel(
        syncClass: String,
        lastSyncedAtMs: Int64?,
        requiresDeviceLogin: Bool,
        now: Date
    ) -> String {
        if requiresDeviceLogin { return "Sign in" }
        let sync = syncClass.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if sync == "not_connected" || sync.isEmpty {
            return "Offline"
        }
        return BrokerSyncRelativeFreshness.format(epochMs: lastSyncedAtMs, now: now) ?? "Waiting"
    }

    static func connectionChipIsWarn(syncClass: String, requiresDeviceLogin: Bool) -> Bool {
        if requiresDeviceLogin { return true }
        switch syncClass.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "stale", "degraded", "disconnected", "not_connected", "failed":
            return true
        default:
            return false
        }
    }

    /// Shown beside connection age when quote lane is stale — does not imply broker disconnect.
    static func quoteLaneChipLabel(
        quoteCapability: String,
        laneObservedAt: Date?,
        now: Date
    ) -> String? {
        let q = quoteCapability.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard q == "stale" else { return nil }
        if let age = BrokerSyncRelativeFreshness.formatShortAge(since: laneObservedAt, now: now) {
            return "Quote stale · \(age)"
        }
        return "Quote stale"
    }
}
