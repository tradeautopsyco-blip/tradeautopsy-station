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
        sessionPnL: Double,
        unrealizedTotal: Double,
        positions: [NotchPosition],
        isDegraded: Bool,
        formatINR: (Double) -> String
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

        let sessionText = formatSignedINR(sessionPnL, formatINR: formatINR)
        let unrealizedText = formatSignedINR(unrealizedTotal, formatINR: formatINR)
        let count = positions.count

        return SessionPulseStripPresentation(
            sessionPnLText: sessionText,
            sessionPnLStyle: pnlStyle(for: sessionPnL),
            unrealizedPnLText: unrealizedText,
            unrealizedPnLStyle: pnlStyle(for: unrealizedTotal),
            positionCountText: count == 0 ? nil : String(count),
            singlePositionSymbol: count == 1 ? positions[0].symbol : nil,
            showsDegradedIndicator: false
        )
    }

    public static func isDegraded(
        agentHealthWarning: AgentHealthWarning?,
        brokerSessionActive: Bool
    ) -> Bool {
        agentHealthWarning != nil || !brokerSessionActive
    }

    private static func formatSignedINR(_ value: Double, formatINR: (Double) -> String) -> String {
        if value >= 0 {
            return "+\(formatINR(value))"
        }
        return formatINR(value)
    }

    private static func pnlStyle(for value: Double) -> PnLStyle {
        if value > 0 { return .positive }
        if value < 0 { return .negative }
        return .neutral
    }
}
