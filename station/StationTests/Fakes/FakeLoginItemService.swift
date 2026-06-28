import Foundation
@testable import Station

@MainActor
final class FakeLoginItemService: LoginItemServicing {
    var registered = false
    var isPromptDismissed = false
    var syncRegisteredValue = false

    private(set) var setRegisteredCalls: [Bool] = []
    private(set) var syncStatusOnLaunchCallCount = 0
    private(set) var markPromptDismissedCallCount = 0

    var isRegistered: Bool { registered }

    func setRegistered(_ enabled: Bool) throws {
        setRegisteredCalls.append(enabled)
        registered = enabled
    }

    func syncStatusOnLaunch() {
        syncStatusOnLaunchCallCount += 1
        registered = syncRegisteredValue
    }

    func markPromptDismissed() {
        markPromptDismissedCallCount += 1
        isPromptDismissed = true
    }
}

final class FakeLoginItemBackend: LoginItemBackend, @unchecked Sendable {
    var registrationStatus: LoginItemRegistrationStatus = .notRegistered

    private(set) var registerCallCount = 0
    private(set) var unregisterCallCount = 0

    func register() throws {
        registerCallCount += 1
        registrationStatus = .enabled
    }

    func unregister() throws {
        unregisterCallCount += 1
        registrationStatus = .notRegistered
    }
}
