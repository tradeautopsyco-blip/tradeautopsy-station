import AppKit
import Foundation
import Testing
@testable import Notch
@testable import Station

private struct FakeInputMonitoringChecker: InputMonitoringChecking {
    let granted: Bool

    func isInputMonitoringGranted() -> Bool { granted }
}

@MainActor
struct HotkeyRegistrarTests {
    private func makeKeyEvent(modifierFlags: NSEvent.ModifierFlags) -> NSEvent {
        NSEvent.keyEvent(
            with: .keyDown,
            location: .zero,
            modifierFlags: modifierFlags,
            timestamp: 0,
            windowNumber: 0,
            context: nil,
            characters: " ",
            charactersIgnoringModifiers: " ",
            isARepeat: false,
            keyCode: 49
        )!
    }

    @Test func altSpaceTogglesNotch() {
        let notchHost = FakeNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: true))
        registrar.registerToggleNotch { notchHost.toggle() }

        registrar.dispatchKeyDownForTesting(makeKeyEvent(modifierFlags: .option))

        #expect(notchHost.toggleCallCount == 1)
    }

    @Test func altShiftSpaceOpensStation() {
        let windowController = FakeStationWindowController()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: true))
        registrar.registerOpenStation { windowController.showAndActivate() }

        registrar.dispatchKeyDownForTesting(makeKeyEvent(modifierFlags: [.option, .shift]))

        #expect(windowController.showAndActivateCallCount == 1)
    }

    @Test func noDuplicateHotkeysWhenNotchLauncherHosted() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.start()

        #expect(launcher.hasInstalledToggleHotkeyMonitors == false)

        launcher.dismiss()
    }

    @Test func unregisterOnQuit() async {
        let agentSupervisor = FakeAgentSupervisor()
        agentSupervisor.scenario = .healthy
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = true
        let coordinator = StationAppCoordinator(
            agentSupervisor: agentSupervisor,
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: hotkeyRegistrar,
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeBarSurfacePhaseProvider()
        )

        await coordinator.launch()
        await coordinator.quit()

        #expect(hotkeyRegistrar.unregisterAllCallCount == 1)
    }

    @Test func inputMonitoringMissing() async {
        let checker = FakeInputMonitoringChecker(granted: false)
        let coordinator = makeCoordinator(inputMonitoringChecker: checker)

        await coordinator.launch()

        #expect(coordinator.inputMonitoringWarning != nil)
        #expect(coordinator.inputMonitoringWarning?.message.contains("Input Monitoring") == true)
    }

    @Test func inputMonitoringGranted() async {
        let checker = FakeInputMonitoringChecker(granted: true)
        let coordinator = makeCoordinator(inputMonitoringChecker: checker)

        await coordinator.launch()

        #expect(coordinator.inputMonitoringWarning == nil)
    }

    private func makeCoordinator(
        inputMonitoringChecker: InputMonitoringChecking
    ) -> StationAppCoordinator {
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = true
        return StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: FakeHotkeyRegistrar(),
            notchHost: FakeNotchHost(),
            notchPolling: FakeNotchPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeBarSurfacePhaseProvider(),
            inputMonitoringChecker: inputMonitoringChecker
        )
    }
}
