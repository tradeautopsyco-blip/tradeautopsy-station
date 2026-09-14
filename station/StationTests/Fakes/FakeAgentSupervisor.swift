import Foundation
@testable import Station

@MainActor
final class FakeAgentSupervisor: AgentSupervising {
    enum Scenario {
        case healthy
        case attachedHealthy
        case launchTimeout
        case portCollisionNonAgent
        case crashLoopExceeded
        case runtimeDisconnected
    }

    var scenario: Scenario = .healthy
    var onHealthChange: ((Bool) -> Void)?

    private(set) var startCallCount = 0
    private(set) var retryCallCount = 0
    private(set) var shutdownCallCount = 0
    private(set) var ownsSpawnedAgent = false

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
        if ownsSpawnedAgent {
            ownsSpawnedAgent = false
        }
    }

    func simulateRuntimeDisconnect() {
        guard isHealthy else { return }
        isHealthy = false
        currentWarning = AgentHealthWarning(
            reason: .runtimeDisconnected,
            message: "Agent disconnected during session. Data may be stale until reconnected.",
            logPath: nil,
            canRetry: true
        )
        onHealthChange?(false)
    }

    func simulateRuntimeRecovery() {
        guard !isHealthy else { return }
        isHealthy = true
        currentWarning = nil
        onHealthChange?(true)
    }

    private func applyScenario() {
        switch scenario {
        case .healthy:
            isHealthy = true
            currentWarning = nil
            ownsSpawnedAgent = true
            onHealthChange?(true)
        case .attachedHealthy:
            isHealthy = true
            currentWarning = nil
            ownsSpawnedAgent = false
            onHealthChange?(true)
        case .launchTimeout:
            isHealthy = false
            ownsSpawnedAgent = false
            currentWarning = AgentHealthWarning(
                reason: .launchTimeout,
                message: "Agent did not respond within 30 seconds.",
                logPath: nil,
                canRetry: true
            )
        case .portCollisionNonAgent:
            isHealthy = false
            ownsSpawnedAgent = false
            currentWarning = AgentHealthWarning(
                reason: .portCollisionNonAgent,
                message: "Port 9137 is in use by a non-agent process. Check DAEMON_PORT and free the port.",
                logPath: nil,
                canRetry: true
            )
        case .crashLoopExceeded:
            isHealthy = false
            ownsSpawnedAgent = false
            currentWarning = AgentHealthWarning(
                reason: .crashLoopExceeded,
                message: "Agent crashed repeatedly. Manual retry required.",
                logPath: nil,
                canRetry: true
            )
        case .runtimeDisconnected:
            isHealthy = false
            ownsSpawnedAgent = false
            currentWarning = AgentHealthWarning(
                reason: .runtimeDisconnected,
                message: "Agent disconnected during session.",
                logPath: nil,
                canRetry: true
            )
        }
    }
}
