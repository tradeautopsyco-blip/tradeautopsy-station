import Foundation
import Testing
@testable import Station

@MainActor
struct OkxConnectLifecycleTests {
    private func metadataStore() -> UserDefaultsBrokerMetadataStore {
        UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.Okx.\(UUID().uuidString)")!
        )
    }

    @Test func enabledOkxCatalogRowMatchesAgent() {
        let okx = BrokerCatalog.descriptor(for: "okx_com")
        #expect(okx?.availability == .enabled)
        #expect(okx?.authScheme == .okxPassphraseSession)
        #expect(okx?.complianceProfileId == "okx_com_compliance")
        #expect(okx?.manifestId == "okx_com.s1.v1")
        #expect(okx?.bookId == "okx-com-spot")
    }

    @Test func emptyOkxFieldsFailLocalValidation() async {
        let runtime = FakeBrokerAgentRuntimeClient()
        let controller = BrokerConnectController(
            identity: .okxComProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "okx_com"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: runtime
        )
        controller.updateOkxFields(apiKey: "", apiSecret: "", passphrase: "")
        let outcome = await controller.connect()
        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .apiSecret, .passphrase]))
        #expect(runtime.vaultCredentialsPresentCallCount == 0)
    }

    @Test func fakeOkxKeySavesPassphraseVaultBlob() async throws {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let controller = BrokerConnectController(
            identity: .okxComProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "okx_com"),
            syncControl: sync,
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateOkxFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.readOnly.apiKey,
            apiSecret: "okx-secret",
            passphrase: "okx-pass"
        )
        let outcome = await controller.connect()
        #expect(outcome == .connected(permissionWarning: nil))
        #expect(sync.startSyncCallCount == 1)
        let saved = try #require(try store.read(for: .okxComProd))
        #expect(saved.authScheme == .okxPassphraseSession)
        #expect(saved.passphrase == "okx-pass")
        let data = try JSONEncoder().encode(saved)
        let json = try #require(String(data: data, encoding: .utf8))
        #expect(json.contains("okx_passphrase_session"))
        #expect(json.contains("passphrase"))
    }

    @Test func invalidFakeOkxKeyDoesNotPersist() async {
        let store = FakeBrokerCredentialStore()
        let controller = BrokerConnectController(
            identity: .okxComProd,
            credentialStore: store,
            validator: BrokerConnectServices.validator(for: "okx_com"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateOkxFields(
            apiKey: LocalPhase4CryptoCredentialValidator.Scenario.invalidCredentials.apiKey,
            apiSecret: "okx-secret",
            passphrase: "okx-pass"
        )
        let outcome = await controller.connect()
        #expect(outcome == .validationPermanentFailure(.invalidCredentials))
        #expect(store.hasCredentials(for: .okxComProd) == false)
    }

    @Test func whitespacePassphraseFailsLocalValidation() async {
        let controller = BrokerConnectController(
            identity: .okxComProd,
            credentialStore: FakeBrokerCredentialStore(),
            validator: BrokerConnectServices.validator(for: "okx_com"),
            syncControl: FakeBrokerSyncControl(),
            metadataStore: metadataStore(),
            runtimeClient: FakeBrokerAgentRuntimeClient()
        )
        controller.updateOkxFields(apiKey: "key", apiSecret: "secret", passphrase: "   ")
        let outcome = await controller.connect()
        #expect(outcome == .localValidationFailed(invalidFields: [.passphrase]))
    }

    @Test func catalogMarksOkxEnabledConnectBeta() {
        #expect(BrokerCatalog.descriptor(for: "okx_com")?.availability == .enabled)
        #expect(BrokerDogfoodProgram.showsConnectBetaBadge(slug: "okx_com"))
    }
}
