import Foundation

@MainActor
public final class LocalBrokerControlClient: BrokerControlling {
    private struct V1BrokerEntry {
        let slug: String
        let displayName: String
        let identity: (TradeAutopsyEnvironment) -> BrokerConnectionIdentity
    }

    private static let v1Brokers: [V1BrokerEntry] = [
        V1BrokerEntry(
            slug: "binance_com",
            displayName: "Binance.com",
            identity: BrokerConnectionIdentity.binanceCom
        ),
        V1BrokerEntry(
            slug: "kotak_neo",
            displayName: "Kotak Neo",
            identity: BrokerConnectionIdentity.kotakNeo
        ),
    ]

    private weak var agentSupervisor: AgentSupervising?
    private let credentialStore: BrokerCredentialStoring
    private let metadataStore: BrokerConnectionMetadataStoring
    private let syncControl: BrokerSyncControlling
    private let runtimeClient: BrokerAgentRuntimeClient
    private let environmentStore: BrokerEnvironmentStoring

    public init(
        agentSupervisor: AgentSupervising,
        credentialStore: BrokerCredentialStoring,
        metadataStore: BrokerConnectionMetadataStoring,
        syncControl: BrokerSyncControlling,
        runtimeClient: BrokerAgentRuntimeClient = LocalAgentBrokerRuntimeClient(),
        environmentStore: BrokerEnvironmentStoring = UserDefaultsBrokerEnvironmentStore()
    ) {
        self.agentSupervisor = agentSupervisor
        self.credentialStore = credentialStore
        self.metadataStore = metadataStore
        self.syncControl = syncControl
        self.runtimeClient = runtimeClient
        self.environmentStore = environmentStore
    }

    public func loadSnapshot() async -> BrokerControlSnapshot {
        let agentAvailable = agentSupervisor?.isHealthy ?? false
        var configured: [BrokerConfiguredConnection] = []
        var runtimeStatusByConnectionID: [String: BrokerCardStatus] = [:]
        let environment = environmentStore.loadActiveEnvironment()

        for entry in Self.v1Brokers {
            let identity = entry.identity(environment)
            guard credentialStore.hasCredentials(for: identity) else { continue }

            let metadata = metadataStore.load(for: identity)
            configured.append(
                BrokerConfiguredConnection(
                    identity: identity,
                    displayName: entry.displayName,
                    permissionWarning: metadata?.permissionWarning,
                    lastValidatedAt: metadata?.lastValidatedAt,
                    lastSyncSummary: nil
                )
            )

            if agentAvailable {
                if metadata?.syncPaused == true {
                    runtimeStatusByConnectionID[
                        identity.brokerConnectionID.uuidString
                    ] = .paused
                } else if let agentStatus = await runtimeClient.fetchRuntimeStatus(for: identity) {
                    runtimeStatusByConnectionID[
                        identity.brokerConnectionID.uuidString
                    ] = agentStatus
                }
            }
        }

        return BrokerControlSnapshot(
            configuredConnections: configured,
            agentAvailable: agentAvailable,
            runtimeStatusByConnectionID: runtimeStatusByConnectionID
        )
    }

    public func startSync(for identity: BrokerConnectionIdentity) async throws {
        try await syncControl.startSync(for: identity)
        if var metadata = metadataStore.load(for: identity) {
            metadata.syncPaused = false
            metadataStore.save(metadata, for: identity)
        }
    }

    public func stopSync(for identity: BrokerConnectionIdentity) async throws {
        try await runtimeClient.stopSync(for: identity)
        var metadata = metadataStore.load(for: identity) ?? BrokerConnectionMetadata()
        metadata.syncPaused = true
        metadataStore.save(metadata, for: identity)
    }

    public func deleteConnection(
        for identity: BrokerConnectionIdentity,
        connectController: BrokerConnectController
    ) async throws {
        if agentSupervisor?.isHealthy == true {
            try await runtimeClient.stopSync(for: identity)
        }
        try connectController.deleteSavedCredentials()
    }
}
