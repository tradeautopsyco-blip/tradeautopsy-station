import Foundation

/// Canonical `POST /api/daemon/bar/declare` JSON dictionary for intraday-style declares (#116). Single place for keys + nesting.
enum BarIntradayDeclarationPayload {
    static func buildJSONObject(
        symbol: String,
        sideBuy: Bool,
        quantity: Double,
        stopLoss: Double,
        declarationKind: String,
        moodStress: Double,
        moodImpulse: Double,
        invalidationNote: String?,
        protectiveSlConsent: Bool,
        entryPrice: Double?,
        targetPrice: Double?,
        scalperSessionId: String?,
        isSessionLevel: Bool = false,
        setupTypeLabel: String? = nil,
        invalidationTypeWire: String? = nil,
    ) -> [String: Any] {
        var s1: [String: Any] = [
            "mood_stress": moodStress,
            "mood_impulse": moodImpulse,
        ]
        if let setup = setupTypeLabel?.trimmingCharacters(in: .whitespacesAndNewlines), !setup.isEmpty {
            s1["setup_type"] = setup
        }
        if let invT = invalidationTypeWire?.trimmingCharacters(in: .whitespacesAndNewlines), !invT.isEmpty {
            s1["invalidation_type"] = invT
        }
        if let inv = invalidationNote?.trimmingCharacters(in: .whitespacesAndNewlines), !inv.isEmpty {
            s1["invalidation"] = inv
        }
        var o: [String: Any] = [
            "symbol": symbol,
            "declaration_kind": declarationKind,
            "side": sideBuy ? "BUY" : "SELL",
            "quantity": quantity,
            "stop_loss": stopLoss,
            "declaration_payload": [
                "v": 1,
                "s1": s1,
                "protective_sl_consent": protectiveSlConsent,
            ] as [String: Any],
        ]
        if let ep = entryPrice { o["entry_price"] = ep }
        if let tp = targetPrice { o["target_price"] = tp }
        if let sid = scalperSessionId?.trimmingCharacters(in: .whitespacesAndNewlines), !sid.isEmpty {
            o["scalper_session_id"] = sid
        }
        if isSessionLevel {
            o["is_session_level"] = true
        }
        return o
    }
}
