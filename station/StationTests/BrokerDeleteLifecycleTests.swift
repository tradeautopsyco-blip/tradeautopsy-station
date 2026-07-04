import Foundation
import Testing
@testable import Station

@MainActor
struct BrokerDeleteLifecycleTests {
    private func makeClient(
        runtimeClient: FakeBrokerAgentRuntimeClient = FakeBrokerAgentRuntimeClient()
    ) async -> (
        LocalBrokerControlClient,
        FakeBrokerCredentialStore,
        UserDefaultsBrokerMetadataStore,
        FakeBrokerAgentRuntimeClient,
        BrokerConnectController,
        FakeAgentSupervisor
    ) {
        let defaults = UserDefaults(suiteName: "StationTests.Delete.\(UUID().uuidString)")!
        let store = FakeBrokerCredentialStore()
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let supervisor = FakeAgentSupervisor()
        supervisor.scenario = .healthy
        await supervisor.start()
        let syncControl = AgentBrokerSyncControl(
            credentialStore: store,
            runtimeClient: runtimeClient
        )
        let client = LocalBrokerControlClient(
            agentSupervisor: supervisor,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: syncControl,
            runtimeClient: runtimeClient
        )
        let connectController = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: syncControl,
            metadataStore: metadataStore
        )
        return (client, store, metadataStore, runtimeClient, connectController, supervisor)
    }

    @Test func deleteConfirmationCopyMentionsKeychainAndSyncStop() {
        let copy = BrokerDeleteConfirmation.message
        #expect(copy.localizedCaseInsensitiveContains("keychain"))
        #expect(copy.localizedCaseInsensitiveContains("stop"))
    }

    @Test func deleteStopsSyncBeforeRemovingCredentials() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        let (client, store, metadataStore, _, connectController, supervisor) = await makeClient(runtimeClient: runtime)
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceUSProd
        )
        metadataStore.save(
            BrokerConnectionMetadata(
                permissionWarning: .tradeEnabled,
                lastValidatedAt: Date(),
                syncPaused: false
            ),
            for: .binanceUSProd
        )

        try await client.deleteConnection(for: .binanceUSProd, connectController: connectController)

        #expect(runtime.stopSyncCallCount == 1)
        #expect(store.hasCredentials(for: .binanceUSProd) == false)
        #expect(store.deleteCallCount == 1)
        #expect(metadataStore.load(for: .binanceUSProd) == nil)
        #expect(connectController.permissionWarning == nil)
    }

    @Test func deleteAvailableWhileAgentOffline() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        let defaults = UserDefaults(suiteName: "StationTests.Delete.Offline.\(UUID().uuidString)")!
        let store = FakeBrokerCredentialStore()
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let supervisor = FakeAgentSupervisor()
        supervisor.scenario = .launchTimeout
        await supervisor.start()
        let syncControl = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)
        let client = LocalBrokerControlClient(
            agentSupervisor: supervisor,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: syncControl,
            runtimeClient: runtime
        )
        let connectController = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: syncControl,
            metadataStore: metadataStore
        )
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceUSProd
        )

        try await client.deleteConnection(for: .binanceUSProd, connectController: connectController)

        #expect(runtime.stopSyncCallCount == 0)
        #expect(store.hasCredentials(for: .binanceUSProd) == false)
    }

    @Test func loadSnapshotAfterDeleteShowsNotConfigured() async throws {
        let (client, store, _, _, connectController, supervisor) = await makeClient()
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceUSProd
        )

        try await client.deleteConnection(for: .binanceUSProd, connectController: connectController)
        let snapshot = await client.loadSnapshot()

        #expect(snapshot.configuredConnections.isEmpty)
        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_us" }
        #expect(binance?.status == .notConfigured)
    }
}
