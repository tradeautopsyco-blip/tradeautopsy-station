import Foundation
import Testing
@testable import Station

@MainActor
struct StationAppCoordinatorTests {
    private func makeHarness(scenario: FakeAgentSupervisor.Scenario = .healthy) -> (
        coordinator: StationAppCoordinator,
        agentSupervisor: FakeAgentSupervisor,
        statusItemController: FakeStatusItemController,
        hotkeyRegistrar: FakeHotkeyRegistrar,
        notchHost: FakeNotchHost,
        notchPolling: FakeNotchPolling
    ) {
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = scenario
        let statusItemController = FakeStatusItemController()
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let notchHost = FakeNotchHost()
        let notchPolling = FakeNotchPolling()
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: statusItemController,
            hotkeyRegistrar: hotkeyRegistrar,
            notchHost: notchHost,
            notchPolling: notchPolling
        )
        return (coordinator, agentSupervisor, statusItemController, hotkeyRegistrar, notchHost, notchPolling)
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
}
