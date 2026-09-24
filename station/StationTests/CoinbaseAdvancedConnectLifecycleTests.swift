import Foundation
import Testing
@testable import Station

@MainActor
struct CoinbaseAdvancedConnectLifecycleTests {
    private let samplePEM = """
    -----BEGIN EC PRIVATE KEY-----
    MHQCAQEEIPlaceholderKeyMaterialForTestsOnlyNotReal==
    -----END EC PRIVATE KEY-----
    """

    private func metadataStore() -> UserDefaultsBrokerMetadataStore {
        UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.Coinbase.\(UUID().uuidString)")!
        )
    }

    @Test func enabledCoinbaseCatalogRowMatchesAgent() {
        let coinbase = BrokerCatalog.descriptor(for: "coinbase_advanced")
        #expect(coinbase?.availability == .enabled)
        #expect(coinbase?.authScheme == .coinbaseJwtEs256Session)
        #expect(coinbase?.complianceProfileId == "coinbase_advanced_compliance")
        #expect(coinbase?.manifestId == "tradeautopsy:coinbase-advanced-spot@0.1.0")
        #expect(coinbase?.bookId == "coinbase-advanced-spot")
    }

    @Test func emptyCoinbaseFieldsFailLocalValidation() async {
        let runtime = FakeBrokerAgentRuntimeClient()
        let controller = BrokerConnectController(
            identity: .coinbaseAdvancedProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "coinbase_advanced"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: runtime
        )
        controller.updateCoinbaseFields(apiKey: "", pemPrivateKey: "")
        let outcome = await controller.connect()
        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .pemPrivateKey]))
        #expect(runtime.vaultCredentialsPresentCallCount == 0)
    }

    @Test func malformedPemFailsLocalValidation() async {
        let controller = BrokerConnectController(
            identity: .coinbaseAdvancedProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "coinbase_advanced"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateCoinbaseFields(apiKey: "organizations/foo/apiKeys/bar", pemPrivateKey: "not-a-pem")
        let outcome = await controller.connect()
        #expect(outcome == .localValidationFailed(invalidFields: [.pemPrivateKey]))
    }

    @Test func fakeCoinbaseKeySavesJwtVaultBlob() async throws {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let controller = BrokerConnectController(
            identity: .coinbaseAdvancedProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "coinbase_advanced"),
            syncControl: sync,
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateCoinbaseFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.readOnly.apiKey,
            pemPrivateKey: samplePEM
        )
        let outcome = await controller.connect()
        #expect(outcome == .connected(permissionWarning: nil))
        #expect(sync.startSyncCallCount == 1)
        let saved = try #require(try store.read(for: .coinbaseAdvancedProd))
        #expect(saved.authScheme == .coinbaseJwtEs256Session)
        #expect(saved.pemPrivateKey?.contains("BEGIN EC PRIVATE KEY") == true)
        let data = try JSONEncoder().encode(saved)
        let json = try #require(String(data: data, encoding: .utf8))
        #expect(json.contains("coinbase_jwt_es256_session"))
        #expect(json.contains("pemPrivateKey"))
    }

    @Test func invalidFakeCoinbaseKeyDoesNotPersist() async {
        let store = FakeBrokerCredentialStore()
        let controller = BrokerConnectController(
            identity: .coinbaseAdvancedProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "coinbase_advanced"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateCoinbaseFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.invalidCredentials.apiKey,
            pemPrivateKey: samplePEM
        )
        let outcome = await controller.connect()
        #expect(outcome == .validationPermanentFailure(.invalidCredentials))
        #expect(store.hasCredentials(for: .coinbaseAdvancedProd) == false)
    }

    @Test func catalogMarksCoinbaseAdvancedEnabled() {
        #expect(BrokerCatalog.descriptor(for: "coinbase_advanced")?.availability == .enabled)
    }
}
