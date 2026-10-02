import Foundation
@testable import Station

@MainActor
final class FakeStatusItemController: StatusItemControlling {
    private(set) var installCallCount = 0
    private(set) var updateAgentStatusCallCount = 0
    private(set) var lastReportedHealthy: Bool?

    func install(coordinator: StationAppCoordinator) {
        installCallCount += 1
    }

    func updateAgentStatus(isHealthy: Bool) {
        updateAgentStatusCallCount += 1
        lastReportedHealthy = isHealthy
    }

    private(set) var updateLaunchAtLoginEnabledCallCount = 0
    private(set) var lastLaunchAtLoginEnabled: Bool?

    func updateLaunchAtLoginEnabled(_ enabled: Bool) {
        updateLaunchAtLoginEnabledCallCount += 1
        lastLaunchAtLoginEnabled = enabled
    }

    private(set) var updateStationLoginRequiredCallCount = 0
    private(set) var lastStationLoginRequired: Bool?

    func updateStationLoginRequired(_ required: Bool) {
        updateStationLoginRequiredCallCount += 1
        lastStationLoginRequired = required
    }
}
