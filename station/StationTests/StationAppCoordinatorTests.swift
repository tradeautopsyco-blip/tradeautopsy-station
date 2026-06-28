import Foundation
import Notch
import Testing
@testable import Station

@MainActor
struct StationAppCoordinatorTests {
    private func makeHarness(
        scenario: FakeAgentSupervisor.Scenario = .healthy,
        deskRouteStore: FakeDeskRouteStore = FakeDeskRouteStore(),
        dateProvider: @escaping () -> Date = Date.init
    ) -> (
        coordinator: StationAppCoordinator,
        agentSupervisor: FakeAgentSupervisor,
        statusItemController: FakeStatusItemController,
        hotkeyRegistrar: FakeHotkeyRegistrar,
        notchHost: FakeNotchHost,
        notchPolling: FakeNotchPolling,
        windowController: FakeStationWindowController,
        launchStore: FakeLaunchStore,
        phaseProvider: FakeBarSurfacePhaseProvider,
        deskRouteStore: FakeDeskRouteStore,
        loginItemService: FakeLoginItemService
    ) {
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = scenario
        let statusItemController = FakeStatusItemController()
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let notchHost = FakeNotchHost()
        let notchPolling = FakeNotchPolling()
        let windowController = FakeStationWindowController()
        let launchStore = FakeLaunchStore()
        let phaseProvider = FakeBarSurfacePhaseProvider()
        let loginItemService = FakeLoginItemService()
        launchStore.isFirstLaunchCompleted = true
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: statusItemController,
            hotkeyRegistrar: hotkeyRegistrar,
            notchHost: notchHost,
            notchPolling: notchPolling,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: phaseProvider,
            loginItemService: loginItemService,
            deskRouteStore: deskRouteStore,
            dateProvider: dateProvider
        )
        return (
            coordinator,
            agentSupervisor,
            statusItemController,
            hotkeyRegistrar,
            notchHost,
            notchPolling,
            windowController,
            launchStore,
            phaseProvider,
            deskRouteStore,
            loginItemService
        )
    }

    // T4: Phase transition, no manual pick → activeRoute updates
    @Test func phaseTransitionWithoutManualPickUpdatesActiveRoute() {
        let harness = makeHarness()
        #expect(harness.coordinator.activeRoute == .preTrade)

        harness.phaseProvider.setPhase(.armed)

        #expect(harness.coordinator.activeRoute == .liveTrade)
    }

    // T5: Manual Session pick → no auto-follow for 60s
    @Test func manualSessionPickBlocksAutoFollowFor60Seconds() {
        var now = Date(timeIntervalSince1970: 1_000_000)
        let harness = makeHarness(dateProvider: { now })
        harness.coordinator.navigateTo(.today)
        #expect(harness.coordinator.activeRoute == .today)

        harness.phaseProvider.setPhase(.armed)
        #expect(harness.coordinator.activeRoute == .today)

        now = now.addingTimeInterval(30)
        harness.phaseProvider.setPhase(.livePlan)
        #expect(harness.coordinator.activeRoute == .today)

        now = now.addingTimeInterval(31)
        harness.phaseProvider.setPhase(.debrief)
        #expect(harness.coordinator.activeRoute == .postTrade)
    }

    // T6: Desk route persistence → save/load round-trip
    @Test func deskRoutePersistsAcrossCoordinatorInstances() {
        let deskRouteStore = FakeDeskRouteStore()
        let phaseProvider = FakeBarSurfacePhaseProvider()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: phaseProvider,
            deskRouteStore: deskRouteStore
        )

        coordinator.navigateTo(.brokers)

        #expect(deskRouteStore.saveCallCount == 1)
        #expect(deskRouteStore.savedDeskRoute == .brokers)

        let relaunched = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: phaseProvider,
            deskRouteStore: deskRouteStore
        )

        #expect(relaunched.activeRoute == .brokers)
    }

    @Test func deskRouteRoundTripsThroughUserDefaults() {
        let suiteName = "StationTests.DeskRoute.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let store = UserDefaultsDeskRouteStore(defaults: defaults)
        store.saveDeskRoute(.escrowMatch)

        let loaded = UserDefaultsDeskRouteStore(defaults: defaults)
        #expect(loaded.savedDeskRoute == .escrowMatch)
    }

    // T7: Launch route → saved Desk restored; Session from phase (not saved Session)
    @Test func launchRouteRestoresDeskAndIgnoresSavedSession() {
        let deskRouteStore = FakeDeskRouteStore()
        deskRouteStore.saveDeskRoute(.patterns)

        let deskHarness = makeHarness(deskRouteStore: deskRouteStore)
        #expect(deskHarness.coordinator.activeRoute == .patterns)

        deskRouteStore.saveDeskRoute(.today)
        let sessionHarness = makeHarness(deskRouteStore: deskRouteStore)
        sessionHarness.phaseProvider.barSurfacePhase = .armed
        let relaunched = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: sessionHarness.phaseProvider,
            deskRouteStore: deskRouteStore
        )
        #expect(relaunched.activeRoute == .liveTrade)
    }

    @Test func deskRouteNeverAutoFollowsPhase() {
        let harness = makeHarness()
        harness.coordinator.navigateTo(.settings)
        #expect(harness.coordinator.activeRoute == .settings)

        harness.phaseProvider.setPhase(.armed)
        #expect(harness.coordinator.activeRoute == .settings)
    }

    // T_single_viewmodel: coordinator has exactly one NotchViewModel; NotchLauncher receives same instance
    @Test func coordinatorOwnsSingleNotchViewModelSharedWithNotchHost() {
        let viewModel = NotchViewModel()
        let notchHost = TrackingNotchHost(viewModel: viewModel)
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: notchHost,
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeBarSurfacePhaseProvider(),
            notchViewModel: viewModel
        )

        #expect(coordinator.notchViewModel === viewModel)
        #expect(notchHost.injectedViewModel === viewModel)
        #expect(notchHost.injectedViewModel === coordinator.notchViewModel)
    }

    // T1: Launch with healthy agent fake → no warning; notch start called; polling started
    @Test func launchWithHealthyAgentStartsNotchAndPolling() async {
        let harness = makeHarness(scenario: .healthy)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning == nil)
        #expect(harness.notchHost.startCallCount == 1)
        #expect(harness.notchPolling.startPollingCallCount == 1)
        #expect(harness.statusItemController.lastReportedHealthy == true)
    }

    // T2: Launch timeout → AgentHealthWarning.launchTimeout; shell navigable
    @Test func launchTimeoutSurfacesWarningAndShellRemainsNavigable() async {
        let harness = makeHarness(scenario: .launchTimeout)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning?.reason == .launchTimeout)
        #expect(harness.coordinator.isShellNavigable)
        #expect(harness.notchHost.startCallCount == 0)
        #expect(harness.notchPolling.startPollingCallCount == 0)
    }

    // T3: Port collision non-agent → blocking warning with canRetry
    @Test func portCollisionNonAgentSurfacesBlockingRetryableWarning() async {
        let harness = makeHarness(scenario: .portCollisionNonAgent)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning?.reason == .portCollisionNonAgent)
        #expect(harness.coordinator.agentHealthWarning?.canRetry == true)
        #expect(harness.agentSupervisor.isHealthy == false)
    }

    // T8: Retry after crash loop → supervisor retry invoked; counter reset
    @Test func retryAfterCrashLoopInvokesSupervisorRetry() async {
        let harness = makeHarness(scenario: .crashLoopExceeded)
        await harness.coordinator.launch()
        #expect(harness.coordinator.agentHealthWarning?.reason == .crashLoopExceeded)

        await harness.coordinator.retryAgent()

        #expect(harness.agentSupervisor.retryCallCount == 1)
        #expect(harness.coordinator.agentHealthWarning == nil)
        #expect(harness.notchHost.startCallCount == 1)
        #expect(harness.notchPolling.startPollingCallCount == 1)
    }

    // T9: Quit → shutdown sequence: hotkeys unregistered, agent shutdown, notch dismiss
    @Test func quitRunsShutdownSequence() async {
        let harness = makeHarness(scenario: .healthy)
        await harness.coordinator.launch()

        await harness.coordinator.quit()

        #expect(harness.hotkeyRegistrar.unregisterAllCallCount == 1)
        #expect(harness.agentSupervisor.shutdownCallCount == 1)
        #expect(harness.notchHost.dismissCallCount == 1)
        #expect(harness.notchPolling.stopPollingCallCount == 1)
    }

    // T_crash_loop: 3 crashes in 60s → crashLoopExceeded warning, canRetry=true, auto-restart stops
    @Test func crashLoopSurfacesExceededWarningWithRetryAndNoAutoRestart() async {
        let harness = makeHarness(scenario: .crashLoopExceeded)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning?.reason == .crashLoopExceeded)
        #expect(harness.coordinator.agentHealthWarning?.canRetry == true)
        #expect(harness.agentSupervisor.startCallCount == 1)
        #expect(harness.coordinator.isShellNavigable)
    }

    // T_runtime_disconnect: healthy → disconnect → runtimeDisconnected warning, strip degrades to —
    @Test func runtimeDisconnectSurfacesWarningAndDegradesPulseStrip() async {
        let harness = makeHarness(scenario: .healthy)
        await harness.coordinator.launch()
        #expect(harness.coordinator.isPulseStripDegraded == false)

        harness.agentSupervisor.simulateRuntimeDisconnect()
        await Task.yield()

        #expect(harness.coordinator.agentHealthWarning?.reason == .runtimeDisconnected)
        #expect(harness.coordinator.isPulseStripDegraded)
        #expect(harness.statusItemController.lastReportedHealthy == false)
    }

    // T_retry_resets: crashLoopExceeded → retry() → counter reset, supervisor start called again
    @Test func retryResetsCrashLoopAndRestartsSupervisor() async {
        let harness = makeHarness(scenario: .crashLoopExceeded)
        await harness.coordinator.launch()
        #expect(harness.agentSupervisor.startCallCount == 1)

        await harness.coordinator.retryAgent()

        #expect(harness.agentSupervisor.retryCallCount == 1)
        #expect(harness.coordinator.agentHealthWarning == nil)
        #expect(harness.agentSupervisor.startCallCount == 1)
        #expect(harness.notchHost.startCallCount == 1)
    }

    // T_port_collision: non-agent on 9137 → portCollisionNonAgent warning, message includes port guidance
    @Test func portCollisionIncludesPortGuidance() async {
        let harness = makeHarness(scenario: .portCollisionNonAgent)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning?.reason == .portCollisionNonAgent)
        #expect(harness.coordinator.agentHealthWarning?.message.contains("9137") == true)
        #expect(harness.coordinator.agentHealthWarning?.message.contains("DAEMON_PORT") == true)
    }

    // T_port_attach: verified TradeAutopsy agent on 9137 → attach, no warning, no spawn
    @Test func portAttachUsesExistingAgentWithoutSpawnOrWarning() async {
        let harness = makeHarness(scenario: .attachedHealthy)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning == nil)
        #expect(harness.agentSupervisor.ownsSpawnedAgent == false)
        #expect(harness.notchHost.startCallCount == 1)

        await harness.coordinator.quit()

        #expect(harness.agentSupervisor.shutdownCallCount == 1)
        #expect(harness.agentSupervisor.ownsSpawnedAgent == false)
    }

    // T_login_item_toggle: coordinator.toggleLaunchAtLogin() → setRegistered with correct value
    @Test func loginItemToggleCallsSetRegisteredWithNextValue() async {
        let loginItemService = FakeLoginItemService()
        let statusItemController = FakeStatusItemController()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: statusItemController,
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeBarSurfacePhaseProvider(),
            loginItemService: loginItemService
        )

        await coordinator.launch()
        #expect(coordinator.launchAtLoginEnabled == false)

        coordinator.toggleLaunchAtLogin()
        #expect(loginItemService.setRegisteredCalls == [true])
        #expect(coordinator.launchAtLoginEnabled == true)
        #expect(statusItemController.lastLaunchAtLoginEnabled == true)

        coordinator.toggleLaunchAtLogin()
        #expect(loginItemService.setRegisteredCalls == [true, false])
        #expect(coordinator.launchAtLoginEnabled == false)
    }

    @Test func firstLaunchShowsLoginItemPromptOnceUntilDismissed() async {
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = false
        let loginItemService = FakeLoginItemService()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeBarSurfacePhaseProvider(),
            loginItemService: loginItemService
        )

        await coordinator.launch()
        #expect(coordinator.showLoginItemPrompt == true)

        coordinator.skipLoginItemPrompt()
        #expect(coordinator.showLoginItemPrompt == false)
        #expect(loginItemService.isPromptDismissed == true)

        await coordinator.launch()
        #expect(coordinator.showLoginItemPrompt == false)
    }
}
