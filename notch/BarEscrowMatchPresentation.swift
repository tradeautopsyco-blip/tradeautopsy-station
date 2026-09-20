import Foundation

// MARK: - #125 — escrow match rows + summary (pure presentation)

enum BarEscrowRowTone: Equatable {
    case green
    case amber
    case red
    case dark
}

struct BarEscrowRowPresentation: Identifiable, Equatable {
    let id: String
    let label: String
    let declared: String
    let actual: String
    let tone: BarEscrowRowTone
    let breakReason: String?
}

enum BarEscrowMatchPresentation {
    static func matchRows(
        from report: BarEscrowMatchReport?,
        pending: BarPendingDeclaration? = nil,
        last: Double? = nil,
        lastStatus: String = "unavailable"
    ) -> [BarEscrowRowPresentation] {
        if let pending {
            return pendingDeclaredRows(pending, last: last, lastStatus: lastStatus)
        }
        return rows(from: report)
    }

    /// LiveBook pending — declared from Confirm; Actual from fills + labeled last vs invalidation/target.
    static func pendingDeclaredRows(
        _ pending: BarPendingDeclaration,
        last: Double? = nil,
        lastStatus: String = "unavailable"
    ) -> [BarEscrowRowPresentation] {
        let qtyText = BarWorkingCompare.formatQty(pending.quantity)
        let product = pending.planSnapshot?.product?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let declaredSide = product.isEmpty ? pending.side : "\(pending.side) · \(product)"
        let fillSym = pending.fillSymbol?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let fillSide = pending.fillSide?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let filledQty = pending.filledQty
        let avgFill = pending.avgFill
        let sideBuy = !pending.side.uppercased().contains("SELL")
        let snap = pending.planSnapshot
        let invKind = snap?.resolvedInvalidationKind
        let invPrice = snap?.resolvedInvalidationPrice
        let invState = BarWorkingCompare.vsInvalidation(
            sideBuy: sideBuy,
            last: last,
            status: lastStatus,
            kind: invKind,
            price: invPrice
        )
        let targetVal = pending.target ?? snap?.targetPrice
        let tgtState = BarWorkingCompare.vsTarget(
            sideBuy: sideBuy,
            last: last,
            status: lastStatus,
            target: targetVal
        )
        let lastText = BarWorkingCompare.labeledLast(last: last, status: lastStatus)
        let invDeclared: String = {
            if let p = invPrice, p > 0 {
                let kind = invKind?.isEmpty == false ? invKind! : "price"
                return "\(kind) \(BarWorkingCompare.formatPrice(p))"
            }
            let line = snap?.resolvedInvalidationLine ?? ""
            if !line.isEmpty { return line }
            if let k = invKind, !k.isEmpty { return k }
            return "—"
        }()
        let tgtDeclared = targetVal.map { BarWorkingCompare.formatPrice($0) } ?? "—"

        var rows: [BarEscrowRowPresentation] = [
            row(
                id: "pending.symbol",
                label: "Symbol",
                declared: pending.symbol,
                actual: fillSym.isEmpty ? "—" : fillSym,
                tone: fillSym.isEmpty ? .amber : .green
            ),
            row(
                id: "pending.side",
                label: "Side / product",
                declared: declaredSide,
                actual: fillSide.isEmpty ? "—" : fillSide,
                tone: fillSide.isEmpty ? .amber : .green
            ),
            row(
                id: "pending.qty",
                label: "Quantity",
                declared: qtyText,
                actual: filledQty.map { BarWorkingCompare.formatQty($0) } ?? "—",
                tone: filledQty == nil ? .amber : .green
            ),
            row(
                id: "pending.avg",
                label: "Avg fill",
                declared: "—",
                actual: avgFill.map { BarWorkingCompare.formatPrice($0) } ?? "—",
                tone: avgFill == nil ? .amber : .green
            ),
            row(
                id: "pending.invalidation",
                label: "Invalidation",
                declared: invDeclared,
                actual: invState == .waiting ? "waiting" : lastText,
                tone: BarWorkingCompare.tone(for: invState)
            ),
            row(
                id: "pending.target",
                label: "Target / policy",
                declared: tgtDeclared,
                actual: lastText,
                tone: BarWorkingCompare.tone(for: tgtState)
            ),
        ]
        if let stop = pending.stopLoss {
            rows.append(
                row(
                    id: "pending.stop",
                    label: "Stop / protect",
                    declared: BarWorkingCompare.formatPrice(stop),
                    actual: "—",
                    tone: .amber
                )
            )
        }
        return rows
    }

    private static func row(
        id: String,
        label: String,
        declared: String,
        actual: String = "—",
        tone: BarEscrowRowTone = .amber
    ) -> BarEscrowRowPresentation {
        BarEscrowRowPresentation(
            id: id,
            label: label,
            declared: declared,
            actual: actual,
            tone: tone,
            breakReason: nil
        )
    }

    static func rows(from report: BarEscrowMatchReport?) -> [BarEscrowRowPresentation] {
        guard let report, !report.nodes.isEmpty else { return [] }
        return report.nodes.map { node in
            BarEscrowRowPresentation(
                id: node.id,
                label: node.label,
                declared: node.declared,
                actual: node.actual,
                tone: tone(from: node.match),
                breakReason: node.breakReason,
            )
        }
    }

    /// Single-line summary when server sends scores; **nil** when there is nothing honest to show (#125).
    static func summaryLine(from report: BarEscrowMatchReport?) -> String? {
        guard let report else { return nil }
        let hasNodes = !report.nodes.isEmpty
        let hasFidelity = report.fidelityPct != nil
        let hasPre = report.preTradeSeconds != nil
        if !hasNodes, !hasFidelity, !hasPre { return nil }
        var parts: [String] = []
        if let p = report.fidelityPct, p.isFinite {
            let pct = Int(p.rounded())
            parts.append("Escrow fidelity ~\(pct)%")
        }
        if let s = report.preTradeSeconds, s >= 0 {
            parts.append("pre-trade \(s)s")
        }
        if parts.isEmpty, hasNodes {
            return "Match — see nodes"
        }
        if parts.isEmpty { return nil }
        return parts.joined(separator: " · ")
    }

    static func tone(from raw: String) -> BarEscrowRowTone {
        switch raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "green": return .green
        case "red": return .red
        case "dark": return .dark
        default: return .amber
        }
    }
}
