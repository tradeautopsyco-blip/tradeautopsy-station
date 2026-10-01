import Foundation

/// Stop-from-size is not in the locked Nautilus identity. Size-from-stop is the risk preview.
enum BarPlanSlSuggestor {
    struct Input: Equatable, Sendable {
        var riskPercentOfMargin: Double?
        var marginLit: Bool
        var marginDisplay: String
        var entry: Double?
        var sideBuy: Bool
        var bookId: String
        var authoredSizeText: String = "—"
    }

    struct Presentation: Equatable, Sendable {
        var suggestedStopText: String
        var suggestedSizeText: String
        var footnote: String
    }

    static let stopNotLockedFootnote =
        "Suggested stop is not in the locked formula. Size uses risk % and the stop when the preview can author it. POSITION-SIZING.md."

    static func present(_ input: Input) -> Presentation {
        guard input.marginLit else {
            return Presentation(
                suggestedStopText: "—",
                suggestedSizeText: "—",
                footnote: "Margin dark — cannot suggest a size from margin %."
            )
        }
        let size = input.authoredSizeText.trimmingCharacters(in: .whitespacesAndNewlines)
        let sizeText = size.isEmpty ? "—" : size
        return Presentation(
            suggestedStopText: "—",
            suggestedSizeText: sizeText,
            footnote: stopNotLockedFootnote
        )
    }
}
