import Foundation
import Testing
@testable import Station

@MainActor
struct KrakenConnectLifecycleTests {
    private func metadataStore() -> UserDefaultsBrokerMetadataStore {
        UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.Kraken.\(UUID().uuidString)")!
        )
    }

    @Test func enabledKrakenCatalogRowMatchesAgent() {
        let kraken = BrokerCatalog.descriptor(for: "kraken")
        #expect(kraken?.availability == .enabled)
        #expect(kraken?.authScheme == .krakenSpotNonceSession)
        #expect(kraken?.complianceProfileId == "kraken_compliance")
        #expect(kraken?.manifestId == "kraken.spot.v1")
        #expect(kraken?.bookId == "kraken-com-spot")
    }

    @Test func emptyKrakenFieldsFailLocalValidation() async {
        let runtime = FakeBrokerAgentRuntimeClient()
        let controller = BrokerConnectController(
            identity: .krakenProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "kraken"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: runtime
        )
        controller.updateFields(apiKey: "", apiSecret: "")
        let outcome = await controller.connect()
        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .apiSecret]))
        #expect(runtime.vaultCredentialsPresentCallCount == 0)
    }

    @Test func fakeKrakenKeySavesKrakenSpotNonceBlob() async throws {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let controller = BrokerConnectController(
            identity: .krakenProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "kraken"),
            syncControl: sync,
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.readOnly.apiKey,
            apiSecret: "kraken-base64-secret"
        )
        let outcome = await controller.connect()
        #expect(outcome == .connected(permissionWarning: nil))
        #expect(sync.startSyncCallCount == 1)
        let saved = try #require(try store.read(for: .krakenProd))
        #expect(saved.authScheme == .krakenSpotNonceSession)
        let data = try JSONEncoder().encode(saved)
        let json = try #require(String(data: data, encoding: .utf8))
        #expect(json.contains("kraken_spot_nonce_session"))
        #expect(json.contains("apiSecret"))
    }

    @Test func withdrawPermissionBlocksKrakenSave() async {
        let store = FakeBrokerCredentialStore()
        let controller = BrokerConnectController(
            identity: .krakenProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "kraken"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.withdraw.apiKey,
            apiSecret: "kraken-secret"
        )
        let outcome = await controller.connect()
        #expect(outcome == .blockedWithdrawPermission)
        #expect(store.hasCredentials(for: .krakenProd) == false)
    }

    @Test func networkFailureDoesNotPersistKrakenCredentials() async {
        let store = FakeBrokerCredentialStore()
        let controller = BrokerConnectController(
            identity: .krakenProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "kraken"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.networkUnavailable.apiKey,
            apiSecret: "kraken-secret"
        )
        let outcome = await controller.connect()
        #expect(outcome == .validationTransientFailure(.networkUnavailable))
        #expect(store.hasCredentials(for: .krakenProd) == false)
    }

    @Test func catalogMarksKrakenEnabledConnectBeta() {
        #expect(BrokerCatalog.descriptor(for: "kraken")?.availability == .enabled)
        #expect(BrokerDogfoodProgram.showsConnectBetaBadge(slug: "kraken"))
    }
}
