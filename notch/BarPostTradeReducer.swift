import Foundation

// MARK: - #124 — post-trade debrief moments (A → B → C); fidelity gated after C

struct BarPostTradeState: Equatable {
    var momentAAcknowledged: Bool
    var momentBAcknowledged: Bool
    var momentCAcknowledged: Bool
    /// Shown only after moment C is acknowledged — spec: fidelity ring gated until terminal.
    var fidelityRingVisible: Bool

    static let initial = BarPostTradeState(
        momentAAcknowledged: false,
        momentBAcknowledged: false,
        momentCAcknowledged: false,
        fidelityRingVisible: false,
    )
}

enum BarPostTradeAction: Equatable {
    case acknowledgeMomentA
    case acknowledgeMomentB
    case acknowledgeMomentC
}

enum BarPostTradeReducer {
    static func reduce(state: BarPostTradeState, action: BarPostTradeAction) -> BarPostTradeState {
        var next = state
        switch action {
        case .acknowledgeMomentA:
            guard !next.momentAAcknowledged else { return state }
            next.momentAAcknowledged = true
        case .acknowledgeMomentB:
            guard next.momentAAcknowledged, !next.momentBAcknowledged else { return state }
            next.momentBAcknowledged = true
        case .acknowledgeMomentC:
            guard next.momentBAcknowledged, !next.momentCAcknowledged else { return state }
            next.momentCAcknowledged = true
            next.fidelityRingVisible = true
        }
        return next
    }
}
