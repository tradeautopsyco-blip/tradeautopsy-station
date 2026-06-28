import Foundation
@testable import Station

@MainActor
final class FakeAgentSupervisor: AgentSupervising {
    enum Scenario {
        case healthy
        case launchTimeout
        case portCollisionNonAgent
        case crashLoopExceeded
    }

    var scenario: Scenario = .healthy
    var onHealthChange: ((Bool) -> Void)?

    private(set) var startCallCount = 0
    private(set) var retryCallCount = 0
    private(set) var shutdownCallCount = 0

    private(set) var isHealthy = false
    private(set) var currentWarning: AgentHealthWarning?

    func start() async {
        startCallCount += 1
        applyScenario()
    }

    func retry() async {
        retryCallCount += 1
        if scenario == .crashLoopExceeded {
            scenario = .healthy
        }
        applyScenario()
    }

    func shutdown() async {
        shutdownCallCount += 1
    }

    private func applyScenario() {
        switch scenario {
        case .healthy:
            isHealthy = true
            currentWarning = nil
            onHealthChange?(true)
        case .launchTimeout:
            isHealthy = false
            currentWarning = AgentHealthWarning(
                reason: .launchTimeout,
                message: "Agent did not respond within 10 seconds.",
                logPath: nil,
                canRetry: true
            )
        case .portCollisionNonAgent:
            isHealthy = false
            currentWarning = AgentHealthWarning(
                reason: .portCollisionNonAgent,
                message: "Port 9137 is in use by a non-agent process.",
                logPath: nil,
                canRetry: true
            )
        case .crashLoopExceeded:
            isHealthy = false
            currentWarning = AgentHealthWarning(
                reason: .crashLoopExceeded,
                message: "Agent crashed repeatedly. Manual retry required.",
                logPath: nil,
                canRetry: true
            )
        }
    }
}
