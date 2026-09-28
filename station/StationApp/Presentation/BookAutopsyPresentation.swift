import Foundation

public enum BookAutopsySearch: String, CaseIterable, Identifiable, Sendable {
    case revenge = "Revenge"
    case fomo = "FOMO"
    case openRisk = "Open risk"
    case expiry = "This expiry"

    public var id: String { rawValue }
}

public struct BookAutopsyBrokerCard: Equatable, Identifiable, Sendable {
    public let id: String
    public let name: String
    public let mark: String
    public let value: String
    public let caption: String

    public init(id: String, name: String, mark: String, value: String, caption: String) {
        self.id = id
        self.name = name
        self.mark = mark
        self.value = value
        self.caption = caption
    }
}

public struct BookAutopsyBar: Equatable, Identifiable, Sendable {
    public let hour: Int
    public let count: Int

    public var id: Int { hour }

    public init(hour: Int, count: Int) {
        self.hour = hour
        self.count = count
    }
}

public struct BookAutopsyFact: Equatable, Identifiable, Sendable {
    public let id: String
    public let label: String
    public let value: String

    public init(id: String, label: String, value: String) {
        self.id = id
        self.label = label
        self.value = value
    }
}

public struct BookAutopsyPeek: Equatable, Sendable {
    public let symbol: String
    public let subtitle: String
    public let facts: [BookAutopsyFact]
    public let valueText: String
    public let tone: TodayValueTone

    public init(
        symbol: String,
        subtitle: String,
        facts: [BookAutopsyFact],
        valueText: String,
        tone: TodayValueTone
    ) {
        self.symbol = symbol
        self.subtitle = subtitle
        self.facts = facts
        self.valueText = valueText
        self.tone = tone
    }
}

/// Book autopsy canvas. One owner: today's screen, the desk slug, and configured brokers.
/// Fills are not split per broker, so only the active desk gets `hero` trades-today.
public struct BookAutopsyPresentation: Equatable, Sendable {
    public let cards: [BookAutopsyBrokerCard]
    public let bars: [BookAutopsyBar]
    public let favorites: [String]
    public let firingSignalCount: Int
    public let closedTradeCount: Int
    public let search: BookAutopsySearch?
    public let chartCaption: String?

    public init(
        cards: [BookAutopsyBrokerCard],
        bars: [BookAutopsyBar],
        favorites: [String],
        firingSignalCount: Int,
        closedTradeCount: Int,
        search: BookAutopsySearch?,
        chartCaption: String?
    ) {
        self.cards = cards
        self.bars = bars
        self.favorites = favorites
        self.firingSignalCount = firingSignalCount
        self.closedTradeCount = closedTradeCount
        self.search = search
        self.chartCaption = chartCaption
    }

    public static func build(
        screen: TodayScreenPresentation,
        brokerSlug: String?,
        configuredBrokerSlugs: [String],
        search: BookAutopsySearch? = nil
    ) -> BookAutopsyPresentation {
        let tradesValue = screen.heroTiles.first { $0.id == "trades" }?.value ?? TodayScreenPresentation.emDash
        let active = brokerSlug.flatMap { $0.isEmpty ? nil : $0 }
        let cards = cardSlugs(active: active, configured: configuredBrokerSlugs).map { slug in
            let name = BrokerConnectServices.displayName(for: slug)
            let isActive = slug == active
            let mark = name.first.map { String($0).uppercased() } ?? "?"
            return BookAutopsyBrokerCard(
                id: slug,
                name: name,
                mark: mark,
                value: isActive ? tradesValue : TodayScreenPresentation.emDash,
                caption: isActive ? "today" : "not this desk"
            )
        }

        let filtered = filteredTrades(screen.trades, search: search)
        let bars = hourlyBars(filtered)
        let chartCaption: String?
        if bars.isEmpty {
            if let search {
                chartCaption = "No \(search.rawValue) fills today."
            } else if screen.trades.isEmpty {
                chartCaption = "No closed trades today."
            } else {
                chartCaption = nil
            }
        } else {
            chartCaption = nil
        }

        return BookAutopsyPresentation(
            cards: cards,
            bars: bars,
            favorites: favoriteSymbols(trades: screen.trades, openRows: screen.openRows),
            firingSignalCount: screen.signals.filter { $0.tone == .firing }.count,
            closedTradeCount: screen.trades.count,
            search: search,
            chartCaption: chartCaption
        )
    }

    public func peek(for symbol: String, in screen: TodayScreenPresentation) -> BookAutopsyPeek? {
        if let trade = screen.trades.first(where: { $0.symbol == symbol }) {
            var facts = [
                BookAutopsyFact(id: "flag", label: "Flag", value: trade.flagText),
                BookAutopsyFact(id: "entry", label: "Entry", value: trade.avgEntryText),
                BookAutopsyFact(id: "exit", label: "Exit", value: trade.avgExitText),
            ]
            if let open = screen.openRows.first(where: { $0.symbol == symbol }) {
                facts.append(BookAutopsyFact(id: "qty", label: "Qty", value: open.qtyText))
            }
            return BookAutopsyPeek(
                symbol: trade.symbol,
                subtitle: trade.timeText,
                facts: facts,
                valueText: trade.pnlText,
                tone: trade.pnlTone
            )
        }
        if let open = screen.openRows.first(where: { $0.symbol == symbol }) {
            return BookAutopsyPeek(
                symbol: open.symbol,
                subtitle: open.sideText,
                facts: [
                    BookAutopsyFact(id: "side", label: "Side", value: open.sideText),
                    BookAutopsyFact(id: "qty", label: "Qty", value: open.qtyText),
                    BookAutopsyFact(id: "behavior", label: "Book", value: open.behaviorText),
                ],
                valueText: open.mtmText,
                tone: open.mtmTone
            )
        }
        return nil
    }

    private static func cardSlugs(active: String?, configured: [String]) -> [String] {
        var ordered: [String] = []
        if let active {
            ordered.append(active)
        }
        for slug in configured where !ordered.contains(slug) {
            ordered.append(slug)
        }
        return Array(ordered.prefix(2))
    }

    private static func filteredTrades(
        _ trades: [TodayTradeRowPresentation],
        search: BookAutopsySearch?
    ) -> [TodayTradeRowPresentation] {
        guard let search else { return trades }
        return trades.filter { trade in
            trade.flagText.range(of: search.rawValue, options: [.caseInsensitive, .diacriticInsensitive]) != nil
        }
    }

    private static func hourlyBars(_ trades: [TodayTradeRowPresentation]) -> [BookAutopsyBar] {
        var counts: [Int: Int] = [:]
        for trade in trades {
            guard let hour = hour(from: trade.timeText) else { continue }
            counts[hour, default: 0] += 1
        }
        return counts.keys.sorted().map { BookAutopsyBar(hour: $0, count: counts[$0] ?? 0) }
    }

    private static func hour(from timeText: String) -> Int? {
        let head = timeText.split(separator: ":").first.map(String.init) ?? ""
        guard let hour = Int(head), (0..<24).contains(hour) else { return nil }
        return hour
    }

    private static func favoriteSymbols(
        trades: [TodayTradeRowPresentation],
        openRows: [TodayOpenRowPresentation]
    ) -> [String] {
        var seen = Set<String>()
        var symbols: [String] = []
        for trade in trades where seen.insert(trade.symbol).inserted {
            symbols.append(trade.symbol)
        }
        for row in openRows where seen.insert(row.symbol).inserted {
            symbols.append(row.symbol)
        }
        return symbols
    }
}
