import Foundation
import Testing
@testable import Station

@MainActor
struct BrokerEnvironmentTests {
    @Test func productionBuildHidesEnvironmentSwitchingByDefault() {
        #if DEBUG
        #expect(InternalBuildGate.environmentSwitchingEnabled() == true)
        #else
        #expect(
            InternalBuildGate.environmentSwitchingEnabled(processInfo: [:]) == false
        )
        #expect(
            InternalBuildGate.environmentSwitchingEnabled(processInfo: ["STATION_INTERNAL": "1"]) == true
        )
        #endif
    }

    @Test func credentialsAreIsolatedPerEnvironment() throws {
        let store = FakeBrokerCredentialStore()
        try store.save(
            credentials: BrokerCredentials(apiKey: "prod-key", apiSecret: "prod-secret"),
            for: .binanceUSProd
        )
        try store.save(
            credentials: BrokerCredentials(apiKey: "staging-key", apiSecret: "staging-secret"),
            for: .binanceUS(.staging)
        )

        let prod = try store.read(for: .binanceUSProd)
        let staging = try store.read(for: .binanceUS(.staging))

        #expect(prod?.apiKey == "prod-key")
        #expect(staging?.apiKey == "staging-key")
        #expect(store.hasCredentials(for: .binanceUS(.dev)) == false)
    }

    @Test func environmentSwitchStopsSyncAndClearsRuntimeMetadata() async throws {
        let defaults = UserDefaults(suiteName: "StationTests.Env.\(UUID().uuidString)")!
        let store = FakeBrokerCredentialStore()
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let environmentStore = UserDefaultsBrokerEnvironmentStore(defaults: defaults)
        environmentStore.saveActiveEnvironment(.prod)
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.runtimeStatus = .syncing
        let supervisor = FakeAgentSupervisor()
        await supervisor.start()
        let syncControl = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)
        let brokerControl = LocalBrokerControlClient(
            agentSupervisor: supervisor,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: syncControl,
            runtimeClient: runtime,
            environmentStore: environmentStore
        )
        let connectController = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: syncControl,
            metadataStore: metadataStore
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date(), syncPaused: false),
            for: .binanceUSProd
        )

        let controller = BrokerEnvironmentController(
            environmentStore: environmentStore,
            brokerControl: brokerControl,
            metadataStore: metadataStore,
            runtimeClient: runtime,
            switchingEnabled: true
        )

        try await controller.switchEnvironment(
            to: .staging,
            connectController: connectController,
            syncWasRunning: true
        )

        #expect(runtime.stopSyncCallCount == 1)
        #expect(metadataStore.load(for: .binanceUSProd) == nil)
        #expect(controller.activeEnvironment == .staging)
        #expect(environmentStore.loadActiveEnvironment() == .staging)
    }
}
