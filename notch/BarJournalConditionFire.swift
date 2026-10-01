import Foundation

/// Wave 7 — append-only condition fire log on the N2 journal object (no orders).
enum BarJournalConditionFire {
    static func workingSnapshotJSON(
        invalidated: Bool,
        last: Double?,
        lastStatus: String,
        invalidationKind: String?,
        invalidationPrice: Double?
    ) -> [String: Any] {
        [
            "invalidation_state": invalidated ? "breached" : "intact",
            "last_status": lastStatus,
            "invalidation_kind": invalidationKind ?? "",
            "invalidation_price": invalidationPrice as Any,
            "last": last as Any,
        ]
    }
}
