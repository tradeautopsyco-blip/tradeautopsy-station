import AppKit
import Foundation
import Testing
@testable import Notch
@testable import Station

private final class FakeInputMonitoringChecker: InputMonitoringChecking, @unchecked Sendable {
    var granted: Bool
    private(set) var requestAccessCallCount = 0

    init(granted: Bool) {
        self.granted = granted
    }

    func isInputMonitoringGranted() -> Bool { granted }

    func requestInputMonitoringAccess() {
        requestAccessCallCount += 1
    }
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

    @Test func registrarRequestsInputMonitoringAccessBeforeMonitors() {
        let checker = FakeInputMonitoringChecker(granted: true)
        let registrar = HotkeyRegistrar(inputMonitoringChecker: checker)
        registrar.registerToggleNotch {}

        #expect(checker.requestAccessCallCount == 1)
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

    @Test func openStationShortcutWorksLocallyWithoutInputMonitoring() {
        let windowController = FakeStationWindowController()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
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

    @Test func systemSettingsDeepLinkUsesInputMonitoringPane() {
        #expect(
            InputMonitoringWarning.systemSettingsURL.absoluteString
                == "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        )
    }

    @Test func recheckShowsRestartReminderWhenPermissionGranted() async {
        let checker = FakeInputMonitoringChecker(granted: false)
        let coordinator = makeCoordinator(inputMonitoringChecker: checker)

        await coordinator.launch()
        #expect(coordinator.inputMonitoringWarning != nil)
        #expect(coordinator.inputMonitoringRestartReminder == nil)

        checker.granted = true
        coordinator.recheckInputMonitoringAccess()

        #expect(coordinator.inputMonitoringWarning == nil)
        #expect(coordinator.inputMonitoringRestartReminder?.contains("quit and reopen") == true)
    }

    @Test func recheckShowsRestartReminderAfterDelayWhenStillDenied() async {
        var now = Date(timeIntervalSince1970: 0)
        let checker = FakeInputMonitoringChecker(granted: false)
        let coordinator = makeCoordinator(
            dateProvider: { now },
            inputMonitoringChecker: checker
        )

        await coordinator.launch()
        #expect(coordinator.inputMonitoringRestartReminder == nil)

        now = now.addingTimeInterval(4)
        coordinator.recheckInputMonitoringAccess()

        #expect(coordinator.inputMonitoringRestartReminder?.contains("quit and reopen") == true)
    }

    private func makeCoordinator(
        dateProvider: @escaping () -> Date = Date.init,
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
            dateProvider: dateProvider,
            inputMonitoringChecker: inputMonitoringChecker
        )
    }
}
