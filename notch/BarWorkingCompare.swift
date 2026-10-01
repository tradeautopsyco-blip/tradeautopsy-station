import Foundation

/// Working Actual vs Declared — last vs invalidation/target. Unbound last stays dark, not breached.
enum BarWorkingLevelState: String, Equatable {
    case waiting
    case intact
    case breached
    case touched
    case dark
}

enum BarWorkingCompare {
    static func lastBound(status: String) -> Bool {
        SessionChartQuoteLast.isBound(status: status)
    }

    static func levelState(fromWire raw: String?) -> BarWorkingLevelState? {
        switch raw?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? "" {
        case "waiting": return .waiting
        case "intact": return .intact
        case "breached": return .breached
        case "touched": return .touched
        case "dark": return .dark
        case "not_captured": return nil
        default: return nil
        }
    }

    /// During session: obtain last. After session: frozen snapshot only — never live last.
    static func invalidationActualText(
        afterSession: Bool,
        atClose: BarPlanConditionAtClose?,
        sideBuy: Bool,
        last: Double?,
        lastStatus: String,
        kind: String?,
        price: Double?
    ) -> (text: String, state: BarWorkingLevelState, showHonestyChip: Bool) {
        if afterSession {
            guard let atClose, atClose.wasCaptured else {
                return ("not captured", .dark, false)
            }
            let status = atClose.lastStatus ?? "unknown"
            let lastLine = labeledLast(last: atClose.last, status: status)
            let text = "After session · \(lastLine)"
            if let wired = atClose.invalidationState, let state = levelState(fromWire: wired) {
                return (text, state, false)
            }
            let state = vsInvalidation(
                sideBuy: sideBuy,
                last: atClose.last,
                status: status,
                kind: kind,
                price: price
            )
            return (text, state, false)
        }
        let state = vsInvalidation(
            sideBuy: sideBuy,
            last: last,
            status: lastStatus,
            kind: kind,
            price: price
        )
        let lastText = labeledLast(last: last, status: lastStatus)
        let text = state == .waiting ? "waiting" : lastText
        let chip = state == .dark
        return (text, state, chip)
    }

    static func targetActualText(
        afterSession: Bool,
        atClose: BarPlanConditionAtClose?,
        sideBuy: Bool,
        last: Double?,
        lastStatus: String,
        target: Double?
    ) -> (text: String, state: BarWorkingLevelState) {
        if afterSession {
            guard let atClose, atClose.wasCaptured else {
                return ("not captured", .dark)
            }
            let status = atClose.lastStatus ?? "unknown"
            let lastLine = labeledLast(last: atClose.last, status: status)
            let text = "After session · \(lastLine)"
            if let wired = atClose.targetState, let state = levelState(fromWire: wired) {
                return (text, state)
            }
            let state = vsTarget(
                sideBuy: sideBuy,
                last: atClose.last,
                status: status,
                target: target
            )
            return (text, state)
        }
        let state = vsTarget(
            sideBuy: sideBuy,
            last: last,
            status: lastStatus,
            target: target
        )
        return (labeledLast(last: last, status: lastStatus), state)
    }

    static func honestyStatusForLast(lastStatus: String) -> HonestyStatus {
        HonestyStatus.fromWire(lastStatus) ?? .unavailable
    }

    static func labeledLast(last: Double?, status: String) -> String {
        let bound = lastBound(status: status)
        let tag = status.trimmingCharacters(in: .whitespacesAndNewlines)
        let tagText = tag.isEmpty ? (bound ? "unknown" : "dark") : tag
        guard bound, let last, last.isFinite, last > 0 else {
            return "— · \(tagText)"
        }
        return "\(formatPrice(last)) · \(tagText)"
    }

    static func vsInvalidation(
        sideBuy: Bool,
        last: Double?,
        status: String,
        kind: String?,
        price: Double?
    ) -> BarWorkingLevelState {
        let k = kind?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        if k == "time" || k == "behaviour" || k == "behavior" || k == "context" {
            return .waiting
        }
        let isPrice = k == "price" || k.isEmpty && price != nil
        guard isPrice else { return .waiting }
        let bound = lastBound(status: status)
        guard bound, let last, last.isFinite, last > 0, let inv = price, inv > 0 else {
            return bound ? .waiting : .dark
        }
        if sideBuy {
            return last <= inv ? .breached : .intact
        }
        return last >= inv ? .breached : .intact
    }

    static func vsTarget(
        sideBuy: Bool,
        last: Double?,
        status: String,
        target: Double?
    ) -> BarWorkingLevelState {
        let bound = lastBound(status: status)
        guard bound, let last, last.isFinite, last > 0, let tgt = target, tgt > 0 else {
            return bound ? .waiting : .dark
        }
        if sideBuy {
            return last >= tgt ? .touched : .intact
        }
        return last <= tgt ? .touched : .intact
    }

    static func isPriceInvalidated(
        sideBuy: Bool,
        last: Double?,
        status: String,
        kind: String?,
        price: Double?
    ) -> Bool {
        vsInvalidation(sideBuy: sideBuy, last: last, status: status, kind: kind, price: price) == .breached
    }

    static func tone(for state: BarWorkingLevelState) -> BarEscrowRowTone {
        switch state {
        case .breached: return .red
        case .intact, .touched: return .green
        case .waiting: return .amber
        case .dark: return .dark
        }
    }

    /// Working list-row chip for invalidation level state.
    static func invalidationChipLabel(for state: BarWorkingLevelState) -> String {
        switch state {
        case .breached: return "Inv breached"
        case .intact: return "Inv intact"
        case .waiting: return "Inv waiting"
        case .dark: return "Inv dark"
        case .touched: return "Inv touched"
        }
    }

    static func formatPrice(_ v: Double) -> String {
        let f = NumberFormatter()
        f.maximumFractionDigits = 2
        f.minimumFractionDigits = 0
        return f.string(from: NSNumber(value: v)) ?? "\(v)"
    }

    static func formatQty(_ v: Double) -> String {
        v == v.rounded() ? String(Int(v.rounded())) : String(v)
    }
}
