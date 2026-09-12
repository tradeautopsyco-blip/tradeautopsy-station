import Foundation

/// Station Journal paint: week-list snapshot only. Decode may still carry notes/fidelity.
public struct JournalDrawerRow: Equatable, Sendable {
    public let label: String
    public let value: String

    public init(label: String, value: String) {
        self.label = label
        self.value = value
    }
}

public enum JournalCardPaint {
    public static func snapLine(_ card: JournalDeclarationCard) -> String {
        let setup = card.snapshot.setupLabel ?? "—"
        let sl = numberText(card.snapshot.stopLoss)
        let inv = card.snapshot.invalidationKind ?? "—"
        return "\(setup) · SL \(sl) · invalidation \(inv)"
    }

    public static func statusChips(_ card: JournalDeclarationCard) -> [String] {
        [statusLabel(card.status)]
    }

    public static func drawerRows(_ card: JournalDeclarationCard) -> [JournalDrawerRow] {
        var rows = [
            JournalDrawerRow(label: "Kind", value: kindLabel(card.declarationKind)),
            JournalDrawerRow(label: "Setup", value: card.snapshot.setupLabel ?? "—"),
            JournalDrawerRow(label: "Calm", value: calmLine(card.snapshot.calmScale)),
            JournalDrawerRow(label: "Confidence", value: confLine(card.snapshot.confidenceScale)),
            JournalDrawerRow(label: "SL", value: numberText(card.snapshot.stopLoss)),
            JournalDrawerRow(label: "Target", value: numberText(card.snapshot.target)),
            JournalDrawerRow(label: "Invalidation", value: card.snapshot.invalidationLine ?? "—"),
            JournalDrawerRow(label: "SL consent", value: card.protectiveSlConsent ? "Yes" : "No"),
            JournalDrawerRow(label: "Qty declared", value: qtyText(card.quantity)),
            JournalDrawerRow(label: "Qty filled", value: card.quantityFilled.map(qtyText) ?? "—"),
        ]
        if let net = card.citedNet, let ccy = card.citedCurrency {
            rows.append(JournalDrawerRow(label: "Matched net", value: "\(net) \(ccy)"))
        }
        return rows
    }

    public static func kindLabel(_ k: String) -> String {
        switch k {
        case "intraday": return "Intraday"
        case "swing": return "Swing"
        case "positional": return "Positional"
        case "scalper_session": return "Scalper"
        case "pre_market": return "Pre-market"
        default: return k
        }
    }

    public static func statusLabel(_ s: String) -> String {
        switch s {
        case "matched": return "Matched"
        case "pending": return "Pending"
        case "expired": return "Expired"
        case "cancelled": return "Cancelled"
        default: return s
        }
    }

    public static func calmLine(_ n: Double?) -> String {
        guard let n else { return "—" }
        let i = Int(n.rounded())
        let word = ["", "Calm", "Focused", "Tense", "Anxious", "Angry"][safe: i] ?? ""
        return word.isEmpty ? "\(i)" : "\(i) \(word)"
    }

    public static func confLine(_ n: Double?) -> String {
        guard let n else { return "—" }
        return "\(Int(n.rounded()))"
    }

    public static func numberText(_ n: Double?) -> String {
        guard let n else { return "—" }
        return String(format: n == n.rounded() ? "%.0f" : "%.2f", n)
    }

    public static func qtyText(_ n: Double) -> String {
        n == n.rounded() ? String(Int(n)) : String(n)
    }
}

private extension Array where Element == String {
    subscript(safe index: Int) -> String? {
        indices.contains(index) ? self[index] : nil
    }
}
