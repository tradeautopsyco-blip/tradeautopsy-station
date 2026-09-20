import Foundation

// MARK: - #5 — Notch post-trade debrief JSON for PATCH `/api/daemon/bar/post-trade-debrief`

struct BarPostTradeAdherenceAnswers: Equatable {
    var stopAsDeclared: Bool
    var sizeAsDeclared: Bool
    var invalidationRespected: Bool
    var exitPerPlan: Bool
    var noImpulsiveAdd: Bool

    var allAffirmed: Bool {
        stopAsDeclared && sizeAsDeclared && invalidationRespected && exitPerPlan && noImpulsiveAdd
    }

    func asJSONObject() -> [String: Bool] {
        [
            "stop_as_declared": stopAsDeclared,
            "size_as_declared": sizeAsDeclared,
            "invalidation_respected": invalidationRespected,
            "exit_per_plan": exitPerPlan,
            "no_impulsive_add": noImpulsiveAdd,
        ]
    }
}

enum BarPostTradeDebriefPayload {
    /// Payload shape only (#5) — server upserts notes on the declaration, then ingestSignal.
    static func buildJSONObject(
        momentANote: String,
        adherence: BarPostTradeAdherenceAnswers,
        momentCContext: String,
        momentCNote: String,
        declarationId: String?,
        completedAtMs: Int,
        liveNote: String = "",
        emotionOut: Int? = nil,
        captureIds: [String] = [],
        impulsive: Bool = false,
        symbol: String? = nil,
        side: String? = nil,
        quantity: Double? = nil,
        stance: String? = nil,
    ) -> [String: Any] {
        var o: [String: Any] = [
            "v": 1,
            "moment_a_note": momentANote,
            "adherence": adherence.asJSONObject(),
            "moment_c_context": momentCContext,
            "moment_c_note": momentCNote,
            "completed_at_ms": completedAtMs,
        ]
        if let declarationId, !declarationId.isEmpty {
            o["declaration_id"] = declarationId
        }
        if !liveNote.isEmpty {
            o["live_note"] = liveNote
        }
        if let emotionOut, (1...5).contains(emotionOut) {
            o["emotion_out"] = emotionOut
        }
        if !captureIds.isEmpty {
            o["capture_ids"] = captureIds
        }
        if impulsive {
            o["impulsive"] = true
            o["stance"] = "reactive"
            if let symbol, !symbol.isEmpty { o["symbol"] = symbol }
            if let side, !side.isEmpty { o["side"] = side }
            if let quantity { o["quantity"] = quantity }
        } else if let stance, !stance.isEmpty {
            o["stance"] = stance
        }
        return o
    }
}
