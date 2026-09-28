import Foundation
import Notch
import Testing
@testable import Station

@MainActor
struct DeskRulesHideHostingTests {
    private func makeEphemeralStore() -> (store: DeskRulesStore, defaults: UserDefaults, suite: String) {
        let suite = "desk.rules.hide.host.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = DeskRulesStore(defaults: defaults, storageKeyPrefix: "deskRules")
        return (store, defaults, suite)
    }

    private func makeCoordinator(
        deskRulesStore: DeskRulesStore,
        floatingNotch: FakeFloatingNotchHost,
        scenario: FakeAgentSupervisor.Scenario = .healthy
    ) -> StationAppCoordinator {
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = scenario
        return StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            brokerControl: FakeBrokerControlClient(),
            deskRulesStore: deskRulesStore,
            floatingNotch: floatingNotch
        )
    }

    @Test func hideNotchPreferencePersistsWithoutChangingHostVisibility() async {
        let ephemeral = makeEphemeralStore()
        defer { ephemeral.defaults.removePersistentDomain(forName: ephemeral.suite) }
        let floatingNotch = FakeFloatingNotchHost()
        let coordinator = makeCoordinator(deskRulesStore: ephemeral.store, floatingNotch: floatingNotch)
        await coordinator.launch()

        coordinator.setHideNotch(true)

        #expect(ephemeral.store.hideNotch == true)
        #expect(floatingNotch.hideCallCount == 0)
        #expect(floatingNotch.showCallCount == 0)
        #expect(floatingNotch.toggleCallCount == 0)
    }

    @Test func altSpaceStillSummonsWhenHidePreferenceIsStored() async {
        let ephemeral = makeEphemeralStore()
        defer { ephemeral.defaults.removePersistentDomain(forName: ephemeral.suite) }
        let floatingNotch = FakeFloatingNotchHost()
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = .healthy
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: hotkeyRegistrar,
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            brokerControl: FakeBrokerControlClient(),
            deskRulesStore: ephemeral.store,
            floatingNotch: floatingNotch
        )
        await coordinator.launch()

        coordinator.setHideNotch(true)
        coordinator.setHideNotch(false)

        #expect(ephemeral.store.hideNotch == false)
        #expect(floatingNotch.hideCallCount == 0)
        #expect(floatingNotch.showCallCount == 0)

        hotkeyRegistrar.toggleNotchHandler?()
        #expect(floatingNotch.toggleCallCount == 1)
    }

    @Test func launchIgnoresPersistedHidePreference() async {
        let ephemeral = makeEphemeralStore()
        defer { ephemeral.defaults.removePersistentDomain(forName: ephemeral.suite) }
        ephemeral.store.setHideNotch(true)
        let floatingNotch = FakeFloatingNotchHost()
        let coordinator = makeCoordinator(deskRulesStore: ephemeral.store, floatingNotch: floatingNotch)
        await coordinator.launch()

        #expect(floatingNotch.startCallCount == 1)
        #expect(floatingNotch.hideCallCount == 0)
    }

    @Test func toggleNotchForwardsToFloatingNotchHost() async {
        let ephemeral = makeEphemeralStore()
        defer { ephemeral.defaults.removePersistentDomain(forName: ephemeral.suite) }
        let floatingNotch = FakeFloatingNotchHost()
        let sessionHost = FakeSessionHost()
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = .healthy
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            sessionHost: sessionHost,
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: FakeLaunchStore(),
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            brokerControl: FakeBrokerControlClient(),
            deskRulesStore: ephemeral.store,
            floatingNotch: floatingNotch
        )
        await coordinator.launch()

        coordinator.toggleNotch()

        #expect(floatingNotch.toggleCallCount == 1)
        #expect(sessionHost.toggleCallCount == 0)
    }
}
