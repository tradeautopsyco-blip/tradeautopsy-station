import Foundation

/// Pure Working desk readouts — chart levels, margin honesty, live conditions (no SwiftUI).
enum BarWorkingLivePresentation {
    struct ConditionCell: Equatable {
        let title: String
        let value: String
        let subtitle: String
    }

    static func planLevels(pending: BarPendingDeclaration?) -> SessionChartPlanLevels? {
        guard let pending else { return nil }
        let sideBuy = !pending.side.uppercased().contains("SELL")
        let snap = pending.planSnapshot
        let entry = resolvedEntry(pending: pending, snapshot: snap)
        let stop = positive(pending.stopLoss) ?? positive(snap?.stopLoss)
        let target = positive(pending.target) ?? positive(snap?.targetPrice)
        guard entry != nil || stop != nil || target != nil else { return nil }
        return SessionChartPlanLevels(sideBuy: sideBuy, entry: entry, stop: stop, target: target)
    }

    static func unrealizedSubtitle(pending: BarPendingDeclaration?, hasOpenFill: Bool) -> String {
        if hasOpenFill { return "mark vs avg fill" }
        if pending != nil { return "Pending — no fill yet" }
        return "No active position"
    }

    static func marginDisplay(
        brokerSyncClass: String,
        barSyncState: String?,
        fundsStatus: String,
        freeText: String,
    ) -> (value: String, subtitle: String) {
        let broker = brokerSyncClass.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let barSync = barSyncState?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
        if broker == "not_connected" || barSync == "NOT_CONNECTED" {
            return ("—", "Broker not synced")
        }
        if fundsStatus == "success", !freeText.isEmpty, freeText != "—" {
            return (freeText, "available · funds")
        }
        if let honesty = HonestyStatus.fromWire(fundsStatus) {
            return ("—", honesty.rawValue.replacingOccurrences(of: "_", with: " "))
        }
        return ("—", "margin unavailable")
    }

    static func conditionCells(
        symbol: String,
        last: Double?,
        lastStatus: String,
        historyStatus: String,
        planState: String?,
        barSyncState: String?,
        brokerSyncClass: String,
    ) -> [ConditionCell] {
        let sym = symbol.trimmingCharacters(in: .whitespacesAndNewlines)
        let lastLine = BarWorkingCompare.labeledLast(last: last, status: lastStatus)
        let sessionLine: String = {
            if historyStatus == "success" { return "Session · live" }
            if let h = HonestyStatus.fromWire(historyStatus) {
                return "Session · \(h.rawValue)"
            }
            return "Session · \(historyStatus)"
        }()
        let plan = (planState?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()).flatMap { $0.isEmpty ? nil : $0 } ?? "—"
        let syncLine = syncHonestyLabel(barSyncState: barSyncState, brokerSyncClass: brokerSyncClass)
        return [
            ConditionCell(title: "SYMBOL", value: sym.isEmpty ? "—" : sym, subtitle: "working desk"),
            ConditionCell(title: "LAST", value: lastLine, subtitle: "market/quote"),
            ConditionCell(title: "SESSION", value: sessionLine, subtitle: "history lane"),
            ConditionCell(title: "PLAN", value: plan, subtitle: "server plan_state"),
            ConditionCell(title: "SYNC", value: syncLine, subtitle: "broker lane"),
        ]
    }

    private static func syncHonestyLabel(barSyncState: String?, brokerSyncClass: String) -> String {
        let broker = brokerSyncClass.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if broker == "not_connected" { return "Not connected" }
        let bar = barSyncState?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if bar.isEmpty { return "Unknown" }
        return bar.replacingOccurrences(of: "_", with: " ").capitalized
    }

    private static func resolvedEntry(pending: BarPendingDeclaration, snapshot: BarPlanSnapshotSummary?) -> Double? {
        if let fill = positive(pending.avgFill) { return fill }
        return positive(snapshot?.entryPrice)
    }

    private static func positive(_ v: Double?) -> Double? {
        guard let v, v > 0, v.isFinite else { return nil }
        return v
    }

    /// Compact chip for Working list rows (`harness-trade-arc.md` §5).
    static func listInvalidationChip(
        pending: BarPendingDeclaration,
        last: Double?,
        lastStatus: String
    ) -> String {
        let snap = pending.planSnapshot
        let kind = snap?.resolvedInvalidationKind ?? "price"
        let price = snap?.resolvedInvalidationPrice
        let sideBuy = !pending.side.uppercased().contains("SELL")
        let state = BarWorkingCompare.vsInvalidation(
            sideBuy: sideBuy,
            last: last,
            status: lastStatus,
            kind: kind,
            price: price
        )
        return BarWorkingCompare.invalidationChipLabel(for: state)
    }
}
