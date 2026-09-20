import Foundation

// MARK: - Collapsed chip is logo + session P&L; intervention still wins

enum CollapsedNotchLayout: Equatable {
    case intervention(keyword: String, chromeBackgroundHex: String, chromeBorderHex: String)
    case impact(AccountImpact)
}

struct CollapsedNotchPresentation: Equatable {
    var layout: CollapsedNotchLayout
    /// Intervention chrome may still pulse; impact chip does not.
    var pillPulseAmber: Bool
    /// Right ear — Dynamic Island waveform slot. Desk-signed session P&L, not a second writer.
    var pnlText: String
    /// Focused-book session hours (COM 24/7 vs NSE). Collapsed uses the short label.
    var sessionClockText: String

    /// Intervention replaces the strip. Otherwise logo + P&L in the hardware-notch ears.
    static func build(
        notch: BarLiveStateResponse?,
        impact: AccountImpact,
        pnlText: String,
        sessionClockText: String = "",
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
                pnlText: pnlText,
                sessionClockText: sessionClockText,
            )
        }
        return CollapsedNotchPresentation(
            layout: .impact(impact),
            pillPulseAmber: false,
            pnlText: pnlText,
            sessionClockText: sessionClockText,
        )
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
