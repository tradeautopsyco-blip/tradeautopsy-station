import Foundation

/// Desk position row for pulse-strip / session surfaces.
/// Re-homed from notch's `NotchPosition` during dependency severance (renamed `DeskPosition`).
public struct DeskPosition: Identifiable, Equatable {
    public var id: String { symbol + "\(qty)" }
    public var symbol: String
    public var qty: Int
    public var unrealizedPnL: Double
    public var direction: String

    public init(symbol: String, qty: Int, unrealizedPnL: Double, direction: String) {
        self.symbol = symbol
        self.qty = qty
        self.unrealizedPnL = unrealizedPnL
        self.direction = direction
    }
}
