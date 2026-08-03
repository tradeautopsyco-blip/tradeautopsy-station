import Foundation

/// Saved broker connection surfaced on the Brokers screen (non-secret metadata only).
public struct BrokerConfiguredConnection: Equatable, Sendable {
    public let identity: BrokerConnectionIdentity
    public let displayName: String
    public let permissionWarning: BrokerPermissionWarning?
    public let lastValidatedAt: Date?
    public let lastSyncSummary: String?
    /// Agent `lastSuccessAtMs` (fallback `lastPollAtMs`) for the active sync slug.
    public let lastSyncedAtMs: Int64?

    public init(
        identity: BrokerConnectionIdentity,
        displayName: String,
        permissionWarning: BrokerPermissionWarning? = nil,
        lastValidatedAt: Date? = nil,
        lastSyncSummary: String? = nil,
        lastSyncedAtMs: Int64? = nil
    ) {
        self.identity = identity
        self.displayName = displayName
        self.permissionWarning = permissionWarning
        self.lastValidatedAt = lastValidatedAt
        self.lastSyncSummary = lastSyncSummary
        self.lastSyncedAtMs = lastSyncedAtMs
    }
}
