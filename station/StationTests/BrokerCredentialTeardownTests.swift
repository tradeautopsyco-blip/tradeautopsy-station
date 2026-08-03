import Foundation
import Testing
@testable import Station

@MainActor
struct BrokerCredentialTeardownTests {
    private let identity = BrokerConnectionIdentity.kotakNeoProd

    private func account(for identity: BrokerConnectionIdentity) -> String {
        "\(identity.environment).\(identity.brokerSlug).\(identity.brokerConnectionID.uuidString)"
    }

    @Test func teardownClearsSessionVaultThenStationStores() async throws {
        let keychain = FakeBrokerKeychainItemStore()
        let credentialStore = FakeBrokerCredentialStore()
        let loginProfiles = FakeKotakLoginProfileStore()
        let defaults = UserDefaults(suiteName: "StationTests.Teardown.\(UUID().uuidString)")!
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let runtime = FakeBrokerAgentRuntimeClient()

        let vaultAccount = account(for: identity)
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: vaultAccount,
            payload: Data("session".utf8)
        )
        try credentialStore.save(
            credentials: BrokerCredentials(
                consumerKey: "ck",
                tradeToken: "tt",
                sid: "sid",
                baseUrl: "https://example.test"
            ),
            for: identity
        )
        try loginProfiles.save(
            KotakLoginProfile(
                consumerKey: "ck",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "1212"
            ),
            for: identity
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date(), syncPaused: false),
            for: identity
        )

        let teardown = BrokerCredentialTeardown(
            keychainItems: keychain,
            credentialStore: credentialStore,
            loginProfileStore: loginProfiles,
            metadataStore: metadataStore,
            runtimeClient: runtime
        )

        try await teardown.teardown(for: identity)

        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: vaultAccount
            ) == false
        )
        #expect(credentialStore.hasCredentials(for: identity) == false)
        #expect(loginProfiles.hasProfile(for: identity) == false)
        #expect(metadataStore.load(for: identity) == nil)
    }

    @Test func vaultDeleteFailureLeavesStationStoresAndThrows() async throws {
        let keychain = FakeBrokerKeychainItemStore()
        let credentialStore = FakeBrokerCredentialStore()
        let loginProfiles = FakeKotakLoginProfileStore()
        let defaults = UserDefaults(suiteName: "StationTests.TeardownFail.\(UUID().uuidString)")!
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let runtime = FakeBrokerAgentRuntimeClient()

        let vaultAccount = account(for: identity)
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: vaultAccount
        )
        keychain.deleteFailureByService[BrokerCredentialOrphanCleanup.kotakSessionVaultService] =
            .keychainDeleteFailed(service: BrokerCredentialOrphanCleanup.kotakSessionVaultService, status: -25293)

        try credentialStore.save(
            credentials: BrokerCredentials(
                consumerKey: "ck",
                tradeToken: "tt",
                sid: "sid",
                baseUrl: "https://example.test"
            ),
            for: identity
        )
        try loginProfiles.save(
            KotakLoginProfile(
                consumerKey: "ck",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "1212"
            ),
            for: identity
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date(), syncPaused: false),
            for: identity
        )

        let teardown = BrokerCredentialTeardown(
            keychainItems: keychain,
            credentialStore: credentialStore,
            loginProfileStore: loginProfiles,
            metadataStore: metadataStore,
            runtimeClient: runtime
        )

        do {
            try await teardown.teardown(for: identity)
            Issue.record("expected teardown to throw on vault delete failure")
        } catch let BrokerCredentialTeardownError.keychainDeleteFailed(service, _) {
            #expect(service == BrokerCredentialOrphanCleanup.kotakSessionVaultService)
        } catch {
            Issue.record("unexpected error: \(error)")
        }

        #expect(credentialStore.hasCredentials(for: identity) == true)
        #expect(loginProfiles.hasProfile(for: identity) == true)
        #expect(metadataStore.load(for: identity) != nil)
        #expect(runtime.clearVaultCredentialsCallCount == 0)
        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: vaultAccount
            ) == true
        )
    }

    @Test func agentClearVaultFailureStillSucceedsWhenKeychainGone() async throws {
        let keychain = FakeBrokerKeychainItemStore()
        let credentialStore = FakeBrokerCredentialStore()
        let loginProfiles = FakeKotakLoginProfileStore()
        let defaults = UserDefaults(suiteName: "StationTests.TeardownAgent.\(UUID().uuidString)")!
        defer { defaults.removePersistentDomain(forName: defaults.description) }
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.clearVaultError = BrokerAgentRuntimeError.requestFailed

        let vaultAccount = account(for: identity)
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: vaultAccount
        )
        try credentialStore.save(
            credentials: BrokerCredentials(
                consumerKey: "ck",
                tradeToken: "tt",
                sid: "sid",
                baseUrl: "https://example.test"
            ),
            for: identity
        )
        metadataStore.save(
            BrokerConnectionMetadata(lastValidatedAt: Date()),
            for: identity
        )

        let teardown = BrokerCredentialTeardown(
            keychainItems: keychain,
            credentialStore: credentialStore,
            loginProfileStore: loginProfiles,
            metadataStore: metadataStore,
            runtimeClient: runtime
        )

        try await teardown.teardown(for: identity)

        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: vaultAccount
            ) == false
        )
        #expect(credentialStore.hasCredentials(for: identity) == false)
        #expect(metadataStore.load(for: identity) == nil)
        #expect(runtime.clearVaultCredentialsCallCount == 1)
    }

    @Test func reconcilePurgesOnlyUnconfiguredVaultAccounts() {
        let keychain = FakeBrokerKeychainItemStore()
        let canonical = BrokerConnectionIdentity.kotakNeoProd
        let orphanID = UUID(uuidString: "00000000-0000-4000-8000-000000000099")!
        let randomID = UUID(uuidString: "11111111-1111-4111-8111-111111111111")!

        let canonicalAccount = account(for: canonical)
        let orphanAccount = "prod.kotak_neo.\(orphanID.uuidString)"
        let randomAccount = "prod.kotak_neo.\(randomID.uuidString)"

        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: canonicalAccount
        )
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: orphanAccount
        )
        keychain.seed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            account: randomAccount
        )
        keychain.seed(
            service: KeychainBrokerCredentialStore.serviceName,
            account: orphanAccount
        )

        let purged = BrokerCredentialOrphanCleanup.reconcileVaultAccounts(
            configuredConnectionIDs: Set([canonical.brokerConnectionID]),
            environments: ["prod"],
            keychainItems: keychain
        )

        #expect(purged >= 2)
        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: canonicalAccount
            ) == true
        )
        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: orphanAccount
            ) == false
        )
        #expect(
            keychain.hasItem(
                service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
                account: randomAccount
            ) == false
        )
        #expect(
            keychain.hasItem(
                service: KeychainBrokerCredentialStore.serviceName,
                account: orphanAccount
            ) == false
        )
    }
}
