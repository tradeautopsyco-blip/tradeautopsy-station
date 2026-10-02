import Foundation
import Testing
@testable import Station

@MainActor
struct TodayViewModelDemoTests {
    private func livePayload() -> TodayAgentPayload {
        TodayAgentPayload(
            localDate: "2026-09-12",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 1_000, tradesToday: 1, winRate: 1),
            topSignals: [],
            trades: [
                TodayTradeRowPayload(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    symbol: "BTCUSDT",
                    avgEntry: 100,
                    avgExit: 110,
                    qty: 1,
                    netPnlUsd: 1_000,
                    primaryFlag: "Clean",
                    flagSeverity: "clean",
                    dataQualityFlags: []
                ),
            ],
            openPositionCount: 1,
            brokerSlug: "binance_com",
            quoteCurrency: "USD"
        )
    }

    private func makeDemoStore(enabled: Bool) -> DemoDeskStore {
        let suite = "today.vm.demo.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = DemoDeskStore(defaults: defaults, storageKey: "tradeautopsy.demoDesk.enabled")
        store.setEnabled(enabled)
        return store
    }

    @Test func demoOnUsesFixtureNotLiveClosedPnL() async {
        let client = FakeTodayAgentClient(payload: livePayload())
        let session = SessionModel()
        session.positions = [
            DeskPosition(symbol: "BTCUSDT", qty: 0.1, unrealizedPnL: 50, direction: "LONG"),
        ]
        let viewModel = TodayViewModel(
            client: client,
            sessionModel: session,
            agentHealthy: { true },
            configuredSlugs: { ["kotak_neo"] },
            demoDeskStore: makeDemoStore(enabled: true)
        )
        await viewModel.load()
        let pnl = viewModel.presentation.heroTiles.first { $0.id == "pnl" }
        #expect(pnl?.value.contains("2,400") == true || pnl?.value.contains("2400") == true)
        #expect(pnl?.value.contains("1,000") != true)
        #expect(viewModel.desk.screen.openRows.first?.symbol == "RELIANCE")
        #expect(viewModel.presentation.subtitle.contains("Demo · not live"))
        #expect(viewModel.presentation.showDegradedBanner)
        #expect(viewModel.presentation.degradedBannerText?.contains("Demo · not live") == true)
    }

    @Test func demoOffStillLoadsLiveToday() async {
        let client = FakeTodayAgentClient(payload: livePayload())
        let viewModel = TodayViewModel(
            client: client,
            sessionModel: SessionModel(),
            agentHealthy: { true },
            configuredSlugs: { ["binance_com"] },
            demoDeskStore: makeDemoStore(enabled: false)
        )
        await viewModel.load()
        let pnl = viewModel.presentation.heroTiles.first { $0.id == "pnl" }
        #expect(pnl?.value.contains("1,000") == true || pnl?.value.contains("1000") == true)
        #expect(!viewModel.presentation.subtitle.contains("Demo · not live"))
    }

    @Test func demoDetectUsesFixtureJournalNotLivePositions() async {
        let client = FakeTodayAgentClient(payload: livePayload())
        let session = SessionModel()
        session.positions = [
            DeskPosition(symbol: "BTCUSDT", qty: 0.1, unrealizedPnL: nil, direction: "LONG"),
        ]
        let viewModel = TodayViewModel(
            client: client,
            sessionModel: session,
            agentHealthy: { true },
            configuredSlugs: { ["kotak_neo"] },
            demoDeskStore: makeDemoStore(enabled: true)
        )
        await viewModel.load()
        let journal = FakeJournalAgentClient(
            payload: JournalWeekPayload(
                timezone: "Asia/Kolkata",
                weekStart: "2026-09-06T18:30:00.000Z",
                weekEnd: "2026-09-13T18:30:00.000Z",
                items: [],
                days: []
            )
        )
        let journalVM = JournalViewModel(
            client: journal,
            sessionModel: session,
            agentHealthy: { true },
            demoDeskStore: makeDemoStore(enabled: true)
        )
        await journalVM.load()
        let input = viewModel.detectCardInput(declarations: journalVM.weekDeclarations)
        #expect(input?.planStop == 1_260)
        #expect(input?.entry == nil)
        #expect(journalVM.week.declarations.map(\.symbol) == ["RELIANCE", "RELIANCE"])
        #expect(journalVM.week.declarations.map(\.status) == ["matched", "pending"])
    }

    @Test func demoKillBannerStillReadsLiveSessionModel() async {
        let session = SessionModel()
        session.killSwitchActive = true
        session.setKillSwitchStateReceivedAt(Date())
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(payload: livePayload()),
            sessionModel: session,
            agentHealthy: { true },
            configuredSlugs: { ["kotak_neo"] },
            demoDeskStore: makeDemoStore(enabled: true)
        )
        await viewModel.load()
        #expect(viewModel.showCircuitBreakerBanner == true)
    }

    @Test func demoOnPaintsRelianceFixtureEvenWhenBothLiveBooksAreConfigured() async {
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(payload: livePayload()),
            sessionModel: SessionModel(),
            agentHealthy: { true },
            configuredSlugs: { ["binance_com", "kotak_neo"] },
            demoDeskStore: makeDemoStore(enabled: true)
        )
        await viewModel.load()
        let pnl = viewModel.presentation.heroTiles.first { $0.id == "pnl" }
        #expect(pnl?.value.contains("2,400") == true || pnl?.value.contains("2400") == true)
        #expect(viewModel.presentation.trades.map(\.symbol) == ["RELIANCE", "RELIANCE", "RELIANCE"])
        #expect(viewModel.desk.chartPoints.count == 3)
        #expect(viewModel.presentation.subtitle.contains("Demo · not live"))
        #expect(!viewModel.presentation.trades.isEmpty)
        #expect(viewModel.desk.quoteCurrency == "INR")
    }

    @Test func liveDualNoBlendStillDashesWhenDemoOffAndBothBooksHaveNoActiveDesk() async {
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(payload: livePayload()),
            sessionModel: SessionModel(),
            agentHealthy: { true },
            configuredSlugs: { ["binance_com", "kotak_neo"] },
            demoDeskStore: makeDemoStore(enabled: false)
        )
        await viewModel.load()
        #expect(viewModel.presentation.heroTiles.allSatisfy { $0.value == TodayScreenPresentation.emDash })
        #expect(viewModel.desk.chartPoints.isEmpty)
        #expect(viewModel.desk.remainingText == TodayScreenPresentation.emDash)
    }

    @Test func demoRemainingUsesSettingsFloorAndDoesNotInventTwelveThousand() async {
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(payload: livePayload()),
            sessionModel: SessionModel(),
            agentHealthy: { true },
            configuredSlugs: { ["kotak_neo"] },
            dailyFloor: { nil },
            demoDeskStore: makeDemoStore(enabled: true)
        )
        await viewModel.load()
        #expect(viewModel.desk.remainingText == TodayScreenPresentation.emDash)
        #expect(!viewModel.desk.remainingText.contains("12,000"))
        #expect(viewModel.desk.floorLine == nil)
    }
}
