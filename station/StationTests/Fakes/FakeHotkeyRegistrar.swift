import Foundation
@testable import Station

@MainActor
final class FakeHotkeyRegistrar: HotkeyRegistering {
    private(set) var registerToggleNotchCallCount = 0
    private(set) var registerOpenStationCallCount = 0
    private(set) var unregisterAllCallCount = 0
    private(set) var toggleNotchHandler: (() -> Void)?
    private(set) var openStationHandler: (() -> Void)?

    private(set) var refreshGlobalMonitorCallCount = 0

    func registerToggleNotch(_ handler: @escaping () -> Void) {
        registerToggleNotchCallCount += 1
        toggleNotchHandler = handler
    }

    func registerOpenStation(_ handler: @escaping () -> Void) {
        registerOpenStationCallCount += 1
        openStationHandler = handler
    }

    func unregisterAll() {
        unregisterAllCallCount += 1
        toggleNotchHandler = nil
        openStationHandler = nil
    }

    func refreshGlobalMonitorIfNeeded() {
        refreshGlobalMonitorCallCount += 1
    }
}
