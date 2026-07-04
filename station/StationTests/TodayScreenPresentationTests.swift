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
}
