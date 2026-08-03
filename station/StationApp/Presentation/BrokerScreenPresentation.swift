import Foundation

public struct BrokerCardPresentation: Equatable, Identifiable, Sendable {
    public let id: String
    public let displayName: String
    public let assetClass: String
    public let quoteCurrency: String
    public let status: BrokerCardStatus
    public let statusLabel: String
    public let isConnectable: Bool
    public let isEditEnabled: Bool
    public let isStartEnabled: Bool
    public let isStopEnabled: Bool
    public let isDeleteEnabled: Bool
    public let lastValidatedAtText: String?
    public let lastSyncSummary: String?
    /// Relative "Last synced" from agent poll success (nil when idle / other slug).
    public let lastSyncedAtText: String?
    public let plannedLabel: String?
    public let permissionWarning: BrokerPermissionWarning?
    public let identity: BrokerConnectionIdentity?

    public init(
        id: String,
        displayName: String,
        assetClass: String,
        quoteCurrency: String,
        status: BrokerCardStatus,
        statusLabel: String,
        isConnectable: Bool,
        isEditEnabled: Bool = false,
        isStartEnabled: Bool,
        isStopEnabled: Bool,
        isDeleteEnabled: Bool,
        lastValidatedAtText: String?,
        lastSyncSummary: String?,
        lastSyncedAtText: String? = nil,
        plannedLabel: String?,
        permissionWarning: BrokerPermissionWarning? = nil,
        identity: BrokerConnectionIdentity?
    ) {
        self.id = id
        self.displayName = displayName
        self.assetClass = assetClass
        self.quoteCurrency = quoteCurrency
        self.status = status
        self.statusLabel = statusLabel
        self.isConnectable = isConnectable
        self.isEditEnabled = isEditEnabled
        self.isStartEnabled = isStartEnabled
        self.isStopEnabled = isStopEnabled
        self.isDeleteEnabled = isDeleteEnabled
        self.lastValidatedAtText = lastValidatedAtText
        self.lastSyncSummary = lastSyncSummary
        self.lastSyncedAtText = lastSyncedAtText
        self.plannedLabel = plannedLabel
        self.permissionWarning = permissionWarning
        self.identity = identity
    }
}

public enum BrokerScreenPresentation {
    public static func build(
        snapshot: BrokerControlSnapshot,
        catalog: [PlannedBrokerDescriptor],
        now: Date = Date()
    ) -> [BrokerCardPresentation] {
        catalog.map { descriptor in
            buildCard(descriptor: descriptor, snapshot: snapshot, now: now)
        }
    }

    private static func buildCard(
        descriptor: PlannedBrokerDescriptor,
        snapshot: BrokerControlSnapshot,
        now: Date
    ) -> BrokerCardPresentation {
        if descriptor.availability == .planned {
            return BrokerCardPresentation(
                id: descriptor.slug,
                displayName: descriptor.displayName,
                assetClass: descriptor.assetClass,
                quoteCurrency: descriptor.quoteCurrency,
                status: .notConfigured,
                statusLabel: "Planned",
                isConnectable: false,
                isStartEnabled: false,
                isStopEnabled: false,
                isDeleteEnabled: false,
                lastValidatedAtText: nil,
                lastSyncSummary: nil,
                lastSyncedAtText: nil,
                plannedLabel: "Planned",
                permissionWarning: nil,
                identity: nil
            )
        }

        if descriptor.availability == .parked {
            return BrokerCardPresentation(
                id: descriptor.slug,
                displayName: descriptor.displayName,
                assetClass: descriptor.assetClass,
                quoteCurrency: descriptor.quoteCurrency,
                status: .notConfigured,
                statusLabel: "Parked",
                isConnectable: false,
                isStartEnabled: false,
                isStopEnabled: false,
                isDeleteEnabled: false,
                lastValidatedAtText: nil,
                lastSyncSummary: nil,
                lastSyncedAtText: nil,
                plannedLabel: "Parked",
                permissionWarning: nil,
                identity: nil
            )
        }

        let connection = snapshot.configuredConnections.first {
            $0.identity.brokerSlug == descriptor.slug
        }
        let status = resolveStatus(
            connection: connection,
            agentAvailable: snapshot.agentAvailable,
            agentReportedStatus: connection.flatMap {
                snapshot.runtimeStatusByConnectionID[$0.identity.brokerConnectionID.uuidString]
            }
        )
        let controls = resolveControls(status: status, connection: connection, agentAvailable: snapshot.agentAvailable)

        return BrokerCardPresentation(
            id: descriptor.slug,
            displayName: descriptor.displayName,
            assetClass: descriptor.assetClass,
            quoteCurrency: descriptor.quoteCurrency,
            status: status,
            statusLabel: status.rawValue,
            isConnectable: connection == nil && snapshot.agentAvailable,
            isEditEnabled: connection != nil,
            isStartEnabled: controls.start,
            isStopEnabled: controls.stop,
            isDeleteEnabled: connection != nil,
            lastValidatedAtText: formatValidatedAt(connection?.lastValidatedAt, now: now),
            lastSyncSummary: connection?.lastSyncSummary,
            lastSyncedAtText: formatRelativeSyncedAt(connection?.lastSyncedAtMs, now: now),
            plannedLabel: nil,
            permissionWarning: connection?.permissionWarning,
            identity: connection?.identity
        )
    }

    static func resolveStatus(
        connection: BrokerConfiguredConnection?,
        agentAvailable: Bool,
        agentReportedStatus: BrokerCardStatus?
    ) -> BrokerCardStatus {
        guard connection != nil else {
            return .notConfigured
        }
        guard agentAvailable else {
            return .unavailableAgentOffline
        }
        if let agentReportedStatus {
            return agentReportedStatus
        }
        return .readyToStart
    }

    private static func resolveControls(
        status: BrokerCardStatus,
        connection: BrokerConfiguredConnection?,
        agentAvailable: Bool
    ) -> (start: Bool, stop: Bool) {
        guard connection != nil, agentAvailable else {
            return (false, false)
        }
        switch status {
        case .readyToStart, .paused, .failed:
            // Idle / paused — Start only.
            return (true, false)
        case .connected, .syncing, .degraded, .rateLimited:
            // Active poll (healthy or partial) — Stop must stay available; Start restarts.
            return (true, true)
        default:
            return (false, false)
        }
    }

    static func formatValidatedAt(_ date: Date?, now: Date) -> String? {
        guard let date else { return nil }
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }

    /// Relative age for Brokers "Last synced" (agent epoch ms).
    public static func formatRelativeSyncedAt(_ epochMs: Int64?, now: Date = Date()) -> String? {
        guard let epochMs else { return nil }
        let synced = Date(timeIntervalSince1970: TimeInterval(epochMs) / 1000.0)
        let seconds = max(0, Int(now.timeIntervalSince(synced)))
        if seconds < 5 {
            return "just now"
        }
        if seconds < 60 {
            return "\(seconds)s ago"
        }
        let minutes = seconds / 60
        if minutes < 60 {
            return "\(minutes)m ago"
        }
        let hours = minutes / 60
        if hours < 48 {
            return "\(hours)h ago"
        }
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: synced)
    }
}
