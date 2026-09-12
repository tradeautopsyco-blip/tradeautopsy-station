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
                let qty = SessionPositionsDecoder.qty(from: p)
                let pnl: Double?
                if p["unrealizedPnl"] is NSNull || p["unrealized_pnl"] is NSNull {
                    pnl = nil
                } else if let n = p["unrealizedPnl"] as? Double {
                    pnl = n
                } else if let n = p["unrealized_pnl"] as? Double {
                    pnl = n
                } else {
                    pnl = nil
                }
                let dir = (p["direction"] as? String) ?? (p["side"] as? String) ?? ""
                let firstFilledAt = parseFirstFilledAt(from: p)
                out.append(
                    DeskPosition(
                        symbol: sym,
                        qty: qty,
                        unrealizedPnL: pnl,
                        direction: dir,
                        firstFilledAt: firstFilledAt
                    )
                )
            }
        }
        return Result(positions: out, killSwitchActive: killSwitch, openOrders: openOrders)
    }

    static func qty(from p: [String: Any]) -> Double {
        if let n = p["qty"] as? Double { return n }
        if let n = p["quantity"] as? Double { return n }
        if let n = p["qty"] as? Int { return Double(n) }
        if let n = p["quantity"] as? Int { return Double(n) }
        return 0
    }

    static func parseFirstFilledAt(from p: [String: Any]) -> Date? {
        let raw = p["firstFilledAt"] ?? p["first_filled_at"]
        if raw is NSNull { return nil }
        if let text = raw as? String, !text.isEmpty {
            let fractional = ISO8601DateFormatter()
            fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
            if let date = fractional.date(from: text) { return date }
            return ISO8601DateFormatter().date(from: text)
        }
        if let ms = raw as? Int {
            return Date(timeIntervalSince1970: TimeInterval(ms) / 1000)
        }
        if let ms = raw as? Double {
            return Date(timeIntervalSince1970: ms / 1000)
        }
        return nil
    }
}
