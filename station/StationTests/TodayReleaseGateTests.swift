import Foundation
import Testing
@testable import Station

struct TodayReleaseGateTests {
    @Test @MainActor func pulseDegradedWhenTodaySyncUnavailable() {
        let degraded = SessionPulseStripPresentation.isDegraded(
            agentHealthWarning: nil,
            brokerSessionActive: true,
            todayDegraded: true
        )
        #expect(degraded)

        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 100,
            unrealizedTotal: 50,
            positions: [],
            isDegraded: degraded,
            formatUSD: TodayScreenPresentation.formatSignedUSD
        )
        #expect(presentation.sessionPnLText == SessionPulseStripPresentation.degradedPlaceholder)
    }

    @Test @MainActor func pulseMatchesTodayHeroWhenHealthy() {
        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 8.79,
            unrealizedTotal: 0,
            positions: [],
            isDegraded: false,
            formatUSD: TodayScreenPresentation.formatSignedUSD
        )
        #expect(presentation.sessionPnLText.contains("8.79"))
    }

    @Test @MainActor func pulseTextUpdatesAfterTodayPayloadRefresh() async {
        let client = FakeTodayAgentClient(
            payload: TodayAgentPayload(
                localDate: "2026-07-04",
                performanceBasisNotTax: true,
                degradedReason: nil,
                learningBaseline: false,
                hero: TodayHeroPayload(pnlTodayUsd: 5.0, tradesToday: 1, winRate: 1.0),
                topSignals: [],
                trades: [],
                openPositionCount: 0
            )
        )
        let viewModel = TodayViewModel(
            client: client,
            sessionModel: SessionModel(),
            agentHealthy: { true }
        )
        await viewModel.load()
        let before = SessionPulseStripPresentation.build(
            sessionPnLUsd: viewModel.sessionPnLUsd,
            unrealizedTotal: 0,
            positions: [],
            isDegraded: false,
            formatUSD: TodayScreenPresentation.formatSignedUSD
        )
        client.payload = TodayAgentPayload(
            localDate: "2026-07-04",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 42.0, tradesToday: 2, winRate: 0.5),
            topSignals: [],
            trades: [],
            openPositionCount: 0
        )
        await viewModel.load()
        let after = SessionPulseStripPresentation.build(
            sessionPnLUsd: viewModel.sessionPnLUsd,
            unrealizedTotal: 0,
            positions: [],
            isDegraded: false,
            formatUSD: TodayScreenPresentation.formatSignedUSD
        )
        #expect(before.sessionPnLText.contains("5"))
        #expect(after.sessionPnLText.contains("42"))
    }

    @Test @MainActor func breakerHiddenWhenAgentDown() {
        let session = SessionModel()
        session.killSwitchActive = true
        session.setKillSwitchStateReceivedAt(Date())
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(),
            sessionModel: session,
            agentHealthy: { false }
        )
        #expect(viewModel.showCircuitBreakerBanner == false)
    }

    @Test @MainActor func breakerHiddenWhenSyncUnavailableAndStale() async {
        let session = SessionModel()
        session.killSwitchActive = true
        session.setKillSwitchStateReceivedAt(Date().addingTimeInterval(-61))
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(
                payload: TodayAgentPayload(
                    localDate: "2026-07-04",
                    performanceBasisNotTax: true,
                    degradedReason: "sync_unavailable",
                    learningBaseline: true,
                    hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
                    topSignals: [],
                    trades: [],
                    openPositionCount: 0
                )
            ),
            sessionModel: session,
            agentHealthy: { true }
        )
        await viewModel.load()
        #expect(viewModel.showCircuitBreakerBanner == false)
    }
}
