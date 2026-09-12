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
    public static let splitClocksSubtitle = "Two clocks · happened and now"

    public let screen: TodayScreenPresentation
    public let remainingAmount: Double?
    public let remainingText: String
    public let remainingBadge: String
    public let chartPoints: [TodayClosedChartPoint]
    public let floorLine: Double?
    public let happenedClosedCount: Int
    public let nowOpenCount: Int
    public let quoteCurrency: String?
    public let usedCaption: String
    public let floorUsedPercentText: String
    public let clockText: String
    public let dateHeadline: String
    public let chartReadout: String
    public let brokerSlug: String?
    public let daySpineSubtitle: String

    public init(
        screen: TodayScreenPresentation,
        remainingAmount: Double?,
        remainingText: String,
        remainingBadge: String,
        chartPoints: [TodayClosedChartPoint],
        floorLine: Double?,
        happenedClosedCount: Int,
        nowOpenCount: Int,
        quoteCurrency: String?,
        usedCaption: String,
        floorUsedPercentText: String,
        clockText: String,
        dateHeadline: String,
        chartReadout: String,
        brokerSlug: String?,
        daySpineSubtitle: String
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
        self.usedCaption = usedCaption
        self.floorUsedPercentText = floorUsedPercentText
        self.clockText = clockText
        self.dateHeadline = dateHeadline
        self.chartReadout = chartReadout
        self.brokerSlug = brokerSlug
        self.daySpineSubtitle = daySpineSubtitle
    }

    public static func build(
        payload: TodayAgentPayload?,
        agentHealthy: Bool,
        positions: [DeskPosition] = [],
        dailyFloor: Double? = nil,
        now: Date = Date(),
        showShallowImpact: Bool = false,
        showActiveMoney: Bool = true,
        calendar: Calendar = .current,
        demoLabeled: Bool = false
    ) -> TodayDeskPresentation {
        let screen = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: agentHealthy,
            now: now,
            positions: positions,
            showShallowImpact: showShallowImpact,
            showActiveMoney: showActiveMoney,
            calendar: calendar
        ).applyingDemoLabel(demoLabeled)
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
        let usedCaption: String
        let floorUsedPercentText: String
        if let quote, !quote.isEmpty, let used, let dailyFloor,
           let percent = DeskRulesPresentation.floorUsedWholePercent(floor: dailyFloor, used: used)
        {
            usedCaption = DeskMoneyFormatting.formatSigned(-used, quoteCurrency: quote)
            let floorText = DeskMoneyFormatting.formatWhole(dailyFloor, quoteCurrency: quote)
            floorUsedPercentText = "\(percent)% of \(floorText) floor used"
        } else {
            usedCaption = TodayScreenPresentation.emDash
            floorUsedPercentText = TodayScreenPresentation.emDash
        }
        var clockCalendar = calendar
        if payload?.brokerSlug == "kotak_neo" {
            clockCalendar.timeZone = TimeZone(identifier: "Asia/Kolkata") ?? clockCalendar.timeZone
        }
        let clockFormatter = DateFormatter()
        clockFormatter.calendar = clockCalendar
        clockFormatter.timeZone = clockCalendar.timeZone
        clockFormatter.locale = Locale(identifier: "en_GB")
        clockFormatter.dateFormat = "HH:mm"
        let clockText = clockFormatter.string(from: now)
        let dateFormatter = DateFormatter()
        dateFormatter.calendar = clockCalendar
        dateFormatter.timeZone = clockCalendar.timeZone
        dateFormatter.locale = Locale(identifier: "en_GB")
        dateFormatter.dateFormat = "EEEE, d MMMM"
        let dateHeadline = dateFormatter.string(from: now)
        let closed = screen.heroTiles.first { $0.id == "pnl" }?.value ?? TodayScreenPresentation.emDash
        let chartReadout =
            "\(clockText) · \(closed) closed · \(remainingText) to floor · open MTM not on this line"
        return TodayDeskPresentation(
            screen: screen,
            remainingAmount: remaining,
            remainingText: remainingText,
            remainingBadge: remainingBadgeText,
            chartPoints: chartPoints,
            floorLine: dailyFloor,
            happenedClosedCount: payload?.hero.tradesToday ?? payload?.trades.count ?? 0,
            nowOpenCount: positions.count,
            quoteCurrency: quote,
            usedCaption: usedCaption,
            floorUsedPercentText: floorUsedPercentText,
            clockText: clockText,
            dateHeadline: dateHeadline,
            chartReadout: chartReadout,
            brokerSlug: payload?.brokerSlug,
            daySpineSubtitle: "One local day · session open · \(clockText)"
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
