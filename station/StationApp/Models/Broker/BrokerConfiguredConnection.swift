import Foundation

/// Saved broker connection surfaced on the Brokers screen (non-secret metadata only).
public struct BrokerConfiguredConnection: Equatable, Sendable {
    public let identity: BrokerConnectionIdentity
    public let displayName: String
    public let permissionWarning: BrokerPermissionWarning?
    public let lastValidatedAt: Date?
    public let lastSyncSummary: String?

    public init(
        identity: BrokerConnectionIdentity,
        displayName: String,
        permissionWarning: BrokerPermissionWarning? = nil,
        lastValidatedAt: Date? = nil,
        lastSyncSummary: String? = nil
    ) {
        self.identity = identity
        self.displayName = displayName
        self.permissionWarning = permissionWarning
        self.lastValidatedAt = lastValidatedAt
        self.lastSyncSummary = lastSyncSummary
    }
}
