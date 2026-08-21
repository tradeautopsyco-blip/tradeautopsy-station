import Foundation
import Testing
@testable import Station

@MainActor
struct BrokerPauseLifecycleTests {
    private func makeClient(
        metadataDefaults: UserDefaults? = nil,
        runtimeClient: FakeBrokerAgentRuntimeClient = FakeBrokerAgentRuntimeClient()
    ) async -> (
        LocalBrokerControlClient,
        FakeBrokerCredentialStore,
        UserDefaultsBrokerMetadataStore,
        FakeBrokerAgentRuntimeClient,
        FakeAgentSupervisor
    ) {
        let defaults = metadataDefaults ?? UserDefaults(suiteName: "StationTests.Pause.\(UUID().uuidString)")!
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
        return (client, store, metadataStore, runtimeClient, supervisor)
    }

    @Test func stopSyncMarksMetadataPausedAndCallsAgent() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        let (client, store, metadataStore, _, supervisor) = await makeClient(runtimeClient: runtime)
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceComProd
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date()),
            for: .binanceComProd
        )

        try await client.stopSync(for: .binanceComProd)

        #expect(runtime.stopSyncCallCount == 1)
        #expect(metadataStore.load(for: .binanceComProd)?.syncPaused == true)
    }

    @Test func loadSnapshotShowsPausedWhenMetadataAndAgentAgree() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.runtimeStatus = .paused
        let (client, store, metadataStore, _, supervisor) = await makeClient(runtimeClient: runtime)
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceComProd
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date(), syncPaused: true),
            for: .binanceComProd
        )

        let snapshot = await client.loadSnapshot()
        let status = snapshot.runtimeStatusByConnectionID[
            BrokerConnectionIdentity.binanceComProd.brokerConnectionID.uuidString
        ]

        #expect(status == .paused)
        #expect(runtime.fetchSyncHealthCallCount == 1)
    }

    @Test func connectAfterManualPauseDoesNotAutoStart() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        let store = FakeBrokerCredentialStore()
        try store.save(
            credentials: BrokerCredentials(apiKey: "existing", apiSecret: "secret"),
            for: .binanceComProd
        )
        let defaults = UserDefaults(suiteName: "StationTests.Pause.Connect.\(UUID().uuidString)")!
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date(), syncPaused: true),
            for: .binanceComProd
        )
        let sync = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)
        let validator = FakeBrokerCredentialValidator()
        validator.nextResult = .success(permissionPosture: .readOnlyConfirmed)
        let controller = BrokerConnectController(
            identity: .binanceComProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        controller.updateFields(apiKey: "rotated-key", apiSecret: "rotated-secret")

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(runtime.startSyncCallCount == 0)
    }

    @Test func startSyncClearsPausedFlag() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        let (client, store, metadataStore, _, supervisor) = await makeClient(runtimeClient: runtime)
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceComProd
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date(), syncPaused: true),
            for: .binanceComProd
        )

        try await client.startSync(for: .binanceComProd)

        #expect(runtime.startSyncCallCount == 1)
        #expect(metadataStore.load(for: .binanceComProd)?.syncPaused == false)
    }

    @Test func loadSnapshotDoesNotReadKeychainSecretsWhenMetadataExists() async throws {
        let (client, store, metadataStore, _, supervisor) = await makeClient()
        _ = supervisor
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceComProd
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date()),
            for: .binanceComProd
        )

        _ = await client.loadSnapshot()

        #expect(store.readCallCount == 0)
        #expect(store.hasCredentialsCallCount == 0)
    }

    @Test func startSyncUsesPresenceNotSecretRead() async throws {
        let store = FakeBrokerCredentialStore()
        try store.save(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            for: .binanceComProd
        )
        let runtime = FakeBrokerAgentRuntimeClient()
        let sync = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)

        try await sync.startSync(for: .binanceComProd)

        #expect(store.readCallCount == 0)
        #expect(store.hasCredentialsCallCount == 1)
        #expect(runtime.startSyncCallCount == 1)
    }
}
