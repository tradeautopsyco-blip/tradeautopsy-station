import Foundation
import Testing
@testable import Station

struct TodayLayoutModeTests {
    @Test @MainActor func daySpineAndSplitClocksRoundTripOnInjectedDefaults() {
        let suite = "today.layout.mode.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = TodayLayoutModeStore(defaults: defaults, storageKey: "layout")
        #expect(store.mode == .daySpine)
        #expect(TodayLayoutMode(rawValue: "b") == nil)
        store.setMode(.splitClocks)
        #expect(store.mode == .splitClocks)
        let reloaded = TodayLayoutModeStore(defaults: defaults, storageKey: "layout")
        #expect(reloaded.mode == .splitClocks)
        store.setMode(.daySpine)
        #expect(store.mode == .daySpine)
        #expect(TodayLayoutMode.allCases.count == 2)
        defaults.removePersistentDomain(forName: suite)
    }

    @Test func bothLayoutsReadTheSameDeskPresentation() {
        let payload = TodayAgentPayload(
            localDate: "2026-09-12",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: -5_800, tradesToday: 1, winRate: 0),
            topSignals: [],
            trades: [
                TodayTradeRowPayload(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    symbol: "RELIANCE",
                    avgEntry: 1284,
                    avgExit: 1260,
                    qty: 50,
                    netPnlUsd: -5_800,
                    primaryFlag: "Clean",
                    flagSeverity: "clean",
                    dataQualityFlags: []
                ),
            ],
            openPositionCount: 0,
            brokerSlug: "kotak_neo",
            quoteCurrency: "INR"
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000
        )
        #expect(TodayLayoutMode.daySpine != TodayLayoutMode.splitClocks)
        #expect(desk.remainingAmount == 6_200)
        #expect(desk.chartPoints.map(\.cumulativeClosedPnL) == [-5_800])
        #expect(desk.screen.heroTiles.first?.value.contains("5,800") == true
            || desk.screen.heroTiles.first?.value.contains("5800") == true)
    }
}
