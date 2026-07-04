import Foundation
import Testing
@testable import Station

@MainActor
struct BrokerCredentialLifecycleTests {
    private func makeController(
        credentialStore: FakeBrokerCredentialStore = FakeBrokerCredentialStore(),
        validator: FakeBrokerCredentialValidator = FakeBrokerCredentialValidator(),
        syncControl: FakeBrokerSyncControl = FakeBrokerSyncControl(),
        metadataDefaults: UserDefaults? = nil,
        behavioralAnalysisOptedOut: Bool = false
    ) -> (
        BrokerConnectController,
        FakeBrokerCredentialStore,
        FakeBrokerCredentialValidator,
        FakeBrokerSyncControl,
        UserDefaults
    ) {
        let suiteName = "StationTests.Broker.\(UUID().uuidString)"
        let defaults = metadataDefaults ?? UserDefaults(suiteName: suiteName)!
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let controller = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: credentialStore,
            validator: validator,
            syncControl: syncControl,
            metadataStore: metadataStore,
            behavioralAnalysisOptedOut: behavioralAnalysisOptedOut
        )
        return (controller, credentialStore, validator, syncControl, defaults)
    }

    private let sampleCredentials = BrokerCredentials(
        apiKey: "test-api-key-abc123",
        apiSecret: "test-api-secret-xyz789"
    )

    @Test func emptyFieldsFailLocalValidationWithoutCallingValidatorOrKeychain() async {
        let (controller, store, validator, sync, _) = makeController()
        controller.updateFields(apiKey: "", apiSecret: "")

        let outcome = await controller.connect()

        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .apiSecret]))
        #expect(validator.validateCallCount == 0)
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func whitespaceOnlyFieldsFailLocalValidation() async {
        let (controller, store, validator, _, _) = makeController()
        controller.updateFields(apiKey: "   ", apiSecret: "\n\t")

        let outcome = await controller.connect()

        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .apiSecret]))
        #expect(validator.validateCallCount == 0)
        #expect(store.saveCallCount == 0)
    }

    @Test func readOnlyConfirmedSavesToKeychainAndStartsSync() async throws {
        let (controller, store, validator, sync, defaults) = makeController()
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        validator.nextResult = .success(permissionPosture: .readOnlyConfirmed)
        controller.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(store.saveCallCount == 1)
        let saved = try store.read(for: .binanceUSProd)
        #expect(saved == sampleCredentials)
        #expect(sync.startSyncCallCount == 1)
        #expect(sync.lastStartedIdentity == .binanceUSProd)
        #expect(controller.permissionWarning == nil)
    }

    @Test func existingCredentialsSaveWithoutAutoStartingSyncAgain() async throws {
        let store = FakeBrokerCredentialStore()
        try store.save(
            credentials: BrokerCredentials(apiKey: "existing-key", apiSecret: "existing-secret"),
            for: .binanceUSProd
        )
        let (controller, _, validator, sync, defaults) = makeController(credentialStore: store)
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        validator.nextResult = .success(permissionPosture: .readOnlyConfirmed)
        controller.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(store.saveCallCount == 2)
        #expect(sync.startSyncCallCount == 0)
        let saved = try store.read(for: .binanceUSProd)
        #expect(saved == sampleCredentials)
    }

    @Test func withdrawPermissionBlocksSaveAndStart() async {
        let (controller, store, validator, sync, _) = makeController()
        validator.nextResult = .success(permissionPosture: .withdrawDetected)
        controller.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await controller.connect()

        #expect(outcome == .blockedWithdrawPermission)
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
        #expect(store.hasCredentials(for: .binanceUSProd) == false)
    }

    @Test func tradeEnabledSavesWithPersistentWarning() async {
        let (_, store, _, sync, defaults) = makeController()
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        let validator = FakeBrokerCredentialValidator()
        validator.nextResult = .success(permissionPosture: .tradeEnabled)
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let tradeController = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        tradeController.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await tradeController.connect()

        #expect(outcome == .connected(permissionWarning: BrokerPermissionWarning.tradeEnabled))
        #expect(tradeController.permissionWarning == .tradeEnabled)
        #expect(store.saveCallCount == 1)
        #expect(sync.startSyncCallCount == 1)

        let reloaded = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        #expect(reloaded.permissionWarning == .tradeEnabled)
    }

    @Test func unverifiablePermissionsSaveWithPersistentWarning() async {
        let (_, store, _, sync, defaults) = makeController()
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        let validator = FakeBrokerCredentialValidator()
        validator.nextResult = .success(permissionPosture: .unverifiable)
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let unverifiableController = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        unverifiableController.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await unverifiableController.connect()

        #expect(outcome == .connected(permissionWarning: BrokerPermissionWarning.unverifiable))
        #expect(unverifiableController.permissionWarning == .unverifiable)
    }

    @Test func transientValidationFailureRetainsSessionValuesWithoutKeychainPersist() async {
        let (controller, store, validator, sync, _) = makeController()
        validator.nextResult = .transientFailure(.networkUnavailable)
        controller.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await controller.connect()

        #expect(outcome == .validationTransientFailure(.networkUnavailable))
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
        #expect(controller.apiKey == sampleCredentials.apiKey)
        #expect(controller.apiSecret == sampleCredentials.apiSecret)
    }

    @Test func permanentValidationFailureDoesNotPersistToKeychain() async {
        let (controller, store, validator, sync, _) = makeController()
        validator.nextResult = .permanentFailure(.invalidCredentials)
        controller.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )

        let outcome = await controller.connect()

        #expect(outcome == .validationPermanentFailure(.invalidCredentials))
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func connectDisclosureMentionsSyncAndBehavioralUpload() {
        let message = BrokerConnectDisclosure.message(behavioralAnalysisOptedOut: false)
        #expect(message.localizedCaseInsensitiveContains("broker sync"))
        #expect(message.localizedCaseInsensitiveContains("behavioral"))
    }

    @Test func connectDisclosureRespectsBehavioralOptOut() {
        let optedOut = BrokerConnectDisclosure.message(behavioralAnalysisOptedOut: true)
        #expect(optedOut.localizedCaseInsensitiveContains("opted out"))
        #expect(!optedOut.localizedCaseInsensitiveContains("unless you opt out"))
    }

    @Test func presentationModelNeverContainsSecretMaterial() {
        let presentation = BrokerConnectPresentation(
            identity: .binanceUSProd,
            status: "Validating",
            permissionWarning: nil,
            behavioralAnalysisOptedOut: false
        )
        #expect(BrokerSecretGuard.presentationIsSafe(presentation, credentials: sampleCredentials))
    }

    @Test func secretGuardDetectsLeakedCredentialsInStrings() {
        let leaked = "error: invalid key \(sampleCredentials.apiKey)"
        #expect(BrokerSecretGuard.containsSecretMaterial(leaked, credentials: sampleCredentials))
        #expect(!BrokerSecretGuard.containsSecretMaterial("broker sync degraded", credentials: sampleCredentials))
    }

    @Test func deleteRemovesKeychainCredentials() async throws {
        let store = FakeBrokerCredentialStore()
        let (_, _, validator, sync, defaults) = makeController(credentialStore: store)
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
        controller.updateFields(
            apiKey: sampleCredentials.apiKey,
            apiSecret: sampleCredentials.apiSecret
        )
        _ = await controller.connect()
        #expect(store.hasCredentials(for: .binanceUSProd))

        try controller.deleteSavedCredentials()

        #expect(store.hasCredentials(for: .binanceUSProd) == false)
        #expect(store.deleteCallCount == 1)
        #expect(controller.permissionWarning == nil)
    }
}
