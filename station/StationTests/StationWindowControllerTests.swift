import Foundation
import Testing
@testable import Station

@MainActor
struct StationWindowControllerTests {
    private func makeHarness(
        scenario: FakeAgentSupervisor.Scenario = .healthy,
        launchStore: FakeLaunchStore = FakeLaunchStore()
    ) -> (
        coordinator: StationAppCoordinator,
        agentSupervisor: FakeAgentSupervisor,
        windowController: FakeStationWindowController,
        launchStore: FakeLaunchStore
    ) {
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = scenario
        let windowController = FakeStationWindowController()
        let statusItemController = FakeStatusItemController()
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let notchHost = FakeNotchHost()
        let notchPolling = FakeNotchPolling()
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: statusItemController,
            hotkeyRegistrar: hotkeyRegistrar,
            notchHost: notchHost,
            notchPolling: notchPolling,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: FakeBarSurfacePhaseProvider()
        )
        return (coordinator, agentSupervisor, windowController, launchStore)
    }

    // T4: red button → hide() called, agent still running (supervisor not shutdown)
    @Test func closeHidesOnlyWithoutShuttingDownAgent() async {
        let harness = makeHarness()
        await harness.coordinator.launch()

        harness.coordinator.closeWindow()

        #expect(harness.windowController.hideCallCount == 1)
        #expect(harness.windowController.isVisible == false)
        #expect(harness.agentSupervisor.shutdownCallCount == 0)
        #expect(harness.launchStore.wasWindowVisibleBeforeQuit == false)
    }

    // T5: second showAndActivate() on visible window → no new window created
    @Test func reopenFrontsExistingWindowWithoutCreatingDuplicate() async {
        let harness = makeHarness()
        await harness.coordinator.launch()

        harness.coordinator.openStation()
        harness.coordinator.openStation()

        #expect(harness.windowController.showAndActivateCallCount == 2)
        #expect(harness.windowController.windowsCreated == 1)
    }

    // T6: move/resize → persistFrame() round-trip via UserDefaults key station.window.frame
    @Test func framePersistedRoundTripsThroughUserDefaults() {
        let suiteName = "station.window.tests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let persistence = StationWindowFramePersistence(defaults: defaults)
        let frame = CGRect(x: 120, y: 240, width: 1100, height: 700)

        persistence.persist(frame: frame)

        let restored = persistence.restoreFrame()
        #expect(restored == frame)
        #expect(defaults.data(forKey: StationWindowFramePersistence.userDefaultsKey) != nil)
    }

    // T7: isFirstLaunch=true → show(orderFrontOnly: true), not showAndActivate
    @Test func firstLaunchShowsOrderFrontOnlyWithoutActivation() async {
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = false
        let harness = makeHarness(launchStore: launchStore)

        await harness.coordinator.launch()

        #expect(harness.windowController.showCallCount == 1)
        #expect(harness.windowController.lastShowOrderFrontOnly == true)
        #expect(harness.windowController.showAndActivateCallCount == 0)
        #expect(launchStore.setFirstLaunchCompletedCallCount == 1)
    }

    // T8: loginAtBoot=true, not firstLaunch → no window shown on launch
    @Test func silentBootDoesNotShowWindowOnLaunch() async {
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = true
        launchStore.loginAtBoot = true
        launchStore.wasWindowVisibleBeforeQuit = false
        let harness = makeHarness(launchStore: launchStore)

        await harness.coordinator.launch()

        #expect(harness.windowController.showCallCount == 0)
        #expect(harness.windowController.showAndActivateCallCount == 0)
        #expect(harness.windowController.isVisible == false)
    }

    // T9: status menu "Open Station" → showAndActivate()
    @Test func menuOpenStationActivatesWindow() async {
        let harness = makeHarness()
        await harness.coordinator.launch()

        harness.coordinator.openStation()

        #expect(harness.windowController.showAndActivateCallCount == 1)
        #expect(harness.windowController.isVisible == true)
        #expect(harness.launchStore.wasWindowVisibleBeforeQuit == true)
    }
}
