import Foundation
import Testing
@testable import Station

@MainActor
struct GrowwConnectLifecycleTests {
    @Test func connectPathMatchesContract() {
        #expect(GrowwConnectContract.connectPath == "/api/daemon/broker/groww/connect")
        #expect(GrowwConnectContract.requiresBrowser == false)
        #expect(GrowwConnectContract.hasLoopbackCallback == false)
    }

    @Test func emptyGrowwFieldsFailLocalValidationWithoutConnect() async {
        let runtime = FakeBrokerAgentRuntimeClient()
        let store = MemoryBrokerCredentialStore()
        let suite = "StationTests.Groww.\(UUID().uuidString)"
        let identity = BrokerConnectionIdentity(
            brokerConnectionID: UUID(),
            brokerSlug: "groww",
            assetClass: "equities",
            environment: suite
        )
        let controller = BrokerConnectController(
            identity: identity,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "groww"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: MemoryBrokerConnectionMetadataStore(),
            runtimeClient: runtime
        )
        controller.updateFields(apiKey: "", apiSecret: "")
        let outcome = await controller.connect()
        guard case .localValidationFailed = outcome else {
            Issue.record("expected local validation failure, got \(outcome)")
            return
        }
        #expect(runtime.connectGrowwCallCount == 0)
    }

    @Test func keyEntryMintThenVaultPresenceStartsSync() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.credentialStore = MemoryBrokerCredentialStore()
        runtime.simulateGrowwVaultAfterConnect = true
        let store = MemoryBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Groww.\(UUID().uuidString)"
        let identity = BrokerConnectionIdentity(
            brokerConnectionID: UUID(),
            brokerSlug: "groww",
            assetClass: "equities",
            environment: suite
        )
        let controller = BrokerConnectController(
            identity: identity,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "groww"),
            syncControl: sync,
            metadataStore: MemoryBrokerConnectionMetadataStore(),
            runtimeClient: runtime
        )
        controller.updateFields(apiKey: "groww-api-key", apiSecret: "groww-api-secret")
        let outcome = await controller.connect()
        guard case .connected = outcome else {
            Issue.record("expected connected, got \(outcome)")
            return
        }
        #expect(runtime.connectGrowwCallCount == 1)
        #expect(sync.startSyncCallCount == 1)
    }

    @Test func plannedGrowwCatalogRowIsNotEnabled() {
        let groww = BrokerCatalog.descriptor(for: "groww")
        #expect(groww?.availability == .enabled)
        #expect(groww?.authScheme == .growwChecksumSession)
        #expect(groww?.bookId == "groww-nse-bse-cash")
    }
}
