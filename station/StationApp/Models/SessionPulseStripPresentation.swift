import Foundation

public struct SessionPulseStripPresentation: Equatable {
    public static let degradedPlaceholder = "—"

    public enum PnLStyle: Equatable {
        case positive
        case negative
        case neutral
    }

    /// Side-by-side desk honesty chip (COM=USD / Kotak=INR). Never blend into one number.
    public struct DualDeskChip: Equatable, Identifiable, Sendable {
        public var id: String { label }
        public let label: String
        public let isActive: Bool

        public init(label: String, isActive: Bool) {
            self.label = label
            self.isActive = isActive
        }
    }

    public let sessionPnLText: String
    public let sessionPnLStyle: PnLStyle
    public let unrealizedPnLText: String
    public let unrealizedPnLStyle: PnLStyle
    public let positionCountText: String?
    public let singlePositionSymbol: String?
    public let showsDegradedIndicator: Bool
    public let dualChips: [DualDeskChip]?

    public init(
        sessionPnLText: String,
        sessionPnLStyle: PnLStyle,
        unrealizedPnLText: String,
        unrealizedPnLStyle: PnLStyle,
        positionCountText: String?,
        singlePositionSymbol: String?,
        showsDegradedIndicator: Bool,
        dualChips: [DualDeskChip]? = nil
    ) {
        self.sessionPnLText = sessionPnLText
        self.sessionPnLStyle = sessionPnLStyle
        self.unrealizedPnLText = unrealizedPnLText
        self.unrealizedPnLStyle = unrealizedPnLStyle
        self.positionCountText = positionCountText
        self.singlePositionSymbol = singlePositionSymbol
        self.showsDegradedIndicator = showsDegradedIndicator
        self.dualChips = dualChips
    }

    public static func build(
        sessionPnLUsd: Double?,
        unrealizedTotal: Double,
        positions: [DeskPosition],
        isDegraded: Bool,
        formatUSD: (Double) -> String,
        dualChips: [DualDeskChip]? = nil,
        showActiveMoney: Bool = true
    ) -> SessionPulseStripPresentation {
        if isDegraded {
            return SessionPulseStripPresentation(
                sessionPnLText: degradedPlaceholder,
                sessionPnLStyle: .neutral,
                unrealizedPnLText: degradedPlaceholder,
                unrealizedPnLStyle: .neutral,
                positionCountText: degradedPlaceholder,
                singlePositionSymbol: nil,
                showsDegradedIndicator: true,
                dualChips: dualChips
            )
        }

        if !showActiveMoney {
            // Dual desk with no single active currency — chips only; never invent a blend.
            return SessionPulseStripPresentation(
                sessionPnLText: degradedPlaceholder,
                sessionPnLStyle: .neutral,
                unrealizedPnLText: degradedPlaceholder,
                unrealizedPnLStyle: .neutral,
                positionCountText: positions.isEmpty ? nil : String(positions.count),
                singlePositionSymbol: positions.count == 1 ? positions[0].symbol : nil,
                showsDegradedIndicator: false,
                dualChips: dualChips
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
            showsDegradedIndicator: false,
            dualChips: dualChips
        )
    }

    /// Build dual chips from DeskHonesty dualNoBlend profiles + optional active slug.
    public static func dualChips(
        profiles: [DeskConnectionProfile],
        activeSlug: String?
    ) -> [DualDeskChip] {
        let active = activeSlug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return profiles.map { profile in
            let name = BrokerCatalog.descriptor(for: profile.brokerSlug)?.displayName
                ?? profile.brokerSlug
            let isActive = active == profile.brokerSlug.lowercased()
            return DualDeskChip(
                label: "\(name) · \(profile.quoteCurrency)",
                isActive: isActive
            )
        }
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
