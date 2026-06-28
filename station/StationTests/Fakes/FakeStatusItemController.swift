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
}
