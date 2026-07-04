import Foundation
import Notch
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

    @Test @MainActor func breakerHiddenWhenAgentDown() {
        let notchVM = NotchViewModel()
        notchVM.killSwitchActive = true
        notchVM.setKillSwitchStateReceivedAt(Date())
        let viewModel = TodayViewModel(
            client: FakeTodayAgentClient(),
            notchViewModel: notchVM,
            agentHealthy: { false }
        )
        #expect(viewModel.showCircuitBreakerBanner == false)
    }

    @Test @MainActor func breakerHiddenWhenSyncUnavailableAndStale() async {
        let notchVM = NotchViewModel()
        notchVM.killSwitchActive = true
        notchVM.setKillSwitchStateReceivedAt(Date().addingTimeInterval(-61))
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
            notchViewModel: notchVM,
            agentHealthy: { true }
        )
        await viewModel.load()
        #expect(viewModel.showCircuitBreakerBanner == false)
    }
}
