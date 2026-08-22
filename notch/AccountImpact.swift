import Foundation

/// Closed-chip account impact (#36). Live pinned P&L vs daily loss limit — not composite score.
enum AccountImpactSign: Equatable {
    case down
    case up
    case zero
}

enum AccountImpact: Equatable {
    case unknown
    case known(percent: Int, sign: AccountImpactSign)

    var chipLabel: String {
        switch self {
        case .unknown:
            return "—"
        case .known(let percent, .down):
            return "\(percent)↓"
        case .known(let percent, .up):
            return "\(percent)↑"
        case .known(_, .zero):
            return "0"
        }
    }

    /// Fill of one half of the center-zero track, 0…1.
    var trackFill: Double {
        guard case .known(let percent, let sign) = self, sign != .zero else { return 0 }
        return min(1, Double(percent) / 100)
    }

    static func compute(
        positions: [(symbol: String, unrealizedPnL: Double)],
        pinnedSymbols: [String],
        dailyLossLimit: Double?,
    ) -> AccountImpact {
        guard let dailyLossLimit, dailyLossLimit > 0, dailyLossLimit.isFinite else {
            return .unknown
        }
        let pins = pinnedSymbols.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
        let selected: [(symbol: String, unrealizedPnL: Double)]
        if pins.isEmpty {
            selected = positions
        } else {
            let pinSet = Set(pins.map { $0.uppercased() })
            selected = positions.filter { pinSet.contains($0.symbol.uppercased()) }
        }
        let pnl = selected.reduce(0.0) { $0 + $1.unrealizedPnL }
        if pnl == 0 {
            return .known(percent: 0, sign: .zero)
        }
        let raw = (100 * abs(pnl) / dailyLossLimit).rounded()
        let percent = min(999, max(0, Int(raw)))
        return .known(percent: percent, sign: pnl < 0 ? .down : .up)
    }
}
