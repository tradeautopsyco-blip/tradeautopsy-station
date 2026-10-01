import Foundation

/// Working screen detail state for the **selected** row (`docs/design/harness-open-plan-working.md` §3).
enum BarWorkingHarnessDetail: Equatable, Sendable {
    case flat
    case declaredNotFilled
    case inTrade
}

enum BarWorkingHarnessDetailResolver {
    static func pendingHasFill(_ pending: BarPendingDeclaration?) -> Bool {
        guard let pending else { return false }
        if let fq = pending.filledQty, fq > 0 { return true }
        if let avg = pending.avgFill, avg > 0 { return true }
        return false
    }

    static func resolve(
        pendingRows: [BarPendingDeclaration],
        selectedPending: BarPendingDeclaration?,
        hasOpenPositions: Bool,
        hasUndeclaredInventory: Bool
    ) -> BarWorkingHarnessDetail {
        if hasOpenPositions || hasUndeclaredInventory {
            return .inTrade
        }
        if pendingHasFill(selectedPending) {
            return .inTrade
        }
        if !pendingRows.isEmpty {
            return .declaredNotFilled
        }
        return .flat
    }

    /// Book-level `plan_state` must not paint GREEN when empty and flat.
    static func shouldShowPlanStateBanner(
        detail: BarWorkingHarnessDetail,
        planStateRaw: String?
    ) -> Bool {
        switch detail {
        case .flat:
            return false
        case .declaredNotFilled:
            let trimmed = planStateRaw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            return !trimmed.isEmpty
        case .inTrade:
            return true
        }
    }

    static func normalizedPlanUpper(planStateRaw: String?, detail: BarWorkingHarnessDetail) -> String? {
        let trimmed = planStateRaw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if trimmed.isEmpty {
            switch detail {
            case .inTrade:
                return nil
            case .declaredNotFilled, .flat:
                return nil
            }
        }
        return trimmed.uppercased()
    }
}
