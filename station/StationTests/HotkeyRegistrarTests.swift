import AppKit
import Foundation
import Testing
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

    @Test func registrarDoesNotRequestInputMonitoringForCarbonHotkeys() {
        let checker = FakeInputMonitoringChecker(granted: false)
        let registrar = HotkeyRegistrar(inputMonitoringChecker: checker)
        registrar.registerToggleNotch {}

        #expect(checker.requestAccessCallCount == 0)
    }

    @Test func altSpaceTogglesFloatingNotch() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: true))
        registrar.registerToggleNotch { floatingNotch.toggle() }

        registrar.dispatchKeyDownForTesting(makeKeyEvent(modifierFlags: .option))

        #expect(floatingNotch.toggleCallCount == 1)
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

    @Test func sessionPollingHostToggleRaisesCallbackWithoutInstallingHotkeys() {
        let host = SessionPollingHost(sessionModel: SessionModel())
        var toggleCount = 0
        host.onToggle = { toggleCount += 1 }
        host.toggle()
        #expect(toggleCount == 1)
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
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            floatingNotch: FakeFloatingNotchHost()
        )

        await coordinator.launch()
        await coordinator.quit()

        #expect(hotkeyRegistrar.unregisterAllCallCount == 1)
    }

    @Test func inputMonitoringMissingDoesNotBlockHotkeysOrBanner() async {
        let checker = FakeInputMonitoringChecker(granted: false)
        let coordinator = makeCoordinator(inputMonitoringChecker: checker)

        await coordinator.launch()

        // Carbon hotkeys do not require Input Monitoring — no banner.
        #expect(coordinator.inputMonitoringWarning == nil)
    }

    @Test func inputMonitoringGranted() async {
        let checker = FakeInputMonitoringChecker(granted: true)
        let coordinator = makeCoordinator(inputMonitoringChecker: checker)

        await coordinator.launch()

        #expect(coordinator.inputMonitoringWarning == nil)
    }

    @Test func altSpaceMatchesWithCapsLockOn() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        registrar.registerToggleNotch { floatingNotch.toggle() }

        registrar.dispatchKeyDownForTesting(makeKeyEvent(modifierFlags: [.option, .capsLock]))

        #expect(floatingNotch.toggleCallCount == 1)
    }

    @Test func carbonHotKeyIdTogglesNotch() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        registrar.registerToggleNotch { floatingNotch.toggle() }

        registrar.dispatchCarbonHotKeyForTesting(id: 1)

        #expect(floatingNotch.toggleCallCount == 1)
    }

    // The Notch summon must start on the same run-loop turn as the keypress. If the Carbon
    // handler ever hops through DispatchQueue.main.async again, the toggle would still be 0 here.
    @Test func carbonHotKeyRunsHandlerSynchronouslyOnMain() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        var toggledOnSameTurn = false
        registrar.registerToggleNotch {
            floatingNotch.toggle()
            toggledOnSameTurn = true
        }

        registrar.dispatchCarbonHotKeyForTesting(id: 1)

        #expect(toggledOnSameTurn)
        #expect(floatingNotch.toggleCallCount == 1)
    }

    @Test func systemSettingsDeepLinkUsesInputMonitoringPane() {
        #expect(
            InputMonitoringWarning.systemSettingsURL.absoluteString
                == "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        )
    }

    @Test func recheckDoesNotSurfaceInputMonitoringBannerForHotkeys() async {
        let checker = FakeInputMonitoringChecker(granted: false)
        let hotkeyRegistrar = FakeHotkeyRegistrar()
        let launchStore = FakeLaunchStore()
        launchStore.isFirstLaunchCompleted = true
        let coordinator = StationAppCoordinator(
            agentSupervisor: FakeAgentSupervisor(),
            statusItemController: FakeStatusItemController(),
            hotkeyRegistrar: hotkeyRegistrar,
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            inputMonitoringChecker: checker,
            floatingNotch: FakeFloatingNotchHost()
        )

        await coordinator.launch()
        #expect(coordinator.inputMonitoringWarning == nil)

        checker.granted = true
        coordinator.recheckInputMonitoringAccess()

        #expect(coordinator.inputMonitoringWarning == nil)
        #expect(coordinator.inputMonitoringRestartReminder == nil)
    }

    @Test func recheckDoesNotShowRestartReminderWhenHotkeysNeedNoPermission() async {
        var now = Date(timeIntervalSince1970: 0)
        let checker = FakeInputMonitoringChecker(granted: false)
        let coordinator = makeCoordinator(
            dateProvider: { now },
            inputMonitoringChecker: checker
        )

        await coordinator.launch()
        now = now.addingTimeInterval(4)
        coordinator.recheckInputMonitoringAccess()

        #expect(coordinator.inputMonitoringRestartReminder == nil)
    }

    @Test func emptySavedBindingsKeepDefaultCarbonIds() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        defer { registrar.unregisterAll() }
        registrar.registerToggleNotch { floatingNotch.toggle() }
        registrar.reloadSavedBindings([])

        registrar.dispatchCarbonHotKeyForTesting(id: 1)

        #expect(floatingNotch.toggleCallCount == 1)
    }

    @Test func savedToggleReplacesDefaultCarbonId() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        defer { registrar.unregisterAll() }
        registrar.registerToggleNotch { floatingNotch.toggle() }
        registrar.reloadSavedBindings([
            DeskHotkeyRegistration(actionId: "toggle_notch", keyCode: 122, carbonModifiers: 0x800),
        ])

        registrar.dispatchCarbonHotKeyForTesting(id: 1)
        #expect(floatingNotch.toggleCallCount == 0)

        registrar.dispatchCarbonHotKeyForTesting(id: 10)
        #expect(floatingNotch.toggleCallCount == 1)
    }

    @Test func clearingSavedBindingsRestoresDefaultCarbonId() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        defer { registrar.unregisterAll() }
        registrar.registerToggleNotch { floatingNotch.toggle() }
        registrar.reloadSavedBindings([
            DeskHotkeyRegistration(actionId: "toggle_notch", keyCode: 122, carbonModifiers: 0x800),
        ])
        registrar.reloadSavedBindings([])

        registrar.dispatchCarbonHotKeyForTesting(id: 1)
        #expect(floatingNotch.toggleCallCount == 1)
    }

    @Test func savedDeskBindingDispatchesActionId() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        defer { registrar.unregisterAll() }
        registrar.registerDeskActions { floatingNotch.performHotkey($0) }
        registrar.reloadSavedBindings([
            DeskHotkeyRegistration(actionId: "kill", keyCode: 40, carbonModifiers: 0x800),
        ])

        registrar.dispatchCarbonHotKeyForTesting(id: 10)

        #expect(floatingNotch.performHotkeyCallCount == 1)
        #expect(floatingNotch.lastHotkeyActionId == "kill")
    }

    @Test func unboundCarbonIdDoesNothing() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        defer { registrar.unregisterAll() }
        registrar.registerDeskActions { floatingNotch.performHotkey($0) }
        registrar.reloadSavedBindings([])

        registrar.dispatchCarbonHotKeyForTesting(id: 10)

        #expect(floatingNotch.performHotkeyCallCount == 0)
    }

    @Test func altSpaceDoesNotToggleWhenSavedBindingSuppressesDefault() {
        let floatingNotch = FakeFloatingNotchHost()
        let registrar = HotkeyRegistrar(inputMonitoringChecker: FakeInputMonitoringChecker(granted: false))
        defer { registrar.unregisterAll() }
        registrar.registerToggleNotch { floatingNotch.toggle() }
        registrar.reloadSavedBindings([
            DeskHotkeyRegistration(actionId: "toggle_notch", keyCode: 122, carbonModifiers: 0x800),
        ])

        registrar.dispatchKeyDownForTesting(makeKeyEvent(modifierFlags: .option))

        #expect(floatingNotch.toggleCallCount == 0)
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
            sessionHost: FakeSessionHost(),
            sessionPolling: FakeSessionPolling(),
            windowController: FakeStationWindowController(),
            launchStore: launchStore,
            phaseProvider: FakeSessionSurfacePhaseProvider(),
            dateProvider: dateProvider,
            inputMonitoringChecker: inputMonitoringChecker,
            floatingNotch: FakeFloatingNotchHost()
        )
    }
}
