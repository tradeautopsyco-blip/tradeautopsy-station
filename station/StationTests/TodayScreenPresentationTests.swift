import Foundation
import Testing
@testable import Station

struct TodayScreenPresentationTests {
    @Test func healthyActiveShowsUsdHero() {
        let payload = TodayAgentPayload(
            localDate: "2026-07-04",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 42.5, tradesToday: 2, winRate: 0.5),
            topSignals: [],
            trades: [],
            openPositionCount: 0
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        #expect(presentation.state == .healthyActive)
        #expect(presentation.heroTiles.first?.value.contains("42") == true)
        #expect(presentation.showSignalsUnavailableMessage == false)
    }

    @Test func healthyEmptyUsesEmDash() {
        let payload = TodayAgentPayload(
            localDate: "2026-07-04",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: true,
            hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
            topSignals: [],
            trades: [],
            openPositionCount: 0
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        #expect(presentation.state == .healthyEmpty)
        #expect(presentation.heroTiles.first?.value == TodayScreenPresentation.emDash)
        #expect(presentation.showSignalsUnavailableMessage == false)
        #expect(presentation.signalsMeta == "learning baseline")
        #expect(!presentation.signals.isEmpty)
    }

    @Test func syncPausedDegradedMatrix() {
        let payload = TodayAgentPayload(
            localDate: "2026-07-04",
            performanceBasisNotTax: true,
            degradedReason: "sync_unavailable",
            learningBaseline: true,
            hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
            topSignals: [],
            trades: [],
            openPositionCount: 0
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        #expect(presentation.state == .syncUnavailable)
        #expect(presentation.showDegradedBanner)
        #expect(presentation.heroTiles.allSatisfy { $0.value == TodayScreenPresentation.emDash })
        #expect(presentation.showSignalsUnavailableMessage)
        #expect(presentation.signals.isEmpty)
        #expect(presentation.signalsMeta == "unavailable")
    }

    @Test func agentDownDegradedMatrix() {
        let presentation = TodayScreenPresentation.build(payload: nil, agentHealthy: false)
        #expect(presentation.state == .agentDown)
        #expect(presentation.heroTiles.allSatisfy { $0.value == TodayScreenPresentation.emDash })
        #expect(presentation.showSignalsUnavailableMessage)
        #expect(presentation.signals.isEmpty)
        #expect(presentation.signalsMeta == "unavailable")
    }

    @Test func degradedSignalsDifferFromHealthyLearningBaseline() {
        let degraded = TodayScreenPresentation.build(
            payload: TodayAgentPayload(
                localDate: "2026-07-04",
                performanceBasisNotTax: true,
                degradedReason: "sync_unavailable",
                learningBaseline: true,
                hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
                topSignals: [],
                trades: [],
                openPositionCount: 0
            ),
            agentHealthy: true
        )
        let learning = TodayScreenPresentation.build(
            payload: TodayAgentPayload(
                localDate: "2026-07-04",
                performanceBasisNotTax: true,
                degradedReason: nil,
                learningBaseline: true,
                hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
                topSignals: [],
                trades: [],
                openPositionCount: 0
            ),
            agentHealthy: true
        )
        #expect(degraded.showSignalsUnavailableMessage)
        #expect(!learning.showSignalsUnavailableMessage)
        #expect(degraded.signals.isEmpty)
        #expect(!learning.signals.isEmpty)
        #expect(degraded.signalsMeta == "unavailable")
        #expect(learning.signalsMeta == "learning baseline")
    }

    @Test func openPositionsWithoutClosedTradesAreNotHealthyEmpty() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: true,
            hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
            topSignals: [],
            trades: [],
            openPositionCount: 1,
            brokerSlug: "kotak_neo",
            quoteCurrency: "INR"
        )
        let positions = [
            DeskPosition(symbol: "RELIANCE", qty: 50, unrealizedPnL: 900, direction: "LONG"),
        ]
        let presentation = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions
        )
        #expect(presentation.state == .healthyActive)
        #expect(presentation.openRows.count == 1)
        #expect(presentation.openRows.first?.accountShareText == TodayScreenPresentation.emDash)
        #expect(presentation.openRows.first?.goalText == TodayScreenPresentation.emDash)
        #expect(presentation.showEmptyTable)
        #expect(!presentation.showEmptyOpenBook)
    }

    @Test func shallowImpactStaysEmDashUntilCapitalOwnerExists() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 12, tradesToday: 1, winRate: 1),
            topSignals: [],
            trades: [
                TodayTradeRowPayload(
                    closedAt: "2026-08-21T10:15:00.000Z",
                    symbol: "AAPL",
                    avgEntry: 100,
                    avgExit: 101,
                    qty: 1,
                    netPnlUsd: 12,
                    primaryFlag: "Clean",
                    flagSeverity: "clean",
                    dataQualityFlags: []
                ),
            ],
            openPositionCount: 0
        )
        let presentation = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: true,
            showShallowImpact: true
        )
        #expect(presentation.trades.first?.accountShareText == TodayScreenPresentation.emDash)
        #expect(presentation.trades.first?.goalText == TodayScreenPresentation.emDash)
        #expect(presentation.showShallowImpact)
    }

    @Test func stubAdapterIsDegradedNotQuietDay() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: "stub_adapter",
            learningBaseline: true,
            hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
            topSignals: [],
            trades: [],
            openPositionCount: 0
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        #expect(presentation.state == .syncUnavailable)
        #expect(presentation.showDegradedBanner)
        #expect(presentation.degradedBannerText?.contains("cannot produce fills") == true)
        #expect(presentation.heroTiles.allSatisfy { $0.value == TodayScreenPresentation.emDash })
        #expect(presentation.takeaway.contains("not a quiet day"))
    }

    @Test func dualDeskWithoutActiveCurrencyDashesMoney() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 99, tradesToday: 4, winRate: 0.5),
            topSignals: [],
            trades: [],
            openPositionCount: 0,
            brokerSlug: "binance_com",
            quoteCurrency: "USD"
        )
        let presentation = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: true,
            showActiveMoney: false
        )
        #expect(presentation.heroTiles.allSatisfy { $0.value == TodayScreenPresentation.emDash })
        #expect(presentation.takeaway.contains("no blended P&L"))
    }

    @Test func winRateCaptionUsesSharedHeroCounts() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(
                pnlTodayUsd: -10,
                tradesToday: 2,
                winRate: 0.5,
                winsToday: 1,
                lossesToday: 1
            ),
            topSignals: [],
            trades: [
                TodayTradeRowPayload(
                    closedAt: "2026-08-21T10:15:00.000Z",
                    symbol: "AAPL",
                    avgEntry: 100,
                    avgExit: 90,
                    qty: 1,
                    netPnlUsd: nil,
                    primaryFlag: "Unknown basis",
                    flagSeverity: "watch",
                    dataQualityFlags: ["unknown_basis"]
                ),
                TodayTradeRowPayload(
                    closedAt: "2026-08-21T11:15:00.000Z",
                    symbol: "MSFT",
                    avgEntry: 100,
                    avgExit: 110,
                    qty: 1,
                    netPnlUsd: nil,
                    primaryFlag: "Clean",
                    flagSeverity: "clean",
                    dataQualityFlags: []
                ),
            ],
            openPositionCount: 0
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        let wr = presentation.heroTiles.first { $0.id == "wr" }
        #expect(wr?.caption == "1 win · 1 loss")
    }

    @Test func openNowDashesUnknownMarkToMarket() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: true,
            hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
            topSignals: [],
            trades: [],
            openPositionCount: 1
        )
        let positions = [
            DeskPosition(symbol: "BTCUSDT", qty: 0.01, unrealizedPnL: nil, direction: "LONG"),
        ]
        let presentation = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions
        )
        #expect(presentation.openRows.first?.mtmText == TodayScreenPresentation.emDash)
        #expect(presentation.openRows.first?.qtyText == "0.01")
    }
}
