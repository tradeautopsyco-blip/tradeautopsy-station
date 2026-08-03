import Foundation
import Testing
@testable import Station

@MainActor
struct KotakTotpConnectLifecycleTests {
    @Test func emptyKotakLoginFieldsFailLocalValidationWithoutMint() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.credentialStore = store
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakMint.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .kotakNeoProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateKotakLoginFields(
            consumerKey: "",
            mobileNumber: "",
            ucc: "",
            totp: "",
            mpin: ""
        )

        let outcome = await controller.connect()

        #expect(outcome == .localValidationFailed(invalidFields: [
            .consumerKey, .mobileNumber, .ucc, .totp, .mpin,
        ]))
        #expect(runtime.mintKotakSessionCallCount == 0)
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func successfulMintPersistsSessionAndAutoStartsSync() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.credentialStore = store
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakMint.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .kotakNeoProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateKotakLoginFields(
            consumerKey: "ck-live",
            mobileNumber: "+919999999999",
            ucc: "UCC1",
            totp: "123456",
            mpin: "1212"
        )

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(runtime.mintKotakSessionCallCount == 1)
        #expect(sync.startSyncCallCount == 1)
        // Vault write is agent-owned (mint); Station must not rewrite ACL ownership.
        #expect(store.saveCallCount == 1)
        let saved = try store.read(for: .kotakNeoProd)
        #expect(saved?.authScheme == .kotakNeoTotpSession)
        #expect(saved?.consumerKey == "ck-live")
        #expect(saved?.tradeToken == "minted-trade-token")
        // TOTP/MPIN must not linger on the controller after connect.
        #expect(controller.totp.isEmpty)
        #expect(controller.mpin.isEmpty)
    }

    @Test func mintRejectionMapsToPermanentFailureWithoutKeychainWriteFromController() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.credentialStore = store
        runtime.mintError = .kotakMintFailed(errorClass: "totp_failed", message: "bad totp deadbeefdeadbeefdeadbeef")
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakMint.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .kotakNeoProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateKotakLoginFields(
            consumerKey: "ck",
            mobileNumber: "+919999999999",
            ucc: "UCC1",
            totp: "000000",
            mpin: "1212"
        )

        let outcome = await controller.connect()

        guard case let .validationPermanentFailure(.kotakMintRejected(message)) = outcome else {
            Issue.record("expected kotakMintRejected permanent failure, got \(outcome)")
            return
        }
        #expect(!message.contains("deadbeef"))
        #expect(message.contains("[redacted]") || message.lowercased().contains("totp") || !message.isEmpty)
        #expect(sync.startSyncCallCount == 0)
        #expect(store.hasCredentials(for: .kotakNeoProd) == false)
    }

    @Test func profileSaveFailureClearsAgentVaultAndFailsConnect() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.credentialStore = store
        let profiles = FakeKotakLoginProfileStore()
        profiles.saveError = KotakLoginProfileStoreError.keychainError(-1)
        let keychain = FakeBrokerKeychainItemStore()
        let vaultAccount = BrokerKeychainContract.connectionAccount(for: .kotakNeoProd)
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: vaultAccount
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakMint.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .kotakNeoProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: profiles,
            keychainItems: keychain
        )
        controller.updateKotakLoginFields(
            consumerKey: "ck",
            mobileNumber: "+919999999999",
            ucc: "UCC1",
            totp: "123456",
            mpin: "1212"
        )

        let outcome = await controller.connect()

        guard case .validationTransientFailure(.kotakMintRejected) = outcome else {
            Issue.record("expected profile-save transient failure, got \(outcome)")
            return
        }
        #expect(runtime.mintKotakSessionCallCount == 1)
        #expect(runtime.clearVaultCredentialsCallCount == 1)
        #expect(store.hasCredentials(for: .kotakNeoProd) == false)
        #expect(sync.startSyncCallCount == 0)
        #expect(profiles.hasProfile(for: .kotakNeoProd) == false)
        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: vaultAccount
            ) == false
        )
    }
}
