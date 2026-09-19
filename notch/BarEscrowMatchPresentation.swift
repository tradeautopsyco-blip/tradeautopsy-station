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
    /// Seven fixed ledger slots for Notch mockup parity (#6); pads with placeholders when the server sends fewer nodes.
    static let ledgerSlotCount = 7

    static func sevenSlotRows(
        from report: BarEscrowMatchReport?,
        pending: BarPendingDeclaration? = nil
    ) -> [BarEscrowRowPresentation] {
        let data = rows(from: report)
        if data.isEmpty, let pending {
            return sevenSlotRowsFromPending(pending)
        }
        var out: [BarEscrowRowPresentation] = []
        for i in 0 ..< ledgerSlotCount {
            if i < data.count {
                out.append(data[i])
            } else {
                out.append(
                    BarEscrowRowPresentation(
                        id: "placeholder.\(i)",
                        label: ledgerPlaceholderLabel(index: i),
                        declared: "—",
                        actual: "—",
                        tone: .amber,
                        breakReason: nil,
                    ),
                )
            }
        }
        return out
    }

    /// LiveBook pending when Console escrow has no nodes — declared column only; actual stays —.
    static func sevenSlotRowsFromPending(_ pending: BarPendingDeclaration) -> [BarEscrowRowPresentation] {
        let qty = pending.quantity
        let qtyText = qty == qty.rounded() ? String(Int(qty.rounded())) : String(qty)
        let stop = pending.stopLoss.map { String($0) } ?? "—"
        let target = pending.target.map { String($0) } ?? "—"
        let declared = [
            "—",
            pending.symbol,
            pending.side,
            qtyText,
            "—",
            stop,
            target,
        ]
        return (0 ..< ledgerSlotCount).map { i in
            BarEscrowRowPresentation(
                id: "pending.\(i)",
                label: ledgerPlaceholderLabel(index: i),
                declared: i < declared.count ? declared[i] : "—",
                actual: "—",
                tone: .amber,
                breakReason: nil,
            )
        }
    }

    private static func ledgerPlaceholderLabel(index: Int) -> String {
        let labels = [
            "Venue / feed",
            "Symbol",
            "Side / product",
            "Quantity",
            "Entry",
            "Stop / protect",
            "Target / policy",
        ]
        return index < labels.count ? labels[index] : "Row \(index + 1)"
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
            return "Escrow match — see nodes"
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
