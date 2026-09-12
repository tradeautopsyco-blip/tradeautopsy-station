import Foundation

/// Settings desk-rules presentation. Display-only: no loss-limits POST, no Kill.
public enum DeskRulesSettingsAction: Equatable, Sendable {
    case apiKeys
    case openNotch
}

public enum DeskRulesPresentation {
    /// Closed hero P&L → used today. Spec: max(0, −closed). Unhealthy or missing → nil.
    public static func usedTodayAmount(closedPnL: Double?, healthy: Bool) -> Double? {
        guard healthy, let closedPnL else { return nil }
        return max(0, -closedPnL)
    }

    /// Labeled preview caption. DualNoBlend: dash when quote is missing. Else "—" when used is missing.
    public static func usedTodayCaption(
        closedPnL: Double?,
        quoteCurrency: String?,
        healthy: Bool
    ) -> String {
        guard let quote = quoteCurrency, !quote.isEmpty else { return "—" }
        guard let used = usedTodayAmount(closedPnL: closedPnL, healthy: healthy) else {
            return "—"
        }
        let painted = DeskMoneyFormatting.formatSigned(-used, quoteCurrency: quote)
        return "Used today \(painted)"
    }

    /// Floor minus used when both exist. Labeled preview — not T8 remaining-risk.
    public static func remainingPreview(floor: Double?, used: Double?) -> Double? {
        guard let floor, let used else { return nil }
        return floor - used
    }

    /// Desk rules never encode `/api/daemon/bar/profile/loss-limits`.
    public static func lossLimitsRequest(
        dailyFloor: Double?,
        meanLoss: Double?,
        maxRoundTrips: Int?
    ) -> URLRequest? {
        _ = dailyFloor
        _ = meanLoss
        _ = maxRoundTrips
        return nil
    }

    public static func navigationDestination(for action: DeskRulesSettingsAction) -> StationRoute? {
        switch action {
        case .apiKeys:
            return .brokers
        case .openNotch:
            return nil
        }
    }
}
