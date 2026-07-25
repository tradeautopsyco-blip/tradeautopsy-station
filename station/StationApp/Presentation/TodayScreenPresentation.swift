import Foundation
import Notch

public enum TodayPalette {
    public static let profit = "#0ECB81"
    public static let loss = "#F6465D"
    public static let watch = "#FF7A6B"
    public static let neutral = "#EDEDED"
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

    public static func build(
        payload: TodayAgentPayload?,
        agentHealthy: Bool,
        now: Date = Date()
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
        if payload.trades.isEmpty && payload.hero.pnlTodayUsd == nil {
            return healthyEmpty(payload: payload, now: now)
        }
        return healthyActive(payload: payload, now: now)
    }

    private static func degraded(agentHealthy: Bool, reason: String) -> TodayScreenPresentation {
        let caption = agentHealthy
            ? (reason == "sync_stale" ? "Unavailable · sync stale" : "Unavailable · sync paused")
            : "Unavailable · agent down"
        return TodayScreenPresentation(
            state: agentHealthy ? .syncUnavailable : .agentDown,
            subtitle: subtitleForDate(Date()),
            showDegradedBanner: agentHealthy,
            degradedBannerText: agentHealthy
                ? "Broker sync paused — hero metrics and trades show placeholders until sync resumes."
                : nil,
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
            showSignalsUnavailableMessage: true
        )
    }

    private static func healthyEmpty(payload: TodayAgentPayload, now: Date) -> TodayScreenPresentation {
        TodayScreenPresentation(
            state: .healthyEmpty,
            subtitle: subtitleForDate(now),
            showDegradedBanner: false,
            degradedBannerText: nil,
            heroTiles: [
                heroTile(id: "pnl", label: "P&L today", value: emDash, caption: "No trades yet today", tone: .empty),
                heroTile(id: "trades", label: "Trades today", value: emDash, caption: "No closed round-trips", tone: .empty),
                heroTile(id: "wr", label: "Win rate", value: emDash, caption: "No closed round-trips", tone: .empty),
            ],
            signalsMeta: payload.learningBaseline ? "learning baseline" : "updated just now",
            signals: signalCards(from: payload.topSignals, learning: payload.learningBaseline),
            tradesMeta: "",
            trades: [],
            showEmptyTable: true,
            learningBaseline: payload.learningBaseline,
            showSignalsUnavailableMessage: false
        )
    }

    private static func healthyActive(payload: TodayAgentPayload, now: Date) -> TodayScreenPresentation {
        let flagged = payload.trades.filter { $0.flagSeverity == "firing" || $0.flagSeverity == "watch" }.count
        let quote = payload.deskQuoteCurrency ?? "USD"
        return TodayScreenPresentation(
            state: .healthyActive,
            subtitle: subtitleForDate(now),
            showDegradedBanner: false,
            degradedBannerText: nil,
            heroTiles: [
                heroTile(
                    id: "pnl",
                    label: "P&L today",
                    value: formatMoney(payload.hero.pnlTodayUsd, quoteCurrency: quote),
                    caption: "Net of fees · performance basis, not tax",
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
                    caption: winRateCaption(payload: payload),
                    tone: toneForWinRate(payload.hero.winRate)
                ),
            ],
            signalsMeta: payload.learningBaseline ? "learning baseline" : "updated just now",
            signals: signalCards(from: payload.topSignals, learning: payload.learningBaseline),
            tradesMeta: flagged > 0 ? "\(flagged) flagged" : "",
            trades: payload.trades.map { tradeRow($0, quoteCurrency: quote) },
            showEmptyTable: false,
            learningBaseline: payload.learningBaseline,
            showSignalsUnavailableMessage: false
        )
    }

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
            isFlagged: row.flagSeverity == "firing"
        )
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

    private static func subtitleForDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "EEEE · local day"
        return formatter.string(from: date)
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

    private static func winRateCaption(payload: TodayAgentPayload) -> String {
        let wins = payload.trades.filter { ($0.netPnlUsd ?? 0) > 0 }.count
        let losses = payload.trades.filter { ($0.netPnlUsd ?? 0) < 0 }.count
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
