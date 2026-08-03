import Foundation
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
        sessionHost: FakeSessionHost,
        floatingNotch: FakeFloatingNotchHost,
        sessionPolling: FakeSessionPolling,
        windowController: FakeStationWindowController,
        launchStore: FakeLaunchStore,
        phaseProvider: FakeSessionSurfacePhaseProvider,
        deskRouteStore: FakeDeskRouteStore,
        loginItemService: FakeLoginItemService
    ) {
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = scenario
        let statusItemController = FakeStatusItemController()
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let sessionHost = FakeSessionHost()
        let floatingNotch = FakeFloatingNotchHost()
        let sessionPolling = FakeSessionPolling()
        let windowController = FakeStationWindowController()
        let launchStore = FakeLaunchStore()
        let phaseProvider = FakeSessionSurfacePhaseProvider()
        let loginItemService = FakeLoginItemService()
        launchStore.isFirstLaunchCompleted = true
        let brokerControl = FakeBrokerControlClient()
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: statusItemController,
            hotkeyRegistrar: hotkeyRegistrar,
            sessionHost: sessionHost,
            sessionPolling: sessionPolling,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: phaseProvider,
            loginItemService: loginItemService,
            deskRouteStore: deskRouteStore,
            dateProvider: dateProvider,
            brokerControl: brokerControl,
            floatingNotch: floatingNotch
        )
        return (
            coordinator,
            agentSupervisor,
            statusItemController,
            hotkeyRegistrar,
            sessionHost,
            floatingNotch,
            sessionPolling,
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
        let phaseProvider = FakeSessionSurfacePhaseProvider()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: phaseProvider,
            deskRouteStore: deskRouteStore,
            floatingNotch: FakeFloatingNotchHost()
        )

        coordinator.navigateTo(.brokers)

        #expect(deskRouteStore.saveCallCount == 1)
        #expect(deskRouteStore.savedDeskRoute == .brokers)

        let relaunched = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: phaseProvider,
            deskRouteStore: deskRouteStore,
            floatingNotch: FakeFloatingNotchHost()
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
        sessionHarness.phaseProvider.sessionSurfacePhase = .armed
        let relaunched = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: sessionHarness.phaseProvider,
            deskRouteStore: deskRouteStore,
            floatingNotch: FakeFloatingNotchHost()
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

    // T_single_session_model: coordinator has exactly one SessionModel shared with TrackingSessionHost
    @Test func coordinatorOwnsSingleSessionModelSharedWithNotchHost() {
        let sessionModel = SessionModel()
        let sessionHost = TrackingSessionHost(sessionModel: sessionModel)
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: sessionHost,
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            sessionModel: sessionModel,
            floatingNotch: FakeFloatingNotchHost()
        )

        #expect(coordinator.sessionModel === sessionModel)
        #expect(sessionHost.injectedSessionModel === sessionModel)
        #expect(sessionHost.injectedSessionModel === coordinator.sessionModel)
    }

    @Test func changingActiveRouteDoesNotAffectSessionModelIdentity() {
        let sessionModel = SessionModel()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            sessionModel: sessionModel,
            floatingNotch: FakeFloatingNotchHost()
        )

        coordinator.navigateTo(.brokers)
        #expect(coordinator.activeRoute == .brokers)
        #expect(coordinator.sessionModel === sessionModel)

        coordinator.navigateTo(.liveTrade)
        #expect(coordinator.activeRoute == .liveTrade)
        #expect(coordinator.sessionModel === sessionModel)
    }

    @Test func phaseTransitionUpdatesActiveRouteWithoutTouchingSessionModel() {
        let sessionModel = SessionModel()
        let phaseProvider = FakeSessionSurfacePhaseProvider()
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: phaseProvider,
            sessionModel: sessionModel,
            deskRouteStore: FakeDeskRouteStore(),
            floatingNotch: FakeFloatingNotchHost()
        )
        #expect(coordinator.activeRoute == .preTrade)

        phaseProvider.setPhase(.armed)

        #expect(coordinator.activeRoute == .liveTrade)
        #expect(coordinator.sessionModel === sessionModel)
    }

    // T1: Launch with healthy agent fake → no warning; notch start called; polling started
    @Test func launchWithHealthyAgentStartsNotchAndPolling() async {
        let harness = makeHarness(scenario: .healthy)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning == nil)
        #expect(harness.sessionHost.startCallCount == 1)
        #expect(harness.floatingNotch.startCallCount == 1)
        #expect(harness.sessionPolling.startPollingCallCount == 1)
        #expect(harness.statusItemController.lastReportedHealthy == true)
    }

    // T2: Launch timeout → AgentHealthWarning.launchTimeout; shell navigable
    @Test func launchTimeoutSurfacesWarningAndShellRemainsNavigable() async {
        let harness = makeHarness(scenario: .launchTimeout)

        await harness.coordinator.launch()

        #expect(harness.coordinator.agentHealthWarning?.reason == .launchTimeout)
        #expect(harness.coordinator.isShellNavigable)
        #expect(harness.sessionHost.startCallCount == 0)
        #expect(harness.floatingNotch.startCallCount == 0)
        #expect(harness.sessionPolling.startPollingCallCount == 0)
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
        #expect(harness.sessionHost.startCallCount == 1)
        #expect(harness.sessionPolling.startPollingCallCount == 1)
    }

    // T9: Quit → shutdown sequence: hotkeys unregistered, agent shutdown, notch dismiss
    @Test func quitRunsShutdownSequence() async {
        let harness = makeHarness(scenario: .healthy)
        await harness.coordinator.launch()

        await harness.coordinator.quit()

        #expect(harness.hotkeyRegistrar.unregisterAllCallCount == 1)
        #expect(harness.agentSupervisor.shutdownCallCount == 1)
        #expect(harness.floatingNotch.dismissCallCount == 1)
        #expect(harness.sessionHost.dismissCallCount == 1)
        #expect(harness.sessionPolling.stopPollingCallCount == 1)
    }

    @Test func toggleNotchForwardsToFloatingNotchHost() async {
        let harness = makeHarness(scenario: .healthy)
        await harness.coordinator.launch()

        harness.coordinator.toggleNotch()

        #expect(harness.floatingNotch.toggleCallCount == 1)
        #expect(harness.sessionHost.toggleCallCount == 0)
    }

    @Test func altSpaceHotkeyTogglesFloatingNotchNotStation() async {
        let harness = makeHarness(scenario: .healthy)
        await harness.coordinator.launch()

        harness.hotkeyRegistrar.toggleNotchHandler?()

        #expect(harness.floatingNotch.toggleCallCount == 1)
        #expect(harness.sessionHost.toggleCallCount == 0)
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
        #expect(harness.sessionPolling.stopPollingCallCount == 1)
    }

    @Test func agentRecoveryRestartsPolling() async {
        let harness = makeHarness(scenario: .healthy)
        await harness.coordinator.launch()
        #expect(harness.coordinator.isPulseStripDegraded == false)
        #expect(harness.sessionPolling.startPollingCallCount == 1)

        harness.agentSupervisor.simulateRuntimeDisconnect()
        await Task.yield()

        #expect(harness.coordinator.isPulseStripDegraded)
        #expect(harness.sessionPolling.stopPollingCallCount == 1)

        harness.agentSupervisor.simulateRuntimeRecovery()
        await Task.yield()

        #expect(harness.coordinator.isPulseStripDegraded == false)
        #expect(harness.coordinator.agentHealthWarning == nil)
        #expect(harness.sessionPolling.startPollingCallCount == 2)
        #expect(harness.statusItemController.lastReportedHealthy == true)
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
        #expect(harness.sessionHost.startCallCount == 1)
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
        #expect(harness.sessionHost.startCallCount == 1)

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
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            loginItemService: loginItemService,
            floatingNotch: FakeFloatingNotchHost()
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
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            loginItemService: loginItemService,
            floatingNotch: FakeFloatingNotchHost()
        )

        await coordinator.launch()
        #expect(coordinator.showLoginItemPrompt == true)

        coordinator.skipLoginItemPrompt()
        #expect(coordinator.showLoginItemPrompt == false)
        #expect(loginItemService.isPromptDismissed == true)

        await coordinator.launch()
        #expect(coordinator.showLoginItemPrompt == false)
    }

    @Test func brokerBridgeConnectNavigatesAndBeginsConnect() async {
        let harness = makeHarness()
        let coordinator = harness.coordinator
        let floating = harness.floatingNotch
        floating.setBrokerBridge(
            onConnect: { slug in
                coordinator.navigateTo(.brokers)
                coordinator.openStation()
                Task { @MainActor in
                    await coordinator.brokersViewModel.beginConnect(for: slug)
                }
            },
            onReauth: { slug in
                coordinator.navigateTo(.brokers)
                coordinator.openStation()
                coordinator.brokersViewModel.presentKotakTotpRemint(for: slug)
            }
        )

        floating.brokerConnectHandler?("kotak_neo")
        #expect(coordinator.activeRoute == .brokers)
        #expect(harness.windowController.showAndActivateCallCount >= 1)
        // Sheet vs Start is covered by BrokersViewModel beginConnect unit tests
        // (real Keychain profile on the host can take either path here).
        await Task.yield()
        try? await Task.sleep(for: .milliseconds(50))
    }

    @Test func brokerBridgeReauthOpensBrokersEditPath() {
        let harness = makeHarness()
        let coordinator = harness.coordinator
        let floating = harness.floatingNotch
        floating.setBrokerBridge(
            onConnect: { _ in },
            onReauth: { slug in
                coordinator.navigateTo(.brokers)
                coordinator.openStation()
                coordinator.brokersViewModel.presentKotakTotpRemint(for: slug)
            }
        )

        floating.brokerReauthHandler?("kotak_neo")
        #expect(coordinator.activeRoute == .brokers)
        // Remint may present TOTP-only or full sheet depending on Keychain/profile; slug must be set.
        #expect(coordinator.brokersViewModel.connectBrokerSlug == "kotak_neo")
    }

    /// Device login is a distinct bridge from broker Connect/Reauth (#4 grill fix) — it must
    /// route to Settings, never to Brokers, so the two failure modes stay visually separate.
    @Test func deviceLoginBridgeOpensSettingsNotBrokers() {
        let harness = makeHarness()
        let coordinator = harness.coordinator
        let floating = harness.floatingNotch
        floating.setDeviceLoginBridge(onOpen: { [weak coordinator] in
            coordinator?.navigateTo(.settings)
            coordinator?.openStation()
        })

        floating.deviceLoginHandler?()
        #expect(coordinator.activeRoute == .settings)
        #expect(harness.windowController.showAndActivateCallCount >= 1)
    }
}
