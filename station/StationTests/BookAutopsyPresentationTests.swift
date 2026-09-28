import Foundation
import Testing
@testable import Station

struct BookAutopsyPresentationTests {
    @Test func activeDeskGetsTradeCountAndOtherBrokerGetsEmDash() {
        let screen = makeScreen(
            tradesValue: "4",
            trades: [trade(symbol: "RELIANCE", time: "09:18", flag: "Clean")]
        )
        let presentation = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: "kotak_neo",
            configuredBrokerSlugs: ["kotak_neo", "binance_com"]
        )
        #expect(presentation.cards.map(\.id) == ["kotak_neo", "binance_com"])
        #expect(presentation.cards[0].name == "Kotak Neo")
        #expect(presentation.cards[0].mark == "K")
        #expect(presentation.cards[0].value == "4")
        #expect(presentation.cards[0].caption == "today")
        #expect(presentation.cards[1].name == "Binance.com")
        #expect(presentation.cards[1].value == TodayScreenPresentation.emDash)
        #expect(presentation.cards[1].caption == "not this desk")
    }

    @Test func emptyTodayDrawsNoBars() {
        let presentation = BookAutopsyPresentation.build(
            screen: makeScreen(tradesValue: TodayScreenPresentation.emDash, trades: []),
            brokerSlug: "kotak_neo",
            configuredBrokerSlugs: ["kotak_neo"]
        )
        #expect(presentation.bars.isEmpty)
        #expect(presentation.chartCaption == "No closed trades today.")
        #expect(presentation.cards[0].value == TodayScreenPresentation.emDash)
    }

    @Test func barsBinClosedTradesByHour() {
        let presentation = BookAutopsyPresentation.build(
            screen: makeScreen(trades: [
                trade(symbol: "RELIANCE", time: "09:18", flag: "Clean"),
                trade(symbol: "INFY", time: "09:40", flag: "Clean"),
                trade(symbol: "TCS", time: "11:02", flag: "Revenge"),
                trade(symbol: "SBIN", time: "--:--", flag: "Clean"),
            ]),
            brokerSlug: "kotak_neo",
            configuredBrokerSlugs: ["kotak_neo"]
        )
        #expect(presentation.bars == [
            BookAutopsyBar(hour: 9, count: 2),
            BookAutopsyBar(hour: 11, count: 1),
        ])
        #expect(presentation.chartCaption == nil)
    }

    @Test func flagFilterDropsNonMatchingRows() {
        let screen = makeScreen(trades: [
            trade(symbol: "RELIANCE", time: "09:18", flag: "Clean"),
            trade(symbol: "INFY", time: "10:05", flag: "Revenge"),
            trade(symbol: "TCS", time: "10:20", flag: "revenge chase"),
        ])
        let presentation = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: "kotak_neo",
            configuredBrokerSlugs: ["kotak_neo"],
            search: .revenge
        )
        #expect(presentation.bars == [BookAutopsyBar(hour: 10, count: 2)])
        #expect(presentation.closedTradeCount == 3)
        #expect(presentation.favorites == ["RELIANCE", "INFY", "TCS"])
    }

    @Test func searchWithNoMatchesNamesTheFilter() {
        let presentation = BookAutopsyPresentation.build(
            screen: makeScreen(trades: [trade(symbol: "RELIANCE", time: "09:18", flag: "Clean")]),
            brokerSlug: nil,
            configuredBrokerSlugs: [],
            search: .fomo
        )
        #expect(presentation.bars.isEmpty)
        #expect(presentation.chartCaption == "No FOMO fills today.")
        #expect(presentation.cards.isEmpty)
    }

    @Test func peekFactsComeFromTheClosedRow() {
        let screen = makeScreen(
            trades: [trade(symbol: "RELIANCE", time: "09:18", flag: "Revenge", entry: "2,940", exit: "2,910", pnl: "−₹1,200", tone: .loss)],
            openRows: [openRow(symbol: "RELIANCE", qty: "50")]
        )
        let presentation = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: "kotak_neo",
            configuredBrokerSlugs: ["kotak_neo"]
        )
        let peek = presentation.peek(for: "RELIANCE", in: screen)
        #expect(peek?.symbol == "RELIANCE")
        #expect(peek?.subtitle == "09:18")
        #expect(peek?.valueText == "−₹1,200")
        #expect(peek?.tone == .loss)
        #expect(peek?.facts.map(\.label) == ["Flag", "Entry", "Exit", "Qty"])
        #expect(peek?.facts.map(\.value) == ["Revenge", "2,940", "2,910", "50"])
    }

    @Test func peekUsesOpenRowWhenNothingClosed() {
        let screen = makeScreen(openRows: [openRow(symbol: "NIFTY", qty: "1", mtm: "−₹640", tone: .loss)])
        let presentation = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: "kotak_neo",
            configuredBrokerSlugs: ["kotak_neo"]
        )
        #expect(presentation.favorites == ["NIFTY"])
        let peek = presentation.peek(for: "NIFTY", in: screen)
        #expect(peek?.facts.map(\.label) == ["Side", "Qty", "Book"])
        #expect(peek?.valueText == "−₹640")
        #expect(presentation.peek(for: "MISSING", in: screen) == nil)
    }

    @Test func firingSignalCountIgnoresWatch() {
        let screen = makeScreen(signals: [
            TodaySignalCardPresentation(id: "a", name: "Loss chasing", severity: "Firing", description: "", tone: .firing),
            TodaySignalCardPresentation(id: "b", name: "Overtrading", severity: "Watch", description: "", tone: .watch),
        ])
        let presentation = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: nil,
            configuredBrokerSlugs: []
        )
        #expect(presentation.firingSignalCount == 1)
    }

    private func makeScreen(
        tradesValue: String = "1",
        trades: [TodayTradeRowPresentation] = [],
        openRows: [TodayOpenRowPresentation] = [],
        signals: [TodaySignalCardPresentation] = []
    ) -> TodayScreenPresentation {
        TodayScreenPresentation(
            state: trades.isEmpty && openRows.isEmpty ? .healthyEmpty : .healthyActive,
            subtitle: "Monday",
            showDegradedBanner: false,
            degradedBannerText: nil,
            heroTiles: [
                TodayHeroTilePresentation(id: "pnl", label: "P&L today", value: "—", caption: "", tone: .empty),
                TodayHeroTilePresentation(id: "trades", label: "Trades today", value: tradesValue, caption: "", tone: .neutral),
                TodayHeroTilePresentation(id: "wr", label: "Win rate", value: "—", caption: "", tone: .empty),
            ],
            signalsMeta: "",
            signals: signals,
            tradesMeta: "",
            trades: trades,
            showEmptyTable: trades.isEmpty,
            learningBaseline: false,
            showSignalsUnavailableMessage: false,
            openRows: openRows,
            showEmptyOpenBook: openRows.isEmpty,
            showShallowImpact: false,
            shallowImpactCaption: "",
            takeaway: "",
            caption: ""
        )
    }

    private func trade(
        symbol: String,
        time: String,
        flag: String,
        entry: String = "1",
        exit: String = "1",
        pnl: String = "—",
        tone: TodayValueTone = .neutral
    ) -> TodayTradeRowPresentation {
        TodayTradeRowPresentation(
            id: "\(time)-\(symbol)",
            timeText: time,
            symbol: symbol,
            avgEntryText: entry,
            avgExitText: exit,
            pnlText: pnl,
            pnlTone: tone,
            flagText: flag,
            flagTone: .watch,
            isFlagged: flag != "Clean",
            accountShareText: "—",
            goalText: "—",
            sideText: "—",
            holdText: "—"
        )
    }

    private func openRow(
        symbol: String,
        qty: String,
        mtm: String = "—",
        tone: TodayValueTone = .neutral
    ) -> TodayOpenRowPresentation {
        TodayOpenRowPresentation(
            id: symbol,
            symbol: symbol,
            sideText: "LONG",
            qtyText: qty,
            mtmText: mtm,
            mtmTone: tone,
            accountShareText: "—",
            goalText: "—",
            behaviorText: "Still open"
        )
    }
}
