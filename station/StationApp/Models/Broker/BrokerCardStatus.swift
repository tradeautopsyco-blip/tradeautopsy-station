import Foundation

/// v1 broker card status taxonomy (PRD Backend Box §Broker Card Status).
public enum BrokerCardStatus: String, Equatable, Sendable {
    case notConfigured = "Not configured"
    case validating = "Validating"
    case readyToStart = "Ready to Start"
    /// Healthy live sync (`syncState == synced`).
    case connected = "Connected"
    case syncing = "Syncing"
    case degraded = "Degraded"
    case rateLimited = "Rate Limited"
    case paused = "Paused"
    case unavailableAgentOffline = "Unavailable: Agent Offline"
    case failed = "Failed"

    /// Statuses that imply an active broker sync session reported by the agent.
    public var impliesAgentConnectedSync: Bool {
        switch self {
        case .connected, .syncing, .degraded, .rateLimited:
            return true
        default:
            return false
        }
    }
}
