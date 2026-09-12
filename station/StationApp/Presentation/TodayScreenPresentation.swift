import Foundation

public enum TodayPalette {
    public static let profit = "#30D158"
    public static let loss = "#FF453A"
    public static let watch = "#FF9F0A"
    public static let neutral = "#FFFFFF"
}

public enum TodayPresentationState: Equatable, Sendable {
    case agentDown
    case syncUnavailable
    case healthyEmpty
    case healthyActive
}

public struct TodayHeroTilePresentation: Equatable, Identifiable, Sendable {
    public let id: String
    public let label: String
    public let value: String
    public let caption: String
    public let tone: TodayValueTone
}

public enum TodayValueTone: Equatable, Sendable {
    case profit
    case loss
    case neutral
    case empty
}

public struct TodaySignalCardPresentation: Equatable, Identifiable, Sendable {
    public let id: String
    public let name: String
    public let severity: String
    public let description: String
    public let tone: TodaySignalTone
}

public enum TodaySignalTone: Equatable, Sendable {
    case firing
    case watch
    case learning
    case unavailable
}

public struct TodayTradeRowPresentation: Equatable, Identifiable, Sendable {
    public let id: String
    public let timeText: String
    public let symbol: String
    public let avgEntryText: String
    public let avgExitText: String
    public let pnlText: String
    public let pnlTone: TodayValueTone
    public let flagText: String
    public let flagTone: TodaySignalTone
    public let isFlagged: Bool
    public let accountShareText: String
    public let goalText: String
    public let sideText: String
    public let holdText: String
}

public struct TodayOpenRowPresentation: Equatable, Identifiable, Sendable {
    public let id: String
    public let symbol: String
    public let sideText: String
    public let qtyText: String
    public let mtmText: String
    public let mtmTone: TodayValueTone
    public let accountShareText: String
    public let goalText: String
    public let behaviorText: String
}

public struct TodayScreenPresentation: Equatable, Sendable {
    public static let emDash = "—"

    public let state: TodayPresentationState
    public let subtitle: String
    public let showDegradedBanner: Bool
    public let degradedBannerText: String?
    public let heroTiles: [TodayHeroTilePresentation]
    public let signalsMeta: String
    public let signals: [TodaySignalCardPresentation]
    public let tradesMeta: String
    public let trades: [TodayTradeRowPresentation]
    public let showEmptyTable: Bool
    public let learningBaseline: Bool
    public let showSignalsUnavailableMessage: Bool
    public let openRows: [TodayOpenRowPresentation]
    public let showEmptyOpenBook: Bool
    public let showShallowImpact: Bool
    public let shallowImpactCaption: String
    public let takeaway: String
    public let caption: String

    public func applyingDemoLabel(_ on: Bool) -> TodayScreenPresentation {
        guard on else { return self }
        let labeledSubtitle = subtitle.contains("Demo · not live")
            ? subtitle
            : "Demo · not live · \(subtitle)"
        let labeledBanner: String
        if let degradedBannerText, !degradedBannerText.contains("Demo · not live") {
            labeledBanner = "Demo · not live. \(degradedBannerText)"
        } else {
            labeledBanner = degradedBannerText ?? "Demo · not live"
        }
        return TodayScreenPresentation(
            state: state,
            subtitle: labeledSubtitle,
            showDegradedBanner: true,
            degradedBannerText: labeledBanner,
            heroTiles: heroTiles,
            signalsMeta: signalsMeta,
            signals: signals,
            tradesMeta: tradesMeta,
            trades: trades,
            showEmptyTable: showEmptyTable,
            learningBaseline: learningBaseline,
            showSignalsUnavailableMessage: showSignalsUnavailableMessage,
            openRows: openRows,
            showEmptyOpenBook: showEmptyOpenBook,
            showShallowImpact: showShallowImpact,
            shallowImpactCaption: shallowImpactCaption,
            takeaway: takeaway,
            caption: caption
        )
    }

    public static func build(
        payload: TodayAgentPayload?,
        agentHealthy: Bool,
        now: Date = Date(),
        positions: [DeskPosition] = [],
        showShallowImpact: Bool = false,
        showActiveMoney: Bool = true,
        calendar: Calendar = .current
    ) -> TodayScreenPresentation {
        if !agentHealthy || payload == nil {
            return degraded(agentHealthy: false, reason: "unavailable")
        }
        guard let payload else {
            return degraded(agentHealthy: true, reason: "unavailable")
        }
        if payload.degradedReason == "sync_unavailable" || payload.degradedReason == "sync_stale" {
            return degraded(agentHealthy: true, reason: payload.degradedReason ?? "sync_unavailable")
        }
        if payload.degradedReason == "stub_adapter" {
            return degraded(agentHealthy: true, reason: "stub_adapter")
        }
        if !showActiveMoney {
            return dualDeskNoBlend(payload: payload, now: now)
        }
        let hasClosed = !payload.trades.isEmpty || payload.hero.pnlTodayUsd != nil
        if !hasClosed && positions.isEmpty {
            return healthyEmpty(payload: payload, now: now, showShallowImpact: showShallowImpact)
        }
        return healthyActive(
            payload: payload,
            now: now,
            positions: positions,
            showShallowImpact: showShallowImpact,
            calendar: calendar
        )
    }

    private static func degraded(agentHealthy: Bool, reason: String) -> TodayScreenPresentation {
        let caption: String
        let banner: String?
        let takeaway: String
        switch reason {
        case "stub_adapter":
            caption = "Unavailable · stub adapter"
            banner = "This connection cannot produce fills — not a quiet trading day."
            takeaway = "Stub-empty is not a quiet day. Hero stays dash, not zero."
        case "sync_stale":
            caption = "Unavailable · sync stale"
            banner = "Broker sync stale — P&L, trades, and signals are placeholders. Not zero."
            takeaway = "Desk numbers are unavailable while broker sync is stale."
        case "sync_unavailable":
            caption = "Unavailable · sync paused"
            banner = "Broker sync paused — P&L, trades, and signals are placeholders. Not zero."
            takeaway = "Desk numbers are unavailable while broker sync is paused."
        default:
            caption = agentHealthy ? "Unavailable · sync paused" : "Unavailable · agent down"
            banner = agentHealthy
                ? "Broker sync paused — hero metrics and trades show placeholders until sync resumes."
                : nil
            takeaway = agentHealthy
                ? "Desk numbers are unavailable while broker sync is paused."
                : "Agent is down. Today stays dash, not zero."
        }
        return TodayScreenPresentation(
            state: agentHealthy ? .syncUnavailable : .agentDown,
            subtitle: subtitleForDate(Date()),
            showDegradedBanner: agentHealthy,
            degradedBannerText: banner,
            heroTiles: [
                heroTile(id: "pnl", label: "P&L today", value: emDash, caption: caption, tone: .empty),
                heroTile(id: "trades", label: "Trades today", value: emDash, caption: caption, tone: .empty),
                heroTile(id: "wr", label: "Win rate", value: emDash, caption: caption, tone: .empty),
            ],
            signalsMeta: "unavailable",
            signals: [],
            tradesMeta: "",
            trades: [],
            showEmptyTable: true,
            learningBaseline: true,
            showSignalsUnavailableMessage: true,
            openRows: [],
            showEmptyOpenBook: true,
            showShallowImpact: false,
            shallowImpactCaption: "",
            takeaway: takeaway,
            caption: "Degraded contract: null, not zero. One owner per number."
        )
    }

    private static func dualDeskNoBlend(
        payload: TodayAgentPayload,
        now: Date
    ) -> TodayScreenPresentation {
        TodayScreenPresentation(
            state: .healthyEmpty,
            subtitle: subtitleForDate(now, payload: payload, dualDesk: true),
            showDegradedBanner: true,
            degradedBannerText: "No active desk — COM is USD, Kotak is INR. Pulse shows both chips. Hero does not add them.",
            heroTiles: [
                heroTile(id: "pnl", label: "P&L today", value: emDash, caption: "No single desk currency", tone: .empty),
                heroTile(id: "trades", label: "Trades today", value: emDash, caption: "Choose COM or Kotak", tone: .empty),
                heroTile(id: "wr", label: "Win rate", value: emDash, caption: "Not a blended rate", tone: .empty),
            ],
            signalsMeta: "unavailable until a desk is active",
            signals: signalCards(from: payload.topSignals, learning: true),
            tradesMeta: "",
            trades: [],
            showEmptyTable: true,
            learningBaseline: true,
            showSignalsUnavailableMessage: false,
            openRows: [],
            showEmptyOpenBook: true,
            showShallowImpact: false,
            shallowImpactCaption: "",
            takeaway: "Two desks are live. There is no blended P&L. Pick a desk or keep money as dash.",
            caption: "T2 honesty. Market cap, weekly gauges, and remaining risk stay off this screen."
        )
    }

    private static func healthyEmpty(
        payload: TodayAgentPayload,
        now: Date,
        showShallowImpact: Bool
    ) -> TodayScreenPresentation {
        TodayScreenPresentation(
            state: .healthyEmpty,
            subtitle: subtitleForDate(now, payload: payload),
            showDegradedBanner: false,
            degradedBannerText: nil,
            heroTiles: [
                heroTile(id: "pnl", label: "P&L today", value: emDash, caption: "No trades yet today", tone: .empty),
                heroTile(id: "trades", label: "Trades today", value: emDash, caption: "No closed round-trips", tone: .empty),
                heroTile(id: "wr", label: "Win rate", value: emDash, caption: "No closed round-trips", tone: .empty),
            ],
            signalsMeta: payload.learningBaseline ? "learning baseline" : "local · top 2 of 4",
            signals: signalCards(from: payload.topSignals, learning: payload.learningBaseline),
            tradesMeta: "",
            trades: [],
            showEmptyTable: true,
            learningBaseline: payload.learningBaseline,
            showSignalsUnavailableMessage: false,
            openRows: [],
            showEmptyOpenBook: true,
            showShallowImpact: showShallowImpact,
            shallowImpactCaption: shallowCaption,
            takeaway: "No closed round-trips yet today. Hero stays dash, not zero.",
            caption: "Performance basis, not tax. Local calendar day. One owner per number."
        )
    }

    private static func healthyActive(
        payload: TodayAgentPayload,
        now: Date,
        positions: [DeskPosition],
        showShallowImpact: Bool,
        calendar: Calendar
    ) -> TodayScreenPresentation {
        let flagged = payload.trades.filter { $0.flagSeverity == "firing" || $0.flagSeverity == "watch" }.count
        let quote = payload.deskQuoteCurrency ?? "USD"
        let closedEmpty = payload.trades.isEmpty && payload.hero.pnlTodayUsd == nil
        let wins = payload.hero.winsToday
        let losses = payload.hero.lossesToday
        return TodayScreenPresentation(
            state: .healthyActive,
            subtitle: subtitleForDate(now, payload: payload),
            showDegradedBanner: false,
            degradedBannerText: nil,
            heroTiles: [
                heroTile(
                    id: "pnl",
                    label: "P&L today",
                    value: formatMoney(payload.hero.pnlTodayUsd, quoteCurrency: quote),
                    caption: closedEmpty
                        ? "No closed round-trips"
                        : "Net of fees · performance basis, not tax",
                    tone: toneForPnL(payload.hero.pnlTodayUsd)
                ),
                heroTile(
                    id: "trades",
                    label: "Trades today",
                    value: payload.hero.tradesToday.map(String.init) ?? emDash,
                    caption: "Closed round-trips only",
                    tone: .neutral
                ),
                heroTile(
                    id: "wr",
                    label: "Win rate",
                    value: formatWinRate(payload.hero.winRate),
                    caption: closedEmpty ? "No closed round-trips" : winRateCaption(wins: wins, losses: losses),
                    tone: toneForWinRate(payload.hero.winRate)
                ),
            ],
            signalsMeta: payload.learningBaseline ? "learning baseline" : "local · top 2 of 4",
            signals: signalCards(from: payload.topSignals, learning: payload.learningBaseline),
            tradesMeta: flagged > 0 ? "\(payload.trades.count) closed · \(flagged) flagged" : "\(payload.trades.count) closed · newest first",
            trades: payload.trades.map { tradeRow($0, quoteCurrency: quote) },
            showEmptyTable: payload.trades.isEmpty,
            learningBaseline: payload.learningBaseline,
            showSignalsUnavailableMessage: false,
            openRows: positions.map {
                openRow($0, quoteCurrency: quote, localDate: payload.localDate, calendar: calendar)
            },
            showEmptyOpenBook: positions.isEmpty,
            showShallowImpact: showShallowImpact,
            shallowImpactCaption: shallowCaption,
            takeaway: takeawayForActive(payload: payload, quote: quote, wins: wins, losses: losses),
            caption: "Performance basis, not tax. Local calendar day. One owner per number."
        )
    }

    private static let shallowCaption =
        "Account % and toward-goal stay — until a capital / goal owner exists. Remaining risk is not this screen."

    private static func heroTile(
        id: String,
        label: String,
        value: String,
        caption: String,
        tone: TodayValueTone
    ) -> TodayHeroTilePresentation {
        TodayHeroTilePresentation(id: id, label: label, value: value, caption: caption, tone: tone)
    }

    private static func signalCards(
        from signals: [TodaySignalPayload],
        learning: Bool
    ) -> [TodaySignalCardPresentation] {
        let source = signals.isEmpty ? defaultLearningSignals() : signals
        return source.prefix(2).enumerated().map { index, signal in
            TodaySignalCardPresentation(
                id: "\(signal.kind)-\(index)",
                name: signal.name,
                severity: severityLabel(signal.severity, learning: learning),
                description: signal.description,
                tone: signalTone(signal.severity, learning: learning)
            )
        }
    }

    private static func defaultLearningSignals() -> [TodaySignalPayload] {
        [
            TodaySignalPayload(
                kind: "loss_chasing",
                severity: "watch",
                name: "Loss chasing",
                description: "Still learning your baseline — signals activate after ~10 round-trips."
            ),
            TodaySignalPayload(
                kind: "overtrading",
                severity: "watch",
                name: "Overtrading",
                description: "Still learning your baseline — no fire until enough history exists."
            ),
        ]
    }

    private static func tradeRow(
        _ row: TodayTradeRowPayload,
        quoteCurrency: String
    ) -> TodayTradeRowPresentation {
        let tone = toneForPnL(row.netPnlUsd)
        let flagTone: TodaySignalTone = switch row.flagSeverity {
        case "firing": .firing
        case "watch": .watch
        default: row.primaryFlag == "Clean" ? .learning : .watch
        }
        return TodayTradeRowPresentation(
            id: row.closedAt + row.symbol,
            timeText: formatTime(row.closedAt),
            symbol: formatSymbol(row.symbol),
            avgEntryText: formatPrice(row.avgEntry),
            avgExitText: formatPrice(row.avgExit),
            pnlText: row.netPnlUsd.map { formatSignedMoney($0, quoteCurrency: quoteCurrency) } ?? emDash,
            pnlTone: tone,
            flagText: row.primaryFlag,
            flagTone: flagTone,
            isFlagged: row.flagSeverity == "firing",
            accountShareText: emDash,
            goalText: emDash,
            sideText: emDash,
            holdText: emDash
        )
    }

    private static func openRow(
        _ position: DeskPosition,
        quoteCurrency: String,
        localDate: String,
        calendar: Calendar
    ) -> TodayOpenRowPresentation {
        return TodayOpenRowPresentation(
            id: position.id,
            symbol: formatSymbol(position.symbol),
            sideText: position.direction.isEmpty ? "—" : position.direction.uppercased(),
            qtyText: formatQty(position.qty),
            mtmText: position.unrealizedPnL.map { formatSignedMoney($0, quoteCurrency: quoteCurrency) } ?? emDash,
            mtmTone: toneForPnL(position.unrealizedPnL),
            accountShareText: emDash,
            goalText: emDash,
            behaviorText: overnightBehavior(firstFilledAt: position.firstFilledAt, localDate: localDate, calendar: calendar)
        )
    }

    /// Overnight only when the leftover lot's first fill local date is before today. No date → Still open.
    public static func overnightBehavior(
        firstFilledAt: Date?,
        localDate: String,
        calendar: Calendar
    ) -> String {
        guard let firstFilledAt else { return "Still open" }
        let fillDay = localDayString(firstFilledAt, calendar: calendar)
        if fillDay < localDate {
            return "Overnight"
        }
        return "Still open"
    }

    private static func localDayString(_ date: Date, calendar: Calendar) -> String {
        let parts = calendar.dateComponents([.year, .month, .day], from: date)
        guard let year = parts.year, let month = parts.month, let day = parts.day else {
            return ""
        }
        return String(format: "%04d-%02d-%02d", year, month, day)
    }

    public static func formatUSD(_ value: Double?) -> String {
        formatMoney(value, quoteCurrency: "USD")
    }

    public static func formatMoney(_ value: Double?, quoteCurrency: String) -> String {
        guard let value else { return emDash }
        return formatSignedMoney(value, quoteCurrency: quoteCurrency)
    }

    public static func formatSignedUSD(_ value: Double) -> String {
        formatSignedMoney(value, quoteCurrency: "USD")
    }

    public static func formatSignedMoney(_ value: Double, quoteCurrency: String) -> String {
        DeskMoneyFormatting.formatSigned(value, quoteCurrency: quoteCurrency)
    }

    private static func formatWinRate(_ value: Double?) -> String {
        guard let value else { return emDash }
        return "\(Int((value * 100).rounded()))%"
    }

    private static func formatPrice(_ value: Double) -> String {
        if value >= 1000 {
            return String(format: "%.0f", value)
        }
        return String(format: "%.2f", value)
    }

    private static func formatSymbol(_ raw: String) -> String {
        if raw.hasSuffix("USDT") {
            let base = String(raw.dropLast(4))
            return "\(base)/USDT"
        }
        return raw
    }

    private static func formatTime(_ iso: String) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = formatter.date(from: iso) ?? ISO8601DateFormatter().date(from: iso) {
            let out = DateFormatter()
            out.dateFormat = "HH:mm"
            return out.string(from: date)
        }
        return "--:--"
    }

    private static func subtitleForDate(
        _ date: Date,
        payload: TodayAgentPayload? = nil,
        dualDesk: Bool = false
    ) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "EEEE · local day"
        var parts = [formatter.string(from: date)]
        if dualDesk {
            parts.append("two desks configured")
        } else if let slug = payload?.brokerSlug, !slug.isEmpty {
            let desk = slug == "kotak_neo" ? "Kotak" : (slug == "binance_com" ? "COM" : slug)
            parts.append(desk)
            if let ccy = payload?.deskQuoteCurrency {
                parts.append(ccy)
            }
        }
        return parts.joined(separator: " · ")
    }

    private static func formatQty(_ qty: Double) -> String {
        if qty == qty.rounded() {
            return String(Int(qty))
        }
        return String(format: "%g", qty)
    }

    private static func takeawayForActive(
        payload: TodayAgentPayload,
        quote: String,
        wins: Int?,
        losses: Int?
    ) -> String {
        guard let pnl = payload.hero.pnlTodayUsd else {
            return "Closed round-trips are on the table. Hero stays dash until eligible P&L exists."
        }
        let money = formatSignedMoney(pnl, quoteCurrency: quote)
        let wl: String
        if let wins, let losses {
            wl = " \(wins) win, \(losses) loss."
        } else {
            wl = ""
        }
        if pnl < 0 {
            return "Closed round-trips today lost \(money) net of fees.\(wl)"
        }
        if pnl > 0 {
            return "Closed round-trips today made \(money) net of fees.\(wl)"
        }
        return "Closed round-trips today netted even.\(wl)"
    }

    private static func toneForPnL(_ value: Double?) -> TodayValueTone {
        guard let value else { return .empty }
        if value > 0 { return .profit }
        if value < 0 { return .loss }
        return .neutral
    }

    private static func toneForWinRate(_ value: Double?) -> TodayValueTone {
        guard let value else { return .empty }
        if value >= 0.5 { return .profit }
        if value > 0 { return .loss }
        return .neutral
    }

    private static func winRateCaption(wins: Int?, losses: Int?) -> String {
        guard let wins, let losses else { return "Closed round-trips only" }
        return "\(wins) win · \(losses) loss"
    }

    private static func severityLabel(_ severity: String, learning: Bool) -> String {
        if learning { return "Learning" }
        switch severity {
        case "firing": return "Firing"
        case "watch": return "Watch"
        default: return "Clean"
        }
    }

    private static func signalTone(_ severity: String, learning: Bool) -> TodaySignalTone {
        if learning { return .learning }
        switch severity {
        case "firing": return .firing
        case "watch": return .watch
        default: return .learning
        }
    }
}
