import Foundation
@testable import Station

@MainActor
final class FakeHotkeyRegistrar: HotkeyRegistering {
    private(set) var registerToggleNotchCallCount = 0
    private(set) var registerOpenStationCallCount = 0
    private(set) var unregisterAllCallCount = 0

    func registerToggleNotch(_ handler: @escaping () -> Void) {
        registerToggleNotchCallCount += 1
    }

    func registerOpenStation(_ handler: @escaping () -> Void) {
        registerOpenStationCallCount += 1
    }

    func unregisterAll() {
        unregisterAllCallCount += 1
    }
}
