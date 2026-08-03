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
        let sessionHost = FakeSessionHost()
        let sessionPolling = FakeSessionPolling()
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: statusItemController,
            hotkeyRegistrar: hotkeyRegistrar,
            sessionHost: sessionHost,
            sessionPolling: sessionPolling,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            floatingNotch: FakeFloatingNotchHost()
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

#if DEBUG
    @Test func debugBuildShowsWindowWhenPreviouslyHidden() async {
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = true
        launchStore.wasWindowVisibleBeforeQuit = false
        launchStore.loginAtBoot = false
        let harness = makeHarness(launchStore: launchStore)

        await harness.coordinator.launch()

        #expect(harness.windowController.showCallCount == 1)
        #expect(harness.windowController.lastShowOrderFrontOnly == true)
    }
#endif

    // T9: status menu "Open Station" → showAndActivate()
    @Test func menuOpenStationActivatesWindow() async {
        let harness = makeHarness()
        await harness.coordinator.launch()

        harness.coordinator.openStation()

        #expect(harness.windowController.showAndActivateCallCount == 1)
        #expect(harness.windowController.isVisible == true)
        #expect(harness.launchStore.wasWindowVisibleBeforeQuit == true)
    }

    @Test func defaultFrameUsesMostOfVisibleScreenClampedToMinimum() {
        let visible = CGRect(x: 0, y: 0, width: 2000, height: 1200)
        let frame = StationWindowController.defaultFrame(in: visible)

        #expect(frame.width == 1800) // 90% of 2000
        #expect(frame.height == 1080) // 90% of 1200
        #expect(abs(frame.midX - visible.midX) < 0.5)
        #expect(abs(frame.midY - visible.midY) < 0.5)
    }

    @Test func defaultFrameOnTinyScreenStillRespectsMinimumWhenPossible() {
        let visible = CGRect(x: 10, y: 20, width: 900, height: 600)
        let frame = StationWindowController.defaultFrame(in: visible)

        #expect(frame.width == 820) // max(min, 90% of 900) → 820
        #expect(frame.height == 560) // max(min, 90% of 600) → 560
        #expect(frame.width <= visible.width)
        #expect(frame.height <= visible.height)
    }

    @Test func restoredFrameRejectedWhenTooSmallOrOffscreen() {
        let screen = CGRect(x: 0, y: 0, width: 1440, height: 900)
        let tiny = CGRect(x: 100, y: 100, width: 400, height: 300)
        #expect(
            StationWindowController.isFrameUsable(tiny, onScreenFrames: [screen]) == false
        )

        let offscreen = CGRect(x: 5000, y: 5000, width: 1100, height: 700)
        #expect(
            StationWindowController.isFrameUsable(offscreen, onScreenFrames: [screen]) == false
        )

        let ok = CGRect(x: 100, y: 80, width: 1100, height: 700)
        #expect(StationWindowController.isFrameUsable(ok, onScreenFrames: [screen]))
    }
}
