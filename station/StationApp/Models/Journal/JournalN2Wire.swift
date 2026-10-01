import Foundation

/// Local N2 day sheet overlay from agent `n2_day_sheet` (Wave 7).
public struct JournalN2ConditionFire: Equatable, Sendable {
    public let ruleId: String
    public let firedAtMs: Int64

    public init(ruleId: String, firedAtMs: Int64) {
        self.ruleId = ruleId
        self.firedAtMs = firedAtMs
    }
}

public struct JournalN2Debrief: Equatable, Sendable {
    public let momentANote: String
    public let momentCNote: String
    public let momentCContext: String

    public init(momentANote: String, momentCNote: String, momentCContext: String) {
        self.momentANote = momentANote
        self.momentCNote = momentCNote
        self.momentCContext = momentCContext
    }

    public var momentCEmpty: Bool {
        momentCNote.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && momentCContext.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

public struct JournalN2DaySheet: Equatable, Sendable {
    public let localDate: String
    public let debrief: JournalN2Debrief
    public let conditionFires: [JournalN2ConditionFire]
    public let hasWorkingSnapshot: Bool

    public init(
        localDate: String,
        debrief: JournalN2Debrief,
        conditionFires: [JournalN2ConditionFire],
        hasWorkingSnapshot: Bool
    ) {
        self.localDate = localDate
        self.debrief = debrief
        self.conditionFires = conditionFires
        self.hasWorkingSnapshot = hasWorkingSnapshot
    }
}

enum JournalN2Wire {
    struct DaySheet: Decodable {
        var localDate: String?
        var debrief: Debrief?
        var conditionFires: [Fire]?
        var working: Working?

        enum CodingKeys: String, CodingKey {
            case debrief, working
            case localDate = "local_date"
            case conditionFires = "condition_fires"
        }
    }

    struct Debrief: Decodable {
        var momentANote: String?
        var momentCNote: String?
        var momentCContext: String?

        enum CodingKeys: String, CodingKey {
            case momentANote = "moment_a_note"
            case momentCNote = "moment_c_note"
            case momentCContext = "moment_c_context"
        }
    }

    struct Fire: Decodable {
        var ruleId: String?
        var firedAtMs: Int64?

        enum CodingKeys: String, CodingKey {
            case ruleId = "rule_id"
            case firedAtMs = "fired_at_ms"
        }
    }

    struct Working: Decodable {
        var invalidationState: String?

        enum CodingKeys: String, CodingKey {
            case invalidationState = "invalidation_state"
        }
    }

    static func map(_ sheet: DaySheet?) -> JournalN2DaySheet? {
        guard let sheet else { return nil }
        let deb = sheet.debrief
        let fires = (sheet.conditionFires ?? []).compactMap { f -> JournalN2ConditionFire? in
            guard let id = f.ruleId, !id.isEmpty, let ms = f.firedAtMs else { return nil }
            return JournalN2ConditionFire(ruleId: id, firedAtMs: ms)
        }
        return JournalN2DaySheet(
            localDate: sheet.localDate ?? "",
            debrief: JournalN2Debrief(
                momentANote: deb?.momentANote ?? "",
                momentCNote: deb?.momentCNote ?? "",
                momentCContext: deb?.momentCContext ?? ""
            ),
            conditionFires: fires,
            hasWorkingSnapshot: sheet.working != nil
        )
    }
}
