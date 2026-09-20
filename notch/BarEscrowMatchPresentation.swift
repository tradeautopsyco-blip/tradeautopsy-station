import Foundation

// MARK: - #125 — escrow match rows + summary (pure presentation)

enum BarEscrowRowTone: Equatable {
    case green
    case amber
    case red
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
        pending: BarPendingDeclaration? = nil
    ) -> [BarEscrowRowPresentation] {
        let data = rows(from: report)
        if !data.isEmpty { return data }
        if let pending {
            return pendingDeclaredRows(pending)
        }
        return []
    }

    /// LiveBook pending when Console escrow has no nodes — declared column only; actual stays —.
    static func pendingDeclaredRows(_ pending: BarPendingDeclaration) -> [BarEscrowRowPresentation] {
        let qty = pending.quantity
        let qtyText = qty == qty.rounded() ? String(Int(qty.rounded())) : String(qty)
        var rows: [BarEscrowRowPresentation] = [
            row(id: "pending.symbol", label: "Symbol", declared: pending.symbol),
            row(id: "pending.side", label: "Side / product", declared: pending.side),
            row(id: "pending.qty", label: "Quantity", declared: qtyText),
        ]
        if let stop = pending.stopLoss {
            rows.append(row(id: "pending.stop", label: "Stop / protect", declared: String(stop)))
        }
        if let target = pending.target {
            rows.append(row(id: "pending.target", label: "Target / policy", declared: String(target)))
        }
        return rows
    }

    private static func row(id: String, label: String, declared: String) -> BarEscrowRowPresentation {
        BarEscrowRowPresentation(
            id: id,
            label: label,
            declared: declared,
            actual: "—",
            tone: .amber,
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
        default: return .amber
        }
    }
}
