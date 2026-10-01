import Foundation

/// Armed PLAN — cancel pending declaration before fill (Wave 5 / Basecamp C2.6).
/// Journal rows stay on the sheet; this is not flatten or hard delete.
enum BarCancelDeclarationChrome {
    struct ReasonChip: Equatable, Sendable {
        let slug: String
        let label: String
    }

    static let reasonChips: [ReasonChip] = [
        ReasonChip(slug: "scratch", label: "Scratch"),
        ReasonChip(slug: "plan_change", label: "Plan changed"),
        ReasonChip(slug: "no_entry", label: "Not taking entry"),
    ]

    static let entryButtonTitle = "Cancel declaration"
    static let confirmPrompt = "Pick a reason, then confirm"
    static let notFlattenHint = "Cancels intent · not flatten · stays on journal"
    static let submittingTitle = "Cancelling declaration…"
    static let dismissTitle = "Back"

    static func showsControl(phase: BarSurfacePhase, declarationId: String?) -> Bool {
        guard phase == .armed else { return false }
        let trimmed = declarationId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return !trimmed.isEmpty
    }

    static func normalizedReasonChip(_ raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        return trimmed
    }
}
