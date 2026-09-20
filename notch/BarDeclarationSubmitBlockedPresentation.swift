import Foundation
import SwiftUI

/// Copy when hosted live-state sets `declaration_submit_blocked` (#183).
enum BarDeclarationSubmitBlockedPresentation {
    static func message(blocked: Bool, activeInterventions: [ActiveIntervention]) -> String {
        guard blocked else {
            return "Circuit active — finish or clear the Harness intervention before declaring."
        }
        let sorted = BarInterventionCardSpec.sortedInterventions(activeInterventions)
        if let primary = sorted.first {
            let kind = humanKind(primary.interventionType)
            let body = primary.primaryMessage.trimmingCharacters(in: .whitespacesAndNewlines)
            if body.isEmpty {
                return "Declaration blocked — \(kind). Resolve in Harness before declaring."
            }
            return "\(body) (\(kind))"
        }
        return "Circuit active — finish or clear the Harness intervention before declaring."
    }

    private static func humanKind(_ raw: String) -> String {
        switch raw.lowercased() {
        case "bar_kill_switch", "kill_switch":
            return "kill switch"
        case "bar_composite_red", "composite_red":
            return "composite RED"
        case "bar_stop_me":
            return "stop-me"
        case "naked_window", "bar_protective_sl":
            return "protective SL"
        default:
            return raw.replacingOccurrences(of: "bar_", with: "").replacingOccurrences(of: "_", with: " ")
        }
    }
}

struct BarDeclarationSubmitBlockedBanner: View {
    let blocked: Bool
    let activeInterventions: [ActiveIntervention]
    var clearKillSwitchBusy: Bool = false
    var onClearKillSwitch: (() -> Void)? = nil

    private var showsKillSwitchClear: Bool {
        guard blocked, onClearKillSwitch != nil else { return false }
        let sorted = BarInterventionCardSpec.sortedInterventions(activeInterventions)
        guard let primary = sorted.first else { return false }
        switch primary.interventionType.lowercased() {
        case "bar_kill_switch", "kill_switch":
            return true
        default:
            return false
        }
    }

    var body: some View {
        if blocked {
            VStack(alignment: .leading, spacing: 6) {
                Text(BarDeclarationSubmitBlockedPresentation.message(
                    blocked: blocked,
                    activeInterventions: activeInterventions
                ))
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Accent.amber)

                if showsKillSwitchClear {
                    Button(clearKillSwitchBusy ? "Resuming…" : "Resume trading") {
                        onClearKillSwitch?()
                    }
                    .font(BarDS.bodyFont(11, weight: .semibold))
                    .foregroundColor(BarDS.Accent.teal)
                    .disabled(clearKillSwitchBusy)
                }
            }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(BarDS.Semantic.amberBg())
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(BarDS.Semantic.amberBorder(), lineWidth: BarDS.borderThin),
            )
        }
    }
}
