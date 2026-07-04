import Foundation
import Notch
import Testing
@testable import Station

@MainActor
struct SessionPulseStripTests {
    private func formatUSD(_ value: Double) -> String {
        TodayScreenPresentation.formatSignedUSD(value)
    }

    // T_healthy_data: viewModel has realizedPnL + positions → strip shows values
    @Test func healthyDataShowsLiveValues() {
        let positions = [
            NotchPosition(symbol: "RELIANCE", qty: 10, unrealizedPnL: 500, direction: "LONG"),
        ]
        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 1_250,
            unrealizedTotal: 500,
            positions: positions,
            isDegraded: false,
            formatUSD: formatUSD
        )

        #expect(presentation.sessionPnLText.contains("1,250") || presentation.sessionPnLText.contains("1250"))
        #expect(presentation.sessionPnLStyle == .positive)
        #expect(presentation.unrealizedPnLText.contains("500"))
        #expect(presentation.positionCountText == "1")
        #expect(presentation.singlePositionSymbol == "RELIANCE")
        #expect(presentation.showsDegradedIndicator == false)
    }

    // T_degraded_no_data: agentHealthWarning non-nil → strip shows — for all numeric fields
    @Test func degradedNoDataShowsEmDashForAllNumericFields() {
        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 9_999,
            unrealizedTotal: 4_000,
            positions: [NotchPosition(symbol: "TCS", qty: 5, unrealizedPnL: 100, direction: "LONG")],
            isDegraded: true,
            formatUSD: formatUSD
        )

        #expect(presentation.sessionPnLText == SessionPulseStripPresentation.degradedPlaceholder)
        #expect(presentation.unrealizedPnLText == SessionPulseStripPresentation.degradedPlaceholder)
        #expect(presentation.positionCountText == SessionPulseStripPresentation.degradedPlaceholder)
        #expect(presentation.singlePositionSymbol == nil)
        #expect(presentation.showsDegradedIndicator)
    }

    // T_degraded_stale: broker disconnected → amber indicator, — values (never stale numbers)
    @Test func brokerDisconnectedShowsDegradedPresentation() {
        let isDegraded = SessionPulseStripPresentation.isDegraded(
            agentHealthWarning: nil,
            brokerSessionActive: false
        )
        #expect(isDegraded)

        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 2_500,
            unrealizedTotal: 800,
            positions: [NotchPosition(symbol: "INFY", qty: 20, unrealizedPnL: 800, direction: "LONG")],
            isDegraded: isDegraded,
            formatUSD: formatUSD
        )

        #expect(presentation.sessionPnLText == SessionPulseStripPresentation.degradedPlaceholder)
        #expect(presentation.unrealizedPnLText == SessionPulseStripPresentation.degradedPlaceholder)
        #expect(presentation.positionCountText == SessionPulseStripPresentation.degradedPlaceholder)
        #expect(presentation.singlePositionSymbol == nil)
        #expect(presentation.showsDegradedIndicator)
    }

    // T_single_position_chip: exactly 1 open position → symbol chip visible
    @Test func singlePositionShowsSymbolChip() {
        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 0,
            unrealizedTotal: 100,
            positions: [NotchPosition(symbol: "HDFCBANK", qty: 1, unrealizedPnL: 100, direction: "LONG")],
            isDegraded: false,
            formatUSD: formatUSD
        )

        #expect(presentation.positionCountText == "1")
        #expect(presentation.singlePositionSymbol == "HDFCBANK")
    }

    // T_multi_position: >1 positions → count only, no symbol chip
    @Test func multiplePositionsShowCountOnly() {
        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: 0,
            unrealizedTotal: 300,
            positions: [
                NotchPosition(symbol: "A", qty: 1, unrealizedPnL: 100, direction: "LONG"),
                NotchPosition(symbol: "B", qty: 2, unrealizedPnL: 200, direction: "SHORT"),
            ],
            isDegraded: false,
            formatUSD: formatUSD
        )

        #expect(presentation.positionCountText == "2")
        #expect(presentation.singlePositionSymbol == nil)
    }

    // T_zero_positions: 0 positions → position field hidden or 0
    @Test func zeroPositionsHidesPositionField() {
        let presentation = SessionPulseStripPresentation.build(
            sessionPnLUsd: nil,
            unrealizedTotal: 0,
            positions: [],
            isDegraded: false,
            formatUSD: formatUSD
        )

        #expect(presentation.positionCountText == nil)
        #expect(presentation.singlePositionSymbol == nil)
        #expect(presentation.sessionPnLText == SessionPulseStripPresentation.degradedPlaceholder)
    }

    // T_pulse_tap: tap strip → coordinator.navigateTo(.liveTrade) called
    @Test func pulseStripTapNavigatesToLiveTradeAndActivatesWindow() {
        let windowController = FakeStationWindowController()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: windowController,
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeBarSurfacePhaseProvider()
        )

        coordinator.openLiveTradeFromPulseStrip()

        #expect(coordinator.activeRoute == .liveTrade)
        #expect(windowController.showAndActivateCallCount == 1)
    }
}
