import Foundation

/// POST `/api/daemon/bar/declare` body for **`declaration_kind`: `scalper_session`** (#117).
/// Top-level `is_session_level` matches `createDeclarationBodySchema` / `declarations-persist` merge semantics.
enum BarScalperSessionDeclarePayload {
    static func buildJSONObject(
        scalperSessionId: String,
        symbol: String,
        sideBuy: Bool,
        quantity: Double,
        stopLoss: Double,
        moodStress: Double,
        moodImpulse: Double,
        invalidationNote: String?,
        protectiveSlConsent: Bool,
    ) -> [String: Any]? {
        guard protectiveSlConsent else { return nil }
        let sid = scalperSessionId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !sid.isEmpty else { return nil }
        let sym = symbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard !sym.isEmpty, quantity > 0, stopLoss > 0 else { return nil }

        var s1: [String: Any] = [
            "mood_stress": moodStress,
            "mood_impulse": moodImpulse,
        ]
        if let inv = invalidationNote?.trimmingCharacters(in: .whitespacesAndNewlines), !inv.isEmpty {
            s1["invalidation"] = inv
        }

        let declarationPayload: [String: Any] = [
            "v": 1,
            "s1": s1,
            "protective_sl_consent": protectiveSlConsent,
        ]

        return [
            "symbol": sym,
            "declaration_kind": "scalper_session",
            "side": sideBuy ? "BUY" : "SELL",
            "quantity": quantity,
            "stop_loss": stopLoss,
            "is_session_level": true,
            "scalper_session_id": sid,
            "declaration_payload": declarationPayload,
        ]
    }

    static func buildJSONData(
        scalperSessionId: String,
        symbol: String,
        sideBuy: Bool,
        quantity: Double,
        stopLoss: Double,
        moodStress: Double,
        moodImpulse: Double,
        invalidationNote: String?,
        protectiveSlConsent: Bool,
    ) -> Data? {
        guard let o = buildJSONObject(
            scalperSessionId: scalperSessionId,
            symbol: symbol,
            sideBuy: sideBuy,
            quantity: quantity,
            stopLoss: stopLoss,
            moodStress: moodStress,
            moodImpulse: moodImpulse,
            invalidationNote: invalidationNote,
            protectiveSlConsent: protectiveSlConsent,
        )
        else { return nil }
        return try? JSONSerialization.data(withJSONObject: o)
    }
}
