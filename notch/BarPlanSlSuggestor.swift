import Foundation

/// SL suggestor from % of margin — formula blocked until `docs/reference/` cites sizing (`POSITION-SIZING.md`).
enum BarPlanSlSuggestor {
    struct Input: Equatable, Sendable {
        var riskPercentOfMargin: Double?
        var marginLit: Bool
        var marginDisplay: String
        var entry: Double?
        var sideBuy: Bool
        var bookId: String
    }

    struct Presentation: Equatable, Sendable {
        var suggestedStopText: String
        var suggestedSizeText: String
        var footnote: String
    }

    static let blockedFootnote =
        "Stop/size from margin % is not shipped — POSITION-SIZING.md has no primary formula. DualNoBlend per book."

    static func present(_ input: Input) -> Presentation {
        guard input.marginLit else {
            return Presentation(
                suggestedStopText: "—",
                suggestedSizeText: "—",
                footnote: "Margin dark — cannot suggest stop from margin %."
            )
        }
        guard let pct = input.riskPercentOfMargin, pct > 0, pct.isFinite else {
            return Presentation(
                suggestedStopText: "—",
                suggestedSizeText: "—",
                footnote: blockedFootnote
            )
        }
        _ = input.entry
        _ = input.sideBuy
        _ = input.bookId
        return Presentation(
            suggestedStopText: "—",
            suggestedSizeText: "—",
            footnote: blockedFootnote
        )
    }
}
