import Foundation

// MARK: - #36 — collapsed chip is account impact; intervention still wins

enum CollapsedNotchLayout: Equatable {
    case intervention(keyword: String, chromeBackgroundHex: String, chromeBorderHex: String)
    case impact(AccountImpact)
}

struct CollapsedNotchPresentation: Equatable {
    var layout: CollapsedNotchLayout
    /// Intervention chrome may still pulse; impact chip does not.
    var pillPulseAmber: Bool

    /// Intervention replaces the strip. Otherwise the closed chip is account impact (#36).
    static func build(
        notch: BarLiveStateResponse?,
        impact: AccountImpact,
    ) -> CollapsedNotchPresentation {
        if let notch, let primary = BarInterventionCardSpec.sortedInterventions(notch.activeInterventions).first {
            let kw = collapsedInterventionKeyword(interventionType: primary.interventionType)
            let chrome = BarInterventionCardSpec.chrome(interventionType: primary.interventionType, isPrimary: true)
            return CollapsedNotchPresentation(
                layout: .intervention(
                    keyword: kw,
                    chromeBackgroundHex: chrome.backgroundHex,
                    chromeBorderHex: chrome.borderColorHex,
                ),
                pillPulseAmber: false,
            )
        }
        return CollapsedNotchPresentation(layout: .impact(impact), pillPulseAmber: false)
    }

    static func collapsedInterventionKeyword(interventionType: String) -> String {
        let k = interventionType.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch k {
        case "token_expired", "bar_broker_session":
            return "EXPIRED"
        case "post_loss_macro", "exit_gate_micro":
            return "COOLING"
        default:
            return "LIMIT"
        }
    }
}
