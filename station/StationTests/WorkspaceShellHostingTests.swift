import AppKit
import Foundation
import SwiftUI
import Testing
@testable import Station

/// Renders the shell offscreen and drives the same routes the rail buttons call.
/// SwiftUI rail buttons are not AppKit controls, so this does not synthesize a click.
@MainActor
struct WorkspaceShellHostingTests {
    @Test func shellRendersAndReportRoutesStayReachable() async throws {
        let suite = "workspace.shell.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        defer { defaults.removePersistentDomain(forName: suite) }
        let demo = DemoDeskStore(defaults: defaults, storageKey: "tradeautopsy.demoDesk.enabled")
        let deskRoutes = FakeDeskRouteStore()
        let agent = FakeAgentSupervisor()
        let coordinator = StationAppCoordinator(
            agentSupervisor: agent,
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            loginItemService: FakeLoginItemService(),
            deskRouteStore: deskRoutes,
            brokerControl: FakeBrokerControlClient(),
            todayClient: FakeTodayAgentClient(payload: payload()),
            demoDeskStore: demo,
            floatingNotch: FakeFloatingNotchHost()
        )
        await agent.start()
        await coordinator.todayViewModel.load()

        let host = NSHostingView(rootView: StationShellView(coordinator: coordinator))
        host.frame = NSRect(x: 0, y: 0, width: 1100, height: 720)
        let window = NSWindow(
            contentRect: NSRect(x: -4000, y: -4000, width: 1100, height: 720),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        window.contentView = host
        window.orderBack(nil)
        defer { window.close() }
        host.layoutSubtreeIfNeeded()
        pump(0.4)

        #expect(host.subviews.count > 3)

        coordinator.navigateTo(.report)
        #expect(coordinator.activeRoute == .report)
        #expect(deskRoutes.savedDeskRoute == .report)
        pump(0.2)

        let screen = coordinator.todayViewModel.presentation
        let open = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: coordinator.todayViewModel.desk.brokerSlug,
            configuredBrokerSlugs: coordinator.brokersViewModel.configuredBrokerSlugs
        )
        #expect(open.favorites.contains("RELIANCE"))
        #expect(open.peek(for: "RELIANCE", in: screen)?.facts.contains { $0.value == "Revenge" } == true)

        let revenge = BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: coordinator.todayViewModel.desk.brokerSlug,
            configuredBrokerSlugs: coordinator.brokersViewModel.configuredBrokerSlugs,
            search: .revenge
        )
        #expect(revenge.bars.count == 1)
        #expect(revenge.chartCaption == nil)

        coordinator.navigateTo(.today)
        #expect(coordinator.activeRoute == .today)
        coordinator.navigateTo(.brokers)
        #expect(coordinator.activeRoute == .brokers)
        coordinator.navigateTo(.settings)
        #expect(coordinator.activeRoute == .settings)
        #expect(deskRoutes.savedDeskRoute == .settings)
    }

    private func payload() -> TodayAgentPayload {
        TodayAgentPayload(
            localDate: "2026-09-28",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: -100, tradesToday: 2, winRate: 0.5),
            topSignals: [
                TodaySignalPayload(kind: "loss_chasing", severity: "firing", name: "Loss chasing", description: "Fired"),
            ],
            trades: [
                TodayTradeRowPayload(
                    closedAt: "2026-09-28T09:18:00.000Z",
                    symbol: "RELIANCE",
                    avgEntry: 2940,
                    avgExit: 2910,
                    qty: 10,
                    netPnlUsd: nil,
                    primaryFlag: "Revenge",
                    flagSeverity: "firing",
                    dataQualityFlags: [],
                    netPnlInr: -1200
                ),
                TodayTradeRowPayload(
                    closedAt: "2026-09-28T11:02:00.000Z",
                    symbol: "INFY",
                    avgEntry: 1800,
                    avgExit: 1810,
                    qty: 5,
                    netPnlUsd: nil,
                    primaryFlag: "Clean",
                    flagSeverity: "clean",
                    dataQualityFlags: [],
                    netPnlInr: 200
                ),
            ],
            openPositionCount: 0,
            brokerSlug: "kotak_neo",
            quoteCurrency: "INR"
        )
    }

    private func pump(_ seconds: TimeInterval) {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline {
            RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.05))
        }
    }
}
