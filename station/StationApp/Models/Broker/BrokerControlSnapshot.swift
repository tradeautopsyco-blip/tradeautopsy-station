import Foundation

/// Agent-reported broker control state presented by Station (PRD §Core State Ownership).
public struct BrokerControlSnapshot: Equatable, Sendable {
    public let configuredConnections: [BrokerConfiguredConnection]
    public let agentAvailable: Bool
    /// Runtime card status keyed by `broker_connection_id`; agent is source of truth.
    public let runtimeStatusByConnectionID: [String: BrokerCardStatus]

    public init(
        configuredConnections: [BrokerConfiguredConnection],
        agentAvailable: Bool,
        runtimeStatusByConnectionID: [String: BrokerCardStatus]
    ) {
        self.configuredConnections = configuredConnections
        self.agentAvailable = agentAvailable
        self.runtimeStatusByConnectionID = runtimeStatusByConnectionID
    }
}
