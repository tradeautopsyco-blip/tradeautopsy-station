import Foundation

enum BarPlanKillPhase: Equatable {
    case idle
    case warning
}

/// PLAN Kill chrome. Stop me is gone; Confirm fires T5 (PRD Q6–Q11).
enum BarPlanKillChrome {
    struct Presentation: Equatable {
        let showsKillButton: Bool
        let showsStopMe: Bool
        let showsWarningCard: Bool
        let killButtonTitle: String
        let warningTitle: String
        let warningBody: String
        let confirmTitle: String
        let cancelTitle: String
    }

    static func presentation(phase: BarPlanKillPhase, agentUp: Bool) -> Presentation {
        let showKill = agentUp && phase == .idle
        let showCard = agentUp && phase == .warning
        return Presentation(
            showsKillButton: showKill,
            showsStopMe: false,
            showsWarningCard: showCard,
            killButtonTitle: "Kill",
            warningTitle: "Kill",
            warningBody: "This locks the desk. Overlay stays until you tap I'm Calm after the countdown. Broker sites for the armed slug may be blocked. Stop is not Kill. This is not a max-loss flatten.",
            confirmTitle: "Confirm",
            cancelTitle: "Cancel"
        )
    }

    /// Policy default_level applies when `level` is omitted.
    static func fireBody(reason: String = "notch_manual") -> [String: Any] {
        ["reason": reason]
    }
}
