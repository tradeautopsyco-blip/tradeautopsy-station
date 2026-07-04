import Foundation
import Notch

public struct SessionPulseStripPresentation: Equatable {
    public static let degradedPlaceholder = "—"

    public enum PnLStyle: Equatable {
        case positive
        case negative
        case neutral
    }

    public let sessionPnLText: String
    public let sessionPnLStyle: PnLStyle
    public let unrealizedPnLText: String
    public let unrealizedPnLStyle: PnLStyle
    public let positionCountText: String?
    public let singlePositionSymbol: String?
    public let showsDegradedIndicator: Bool

    public static func build(
        sessionPnLUsd: Double?,
        unrealizedTotal: Double,
        positions: [NotchPosition],
        isDegraded: Bool,
        formatUSD: (Double) -> String
    ) -> SessionPulseStripPresentation {
        if isDegraded {
            return SessionPulseStripPresentation(
                sessionPnLText: degradedPlaceholder,
                sessionPnLStyle: .neutral,
                unrealizedPnLText: degradedPlaceholder,
                unrealizedPnLStyle: .neutral,
                positionCountText: degradedPlaceholder,
                singlePositionSymbol: nil,
                showsDegradedIndicator: true
            )
        }

        let sessionText = formatSessionPnL(sessionPnLUsd, formatUSD: formatUSD)
        let unrealizedText = formatSignedUSD(unrealizedTotal, formatUSD: formatUSD)
        let count = positions.count

        return SessionPulseStripPresentation(
            sessionPnLText: sessionText,
            sessionPnLStyle: pnlStyle(for: sessionPnLUsd ?? 0),
            unrealizedPnLText: unrealizedText,
            unrealizedPnLStyle: pnlStyle(for: unrealizedTotal),
            positionCountText: count == 0 ? nil : String(count),
            singlePositionSymbol: count == 1 ? positions[0].symbol : nil,
            showsDegradedIndicator: false
        )
    }

    public static func isDegraded(
        agentHealthWarning: AgentHealthWarning?,
        brokerSessionActive: Bool,
        todayDegraded: Bool = false
    ) -> Bool {
        agentHealthWarning != nil || !brokerSessionActive || todayDegraded
    }

    private static func formatSessionPnL(_ value: Double?, formatUSD: (Double) -> String) -> String {
        guard let value else { return degradedPlaceholder }
        return formatSignedUSD(value, formatUSD: formatUSD)
    }

    private static func formatSignedUSD(_ value: Double, formatUSD: (Double) -> String) -> String {
        if value >= 0 {
            return "+\(formatUSD(value))"
        }
        return formatUSD(value)
    }

    private static func pnlStyle(for value: Double) -> PnLStyle {
        if value > 0 { return .positive }
        if value < 0 { return .negative }
        return .neutral
    }
}
