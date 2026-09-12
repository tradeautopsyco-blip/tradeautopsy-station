import Foundation

/// Desk position row for pulse-strip / session surfaces.
/// Re-homed from notch's `NotchPosition` during dependency severance (renamed `DeskPosition`).
public struct DeskPosition: Identifiable, Equatable {
    public var id: String { symbol + "\(qty)" }
    public var symbol: String
    public var qty: Double
    public var unrealizedPnL: Double?
    public var direction: String
    /// Earliest fill of the current leftover lot. Nil means no overnight claim.
    public var firstFilledAt: Date?

    public init(
        symbol: String,
        qty: Double,
        unrealizedPnL: Double?,
        direction: String,
        firstFilledAt: Date? = nil
    ) {
        self.symbol = symbol
        self.qty = qty
        self.unrealizedPnL = unrealizedPnL
        self.direction = direction
        self.firstFilledAt = firstFilledAt
    }

    public init(symbol: String, qty: Int, unrealizedPnL: Double, direction: String) {
        self.init(symbol: symbol, qty: Double(qty), unrealizedPnL: unrealizedPnL, direction: direction)
    }
}
