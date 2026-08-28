import Foundation

/// Canonical `POST /api/daemon/bar/declare` JSON dictionary for intraday-style declares (#116). Single place for keys + nesting.
enum BarIntradayDeclarationPayload {
    struct OptionLeg: Equatable, Sendable {
        var underlying: String
        var expiry: String
        var strike: String
        var right: String
        var sideBuy: Bool
        var lots: Int
    }

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
        optionLeg: OptionLeg? = nil,
        optionLegs: [OptionLeg] = [],
        horizonDays: Int? = nil,
        maxPlannedLossINR: Double? = nil,
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
        var quantityOut = quantity
        var legsOut: [OptionLeg] = optionLegs
        if legsOut.isEmpty, let optionLeg {
            legsOut = [optionLeg]
        }
        if let first = legsOut.first {
            quantityOut = Double(first.lots)
        }
        var o: [String: Any] = [
            "symbol": symbol,
            "declaration_kind": declarationKind,
            "side": sideBuy ? "BUY" : "SELL",
            "quantity": quantityOut,
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
        if let days = horizonDays {
            o["horizon_days"] = days
        }
        if let maxLoss = maxPlannedLossINR {
            o["max_planned_loss_inr"] = maxLoss
        }
        if !legsOut.isEmpty {
            o["legs"] = legsOut.map { leg in
                [
                    "underlying": leg.underlying,
                    "expiry": leg.expiry,
                    "strike": leg.strike,
                    "right": leg.right == "PE" ? "PE" : "CE",
                    "side": leg.sideBuy ? "BUY" : "SELL",
                    "lots": leg.lots,
                ] as [String: Any]
            }
        }
        return o
    }
}
