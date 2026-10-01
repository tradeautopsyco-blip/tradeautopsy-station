import Foundation

/// Plan size display — formula blocked until `docs/reference/` cites sizing (Wave 4.2).
enum BarPlanSizer {
    struct Inputs: Equatable, Sendable {
        var dailyFloor: Double?
        var entry: Double?
        var invalidationPrice: Double?
        var stopLoss: Double?
        var fundsLit: Bool
    }

    struct Presentation: Equatable, Sendable {
        var plannedRiskText: String
        var sizeText: String
        var sizeEmphasisDashed: Bool
        var footnote: String
    }

    /// Primary-source sizing rule is **NOT SPECIFIED IN SOURCE** — never invent qty from capital %.
    static let formulaBlockedFootnote =
        "Sizer formula not shipped — no reference doc. Enter quantity manually."

    static func present(_ input: Inputs, quoteCurrency: String) -> Presentation {
        let riskText = formatPlannedRisk(input.dailyFloor, quoteCurrency: quoteCurrency)
        let invalidationLit = invalidationDistance(input) != nil
        let sizeEmphasisDashed = !input.fundsLit || !invalidationLit

        return Presentation(
            plannedRiskText: riskText,
            sizeText: "—",
            sizeEmphasisDashed: sizeEmphasisDashed,
            footnote: Self.formulaBlockedFootnote
        )
    }

    private static func formatPlannedRisk(_ floor: Double?, quoteCurrency: String) -> String {
        guard let floor else { return "—" }
        let ccy = quoteCurrency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let f = NumberFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.numberStyle = .decimal
        f.maximumFractionDigits = 0
        f.minimumFractionDigits = 0
        let n = f.string(from: NSNumber(value: floor)) ?? String(format: "%.0f", floor)
        if ccy.isEmpty { return n }
        return "\(ccy) \(n)"
    }

    private static func invalidationDistance(_ input: Inputs) -> Double? {
        guard let entry = input.entry, entry > 0 else { return nil }
        if let inv = input.invalidationPrice, inv > 0 {
            let d = abs(entry - inv)
            return d > 0 ? d : nil
        }
        if let stop = input.stopLoss, stop > 0 {
            let d = abs(entry - stop)
            return d > 0 ? d : nil
        }
        return nil
    }
}
