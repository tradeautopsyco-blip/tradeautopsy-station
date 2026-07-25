import Foundation

public struct BrokerCardPresentation: Equatable, Identifiable, Sendable {
    public let id: String
    public let displayName: String
    public let assetClass: String
    public let status: BrokerCardStatus
    public let statusLabel: String
    public let isConnectable: Bool
    public let isStartEnabled: Bool
    public let isStopEnabled: Bool
    public let isDeleteEnabled: Bool
    public let lastValidatedAtText: String?
    public let lastSyncSummary: String?
    public let plannedLabel: String?
    public let permissionWarning: BrokerPermissionWarning?
    public let identity: BrokerConnectionIdentity?

    public init(
        id: String,
        displayName: String,
        assetClass: String,
        status: BrokerCardStatus,
        statusLabel: String,
        isConnectable: Bool,
        isStartEnabled: Bool,
        isStopEnabled: Bool,
        isDeleteEnabled: Bool,
        lastValidatedAtText: String?,
        lastSyncSummary: String?,
        plannedLabel: String?,
        permissionWarning: BrokerPermissionWarning? = nil,
        identity: BrokerConnectionIdentity?
    ) {
        self.id = id
        self.displayName = displayName
        self.assetClass = assetClass
        self.status = status
        self.statusLabel = statusLabel
        self.isConnectable = isConnectable
        self.isStartEnabled = isStartEnabled
        self.isStopEnabled = isStopEnabled
        self.isDeleteEnabled = isDeleteEnabled
        self.lastValidatedAtText = lastValidatedAtText
        self.lastSyncSummary = lastSyncSummary
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
                status: .notConfigured,
                statusLabel: "Planned",
                isConnectable: false,
                isStartEnabled: false,
                isStopEnabled: false,
                isDeleteEnabled: false,
                lastValidatedAtText: nil,
                lastSyncSummary: nil,
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
                status: .notConfigured,
                statusLabel: "Parked",
                isConnectable: false,
                isStartEnabled: false,
                isStopEnabled: false,
                isDeleteEnabled: false,
                lastValidatedAtText: nil,
                lastSyncSummary: nil,
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
            status: status,
            statusLabel: status.rawValue,
            isConnectable: connection == nil && snapshot.agentAvailable,
            isStartEnabled: controls.start,
            isStopEnabled: controls.stop,
            isDeleteEnabled: connection != nil,
            lastValidatedAtText: formatValidatedAt(connection?.lastValidatedAt, now: now),
            lastSyncSummary: connection?.lastSyncSummary,
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
        case .readyToStart, .paused, .degraded, .rateLimited, .failed:
            return (true, false)
        case .syncing:
            return (false, true)
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
}
