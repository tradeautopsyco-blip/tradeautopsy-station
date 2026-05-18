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
    /// Payload shape only (#5) — server persists via `ingestSignal` on hosted route.
    static func buildJSONObject(
        momentANote: String,
        adherence: BarPostTradeAdherenceAnswers,
        momentCContext: String,
        momentCNote: String,
        declarationId: String?,
        completedAtMs: Int,
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
        return o
    }
}
