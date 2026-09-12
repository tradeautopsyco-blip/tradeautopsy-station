import Foundation
import Notch

/// Detect card input from an open inventory row + Journal week coverage.
/// Same impulsive rule as `JournalWeek`: pending/matched symbols are covered.
public enum TodayDetectJoin {
    public static func coveredSymbols(from declarations: [JournalDeclarationCard]) -> Set<String> {
        Set(
            declarations
                .filter { $0.isPending || $0.isMatched }
                .map { $0.symbol.uppercased() }
        )
    }

    public static func input(
        position: DeskPosition?,
        declarations: [JournalDeclarationCard],
        quoteCurrency: String?,
        liveStop: Double? = nil
    ) -> DetectCardInput? {
        guard let position else { return nil }
        let symbol = position.symbol.uppercased()
        let covered = coveredSymbols(from: declarations)
        let match = declarations.first {
            ($0.isPending || $0.isMatched) && $0.symbol.uppercased() == symbol
        }
        let planStop = covered.contains(symbol) ? match?.snapshot.stopLoss : nil
        let quote = quoteCurrency?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let dir = position.direction.uppercased()
        let sideBuy = !dir.contains("SELL") && !dir.contains("SHORT")
        return DetectCardInput(
            qty: position.qty,
            entry: nil,
            planStop: planStop,
            liveStop: liveStop,
            sideBuy: sideBuy,
            accountEquity: nil,
            tradeCurrency: quote,
            accountCurrency: quote
        )
    }
}
