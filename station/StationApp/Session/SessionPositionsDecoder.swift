import Foundation

/// Pure decode of `GET /api/daemon/positions` JSON → desk positions + kill-switch flag.
/// Kept separate from `SessionModel` so unit tests can assert byte-identical parsing
/// without a live agent.
public enum SessionPositionsDecoder {
    public struct Result: Equatable {
        public var positions: [DeskPosition]
        public var killSwitchActive: Bool?
        public var openOrders: Int?

        public init(positions: [DeskPosition], killSwitchActive: Bool?, openOrders: Int?) {
            self.positions = positions
            self.killSwitchActive = killSwitchActive
            self.openOrders = openOrders
        }
    }

    public static func decode(_ data: Data) throws -> Result {
        let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
        let killSwitch = j["kill_switch_active"] as? Bool
        let openOrders = j["open_orders"] as? Int
        var out: [DeskPosition] = []
        if let arr = j["positions"] as? [[String: Any]] {
            for p in arr {
                let sym =
                    DeskBrokerTicker.fromPositionRow(p)
                    ?? (p["tradingSymbol"] as? String)
                    ?? (p["symbol"] as? String)
                    ?? "—"
                let qty = (p["quantity"] as? Int) ?? (p["qty"] as? Int) ?? 0
                let pnl = (p["unrealizedPnl"] as? Double) ?? (p["unrealized_pnl"] as? Double) ?? 0
                let dir = (p["direction"] as? String) ?? (p["side"] as? String) ?? ""
                out.append(DeskPosition(symbol: sym, qty: qty, unrealizedPnL: pnl, direction: dir))
            }
        }
        return Result(positions: out, killSwitchActive: killSwitch, openOrders: openOrders)
    }
}
