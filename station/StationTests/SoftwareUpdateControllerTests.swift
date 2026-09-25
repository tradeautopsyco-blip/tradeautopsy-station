import Testing
@testable import Station

@MainActor
struct SoftwareUpdateControllerTests {
    @Test func coordinatorForwardsAutomaticCheckToggleToSoftwareUpdateController() {
        let fake = FakeSoftwareUpdateController()
        fake.automaticallyChecksForUpdates = false
        let coordinator = makeMinimalCoordinator(softwareUpdateController: fake)

        coordinator.setAutomaticallyChecksForUpdates(true)

        #expect(fake.automaticallyChecksForUpdates == true)
        #expect(coordinator.automaticallyChecksForUpdates == true)
    }

    @Test func coordinatorCheckForUpdatesDelegatesToController() {
        let fake = FakeSoftwareUpdateController()
        let coordinator = makeMinimalCoordinator(softwareUpdateController: fake)

        coordinator.checkForSoftwareUpdates()

        #expect(fake.checkForUpdatesCallCount == 1)
    }

    private func makeMinimalCoordinator(
        softwareUpdateController: SoftwareUpdateControlling
    ) -> StationAppCoordinator {
        StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            softwareUpdateController: softwareUpdateController,
            floatingNotch: FakeFloatingNotchHost()
        )
    }
}
