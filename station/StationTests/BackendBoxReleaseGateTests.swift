import Foundation
import Testing
@testable import Station

/// Issue #20 — release-blocking proofs that must stay green in CI.
@MainActor
struct BackendBoxReleaseGateTests {
    @Test func withdrawPermissionHardBlockNeverPersistsCredentials() async {
        let (controller, store, validator, sync, _) = makeConnectController()
        validator.nextResult = .success(permissionPosture: .withdrawDetected)
        controller.updateFields(apiKey: "key", apiSecret: "secret")

        let outcome = await controller.connect()

        #expect(outcome == .blockedWithdrawPermission)
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func deleteRemovesKeychainCredentials() async throws {
        let store = FakeBrokerCredentialStore()
        let (_, _, validator, sync, defaults) = makeConnectController(credentialStore: store)
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let controller = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        validator.nextResult = .success(permissionPosture: .readOnlyConfirmed)
        controller.updateFields(apiKey: "key", apiSecret: "secret")
        _ = await controller.connect()

        try controller.deleteSavedCredentials()

        #expect(store.hasCredentials(for: .binanceUSProd) == false)
    }

    @Test func behavioralOptOutDisclosureDiffersFromDefault() {
        let defaultMessage = BrokerConnectDisclosure.message(behavioralAnalysisOptedOut: false)
        let optedOut = BrokerConnectDisclosure.message(behavioralAnalysisOptedOut: true)
        #expect(defaultMessage != optedOut)
        #expect(optedOut.localizedCaseInsensitiveContains("opted out"))
    }

    @Test func presentationAndCardModelsRejectSecretLeakage() {
        let credentials = BrokerCredentials(apiKey: "gate-key", apiSecret: "gate-secret")
        let presentation = BrokerConnectPresentation(
            identity: .binanceUSProd,
            status: "Syncing",
            permissionWarning: nil,
            behavioralAnalysisOptedOut: false
        )
        #expect(BrokerSecretGuard.presentationIsSafe(presentation, credentials: credentials))
        #expect(!BrokerSecretGuard.containsSecretMaterial("broker sync degraded", credentials: credentials))
    }

    private func makeConnectController(
        credentialStore: FakeBrokerCredentialStore = FakeBrokerCredentialStore()
    ) -> (
        BrokerConnectController,
        FakeBrokerCredentialStore,
        FakeBrokerCredentialValidator,
        FakeBrokerSyncControl,
        UserDefaults
    ) {
        let defaults = UserDefaults(suiteName: "StationTests.Gates.\(UUID().uuidString)")!
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let validator = FakeBrokerCredentialValidator()
        let sync = FakeBrokerSyncControl()
        let controller = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: credentialStore,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        return (controller, credentialStore, validator, sync, defaults)
    }
}
