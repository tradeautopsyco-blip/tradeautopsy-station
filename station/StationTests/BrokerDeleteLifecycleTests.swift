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
            identity: .binanceComProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: syncControl,
            metadataStore: metadataStore,
            runtimeClient: runtimeClient,
            keychainItems: FakeBrokerKeychainItemStore()
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
            for: .binanceComProd
        )
        metadataStore.save(
            BrokerConnectionMetadata(
                permissionWarning: .tradeEnabled,
                lastValidatedAt: Date(),
                syncPaused: false
            ),
            for: .binanceComProd
        )

        try await client.deleteConnection(for: .binanceComProd, connectController: connectController)

        #expect(runtime.stopSyncCallCount == 1)
        #expect(runtime.clearVaultCredentialsCallCount == 1)
        #expect(store.hasCredentials(for: .binanceComProd) == false)
        #expect(store.deleteCallCount == 1)
        #expect(metadataStore.load(for: .binanceComProd) == nil)
        #expect(connectController.permissionWarning == nil)
    }

    @Test func offlineDeleteClearsKeychainDirectlyEvenWhenAgentClearFails() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.clearVaultError = BrokerAgentRuntimeError.requestFailed
        let defaults = UserDefaults(suiteName: "StationTests.Delete.Offline.\(UUID().uuidString)")!
        let store = FakeBrokerCredentialStore()
        let keychain = FakeBrokerKeychainItemStore()
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
        let identity = BrokerConnectionIdentity.binanceComProd
        let account =
            "\(identity.environment).\(identity.brokerSlug).\(identity.brokerConnectionID.uuidString)"
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: account
        )
        keychain.seed(
            service: KeychainBrokerCredentialStore.serviceName,
            account: account
        )
        let connectController = BrokerConnectController(
            identity: identity,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: syncControl,
            metadataStore: metadataStore,
            runtimeClient: runtime,
            keychainItems: keychain
        )
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: identity
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date()),
            for: identity
        )

        try await client.deleteConnection(for: identity, connectController: connectController)

        #expect(runtime.stopSyncCallCount == 0)
        #expect(runtime.clearVaultCredentialsCallCount == 1)
        #expect(store.hasCredentials(for: identity) == false)
        #expect(metadataStore.load(for: identity) == nil)
        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: account
            ) == false
        )
        #expect(
            keychain.hasItem(
                service: KeychainBrokerCredentialStore.serviceName,
                account: account
            ) == false
        )
    }

    @Test func loadSnapshotAfterDeleteShowsNotConfigured() async throws {
        let (client, store, _, _, connectController, supervisor) = await makeClient()
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceComProd
        )

        try await client.deleteConnection(for: .binanceComProd, connectController: connectController)
        let snapshot = await client.loadSnapshot()

        #expect(snapshot.configuredConnections.isEmpty)
        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }
        #expect(binance?.status == .notConfigured)
    }
}
