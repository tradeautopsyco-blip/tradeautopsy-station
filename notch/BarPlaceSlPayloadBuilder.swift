import Foundation

/// Pure `place_sl` trade leg resolution (#182) — declaration ticker wins over positions; never form field text.
enum BarPlaceSlPayloadBuilder {
    struct TradeLeg: Equatable, Sendable {
        let symbol: String
        let sideRaw: String
        let qty: Double
    }

    static func resolveTradeLeg(
        liveState: BarLiveStateResponse,
        positions: [NotchPosition],
    ) -> Result<TradeLeg, String> {
        let symbol: String
        let sideRaw: String
        let qty: Double

        if let pd = liveState.pendingDeclaration {
            symbol = pd.symbol
            sideRaw = pd.side
            qty = pd.quantity
        } else if let pos = positions.first(where: { candidate in
            guard let breakdown = liveState.composite?.breakdown, !breakdown.isEmpty else { return false }
            return breakdown.contains { $0.symbol.uppercased() == candidate.symbol.uppercased() }
        }) {
            symbol = pos.symbol
            sideRaw = pos.direction
            qty = Double(abs(pos.qty))
        } else if let pos = positions.first {
            symbol = pos.symbol
            sideRaw = pos.direction
            qty = Double(abs(pos.qty))
        } else {
            return .failure("Need an open position snapshot to place SL from Notch.")
        }

        guard let symNorm = BarBrokerTicker.normalize(raw: symbol) else {
            return .failure("Symbol must be a broker ticker (e.g. RELIANCE), not a company name.")
        }
        guard qty > 0 else {
            return .failure("Symbol or quantity missing.")
        }

        return .success(TradeLeg(symbol: symNorm, sideRaw: sideRaw, qty: qty))
    }
}
