import Foundation

public struct TodayClosedChartPoint: Equatable, Sendable {
    public let closedAt: String
    public let cumulativeClosedPnL: Double

    public init(closedAt: String, cumulativeClosedPnL: Double) {
        self.closedAt = closedAt
        self.cumulativeClosedPnL = cumulativeClosedPnL
    }
}

/// Happened / now tiles, remaining preview, and this-day closed chart.
/// Same payload as `TodayScreenPresentation`. No second money owner.
public struct TodayDeskPresentation: Equatable, Sendable {
    public static let remainingBadgeText = "T8 preview · floor not live"

    public let screen: TodayScreenPresentation
    public let remainingAmount: Double?
    public let remainingText: String
    public let remainingBadge: String
    public let chartPoints: [TodayClosedChartPoint]
    public let floorLine: Double?
    public let happenedClosedCount: Int
    public let nowOpenCount: Int
    public let quoteCurrency: String?

    public init(
        screen: TodayScreenPresentation,
        remainingAmount: Double?,
        remainingText: String,
        remainingBadge: String,
        chartPoints: [TodayClosedChartPoint],
        floorLine: Double?,
        happenedClosedCount: Int,
        nowOpenCount: Int,
        quoteCurrency: String?
    ) {
        self.screen = screen
        self.remainingAmount = remainingAmount
        self.remainingText = remainingText
        self.remainingBadge = remainingBadge
        self.chartPoints = chartPoints
        self.floorLine = floorLine
        self.happenedClosedCount = happenedClosedCount
        self.nowOpenCount = nowOpenCount
        self.quoteCurrency = quoteCurrency
    }

    public static func build(
        payload: TodayAgentPayload?,
        agentHealthy: Bool,
        positions: [DeskPosition] = [],
        dailyFloor: Double? = nil,
        now: Date = Date(),
        showShallowImpact: Bool = false,
        showActiveMoney: Bool = true,
        calendar: Calendar = .current
    ) -> TodayDeskPresentation {
        let screen = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: agentHealthy,
            now: now,
            positions: positions,
            showShallowImpact: showShallowImpact,
            showActiveMoney: showActiveMoney,
            calendar: calendar
        )
        let quote = payload?.deskQuoteCurrency
        let hasQuote = quote.map { !$0.isEmpty } ?? false
        let moneyHealthy =
            agentHealthy
            && showActiveMoney
            && (screen.state == .healthyActive || screen.state == .healthyEmpty)
        let used = DeskRulesPresentation.usedTodayAmount(
            closedPnL: payload?.hero.pnlTodayUsd,
            healthy: moneyHealthy
        )
        let remaining: Double?
        if showActiveMoney, hasQuote {
            remaining = DeskRulesPresentation.remainingPreview(floor: dailyFloor, used: used)
        } else {
            remaining = nil
        }
        let remainingText: String
        if let remaining, let quote, !quote.isEmpty {
            remainingText = DeskMoneyFormatting.formatWhole(remaining, quoteCurrency: quote)
        } else {
            remainingText = TodayScreenPresentation.emDash
        }
        let chartPoints: [TodayClosedChartPoint]
        if showActiveMoney, let payload {
            chartPoints = Self.closedChartPoints(from: payload.trades)
        } else {
            chartPoints = []
        }
        return TodayDeskPresentation(
            screen: screen,
            remainingAmount: remaining,
            remainingText: remainingText,
            remainingBadge: remainingBadgeText,
            chartPoints: chartPoints,
            floorLine: dailyFloor,
            happenedClosedCount: payload?.hero.tradesToday ?? payload?.trades.count ?? 0,
            nowOpenCount: positions.count,
            quoteCurrency: quote
        )
    }

    /// Cumulative closed P&L in fill time. Nil net is skipped, not zero. Open MTM is not on this line.
    public static func closedChartPoints(from trades: [TodayTradeRowPayload]) -> [TodayClosedChartPoint] {
        let ordered = trades.sorted { $0.closedAt < $1.closedAt }
        var cumulative = 0.0
        var points: [TodayClosedChartPoint] = []
        for row in ordered {
            guard let net = row.netPnlUsd else { continue }
            cumulative += net
            points.append(TodayClosedChartPoint(closedAt: row.closedAt, cumulativeClosedPnL: cumulative))
        }
        return points
    }
}
