import Foundation
import Testing
@testable import Station

@MainActor
struct BybitConnectLifecycleTests {
    @Test func plannedBybitCatalogRowMatchesAgent() {
        let bybit = BrokerCatalog.descriptor(for: "bybit")
        #expect(bybit?.availability == .planned)
        #expect(bybit?.authScheme == .hmacApiKeySecret)
        #expect(bybit?.calcProfileId == "crypto_spot_usd")
        #expect(bybit?.complianceProfileId == "bybit_compliance")
        #expect(bybit?.manifestId == "bybit.spot.v1")
        #expect(bybit?.bookId == "bybit-com-spot")
    }

    @Test func emptyBybitFieldsFailLocalValidationWithoutValidator() async {
        let runtime = FakeBrokerAgentRuntimeClient()
        let controller = BrokerConnectController(
            identity: .bybitProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "bybit"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: UserDefaultsBrokerMetadataStore(
                defaults: UserDefaults(suiteName: "StationTests.Bybit.\(UUID().uuidString)")!
            ),
            runtimeClient: runtime
        )
        controller.updateFields(apiKey: "", apiSecret: "")
        let outcome = await controller.connect()
        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .apiSecret]))
        #expect(runtime.vaultCredentialsPresentCallCount == 0)
    }

    @Test func fakeBybitKeySavesHmacVaultBlobAndStartsSync() async throws {
        let runtime = FakeBrokerAgentRuntimeClient()
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let controller = BrokerConnectController(
            identity: .bybitProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "bybit"),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(
                defaults: UserDefaults(suiteName: "StationTests.Bybit.\(UUID().uuidString)")!
            ),
            runtimeClient: runtime
        )
        controller.updateFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.readOnly.apiKey,
            apiSecret: "bybit-secret"
        )
        let outcome = await controller.connect()
        #expect(outcome == .connected(permissionWarning: nil))
        #expect(sync.startSyncCallCount == 1)
        let saved = try #require(try store.read(for: .bybitProd))
        #expect(saved.authScheme == .hmacApiKeySecret)
        let data = try JSONEncoder().encode(saved)
        let json = try #require(String(data: data, encoding: .utf8))
        #expect(json.contains("hmac_api_key_secret"))
        #expect(json.contains("apiKey"))
        #expect(json.contains("apiSecret"))
    }

    @Test func withdrawPermissionBlocksBybitSave() async {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let controller = BrokerConnectController(
            identity: .bybitProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "bybit"),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(
                defaults: UserDefaults(suiteName: "StationTests.Bybit.\(UUID().uuidString)")!
            ),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.withdraw.apiKey,
            apiSecret: "bybit-secret"
        )
        let outcome = await controller.connect()
        #expect(outcome == .blockedWithdrawPermission)
        #expect(store.hasCredentials(for: .bybitProd) == false)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func invalidFakeBybitKeyMapsToPermanentFailure() async {
        let controller = BrokerConnectController(
            identity: .bybitProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "bybit"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: UserDefaultsBrokerMetadataStore(
                defaults: UserDefaults(suiteName: "StationTests.Bybit.\(UUID().uuidString)")!
            ),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.invalidCredentials.apiKey,
            apiSecret: "bybit-secret"
        )
        let outcome = await controller.connect()
        #expect(outcome == .validationPermanentFailure(.invalidCredentials))
    }

    @Test func dogfoodAllowsConnectWhilePlanned() {
        #expect(BrokerDogfoodProgram.allowsConnectWhilePlanned(slug: "bybit"))
    }
}
