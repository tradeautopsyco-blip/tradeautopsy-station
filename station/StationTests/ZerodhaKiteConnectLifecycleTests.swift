import Foundation
import Testing
@testable import Station

@MainActor
struct ZerodhaKiteConnectLifecycleTests {
    @Test func loopbackRedirectMatchesAgentContract() {
        #expect(
            ZerodhaKiteConnectContract.loopbackRedirectURI
                == "https://127.0.0.1:9140/api/daemon/broker/zerodha/callback"
        )
    }

    @Test func emptyKiteFieldsFailLocalValidationWithoutBegin() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Zerodha.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .zerodhaKiteProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateFields(apiKey: "", apiSecret: "")

        let outcome = await controller.connect()

        #expect(outcome == .localValidationFailed(invalidFields: [.apiKey, .apiSecret]))
        #expect(runtime.beginZerodhaConnectCallCount == 0)
        #expect(store.saveCallCount == 0)
    }

    @Test func successfulBrowserFlowAutoStartsSyncWithoutRewritingAgentVault() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.simulateZerodhaVaultAfterBegin = true
        let sync = FakeBrokerSyncControl()
        let opened = TestURLCapture()
        let suite = "StationTests.Zerodha.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .zerodhaKiteProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            oauthWebLogin: { url, _ in
                opened.append(url)
                return true
            }
        )
        controller.updateFields(apiKey: "kite-key", apiSecret: "kite-secret")

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(runtime.beginZerodhaConnectCallCount == 1)
        #expect(opened.urls.count == 1)
        #expect(opened.urls[0].absoluteString.contains("kite.zerodha.com"))
        #expect(sync.startSyncCallCount == 1)
        // Agent owns the session blob after OAuth; Station must not overwrite it with app keys.
        #expect(store.saveCallCount == 0)
    }

    @Test func redirectUriMismatchFailsWithoutStartingSync() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginZerodhaConnectResult = ZerodhaConnectBeginResult(
            state: "s",
            loginURL: URL(string: "https://kite.zerodha.com/connect/login?v=3")!,
            redirectURI: "http://127.0.0.1:9999/wrong/callback"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Zerodha.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .zerodhaKiteProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            oauthWebLogin: { _, _ in true }
        )
        controller.updateFields(apiKey: "k", apiSecret: "s")

        let outcome = await controller.connect()

        guard case .validationTransientFailure(.kiteConnectRejected) = outcome else {
            Issue.record("expected kite redirect mismatch failure, got \(outcome)")
            return
        }
        #expect(sync.startSyncCallCount == 0)
        #expect(store.hasCredentials(for: .zerodhaKiteProd) == false)
    }

    @Test func beginRejectionMapsToPermanentFailureAndClearsSavedKeys() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginZerodhaConnectError = .zerodhaBeginFailed(
            errorClass: "invalid_credentials",
            message: "bad key deadbeefdeadbeef"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Zerodha.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .zerodhaKiteProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateFields(apiKey: "k", apiSecret: "s")

        let outcome = await controller.connect()

        guard case let .validationPermanentFailure(.kiteConnectRejected(message)) = outcome else {
            Issue.record("expected permanent kite failure, got \(outcome)")
            return
        }
        #expect(!message.contains("deadbeef"))
        #expect(store.hasCredentials(for: .zerodhaKiteProd) == false)
    }

    @Test func zerodhaCatalogRowIsEnabled() {
        let kite = BrokerCatalog.descriptor(for: "zerodha_kite")
        #expect(kite?.availability == .planned)
        #expect(BrokerDogfoodProgram.allowsConnectWhilePlanned(slug: "zerodha_kite"))
        #expect(kite?.authScheme == .kiteChecksumSession)
        #expect(kite?.bookId == "zerodha-nse-bse-cash")
    }
}
