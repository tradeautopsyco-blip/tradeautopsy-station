import Foundation

enum BarPlaceSlPayloadError: Error, LocalizedError {
    case noPositionSnapshot
    case invalidSymbol
    case missingQuantity

    var errorDescription: String? {
        switch self {
        case .noPositionSnapshot:
            return "Need an open position snapshot to place SL from Notch."
        case .invalidSymbol:
            return "Symbol must be a broker ticker (e.g. RELIANCE), not a company name."
        case .missingQuantity:
            return "Symbol or quantity missing."
        }
    }
}

/// Pure trade-leg resolution (#182) — declaration ticker wins over positions; never form field text.
/// T5 K3 — remains for Cancel / unit tests. Do not wire a Notch `place_sl` send.
enum BarPlaceSlPayloadBuilder {
    struct TradeLeg: Equatable, Sendable {
        let symbol: String
        let sideRaw: String
        let qty: Double
    }

    static func resolveTradeLeg(
        liveState: BarLiveStateResponse,
        positions: [NotchPosition],
    ) -> Result<TradeLeg, BarPlaceSlPayloadError> {
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
            return .failure(.noPositionSnapshot)
        }

        guard let symNorm = BarBrokerTicker.normalize(raw: symbol) else {
            return .failure(.invalidSymbol)
        }
        guard qty > 0 else {
            return .failure(.missingQuantity)
        }

        return .success(TradeLeg(symbol: symNorm, sideRaw: sideRaw, qty: qty))
    }
}
