import Foundation
import Testing
@testable import Station

@MainActor
struct FyersConnectLifecycleTests {
    @Test func loopbackRedirectMatchesAgentContract() {
        #expect(
            FyersConnectContract.loopbackRedirectURI
                == "https://127.0.0.1:9140/api/daemon/broker/fyers/callback"
        )
    }

    @Test func emptyFyersFieldsFailLocalValidationWithoutBegin() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Fyers.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .fyersProd,
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
        #expect(runtime.beginFyersConnectCallCount == 0)
        #expect(store.saveCallCount == 0)
    }

    @Test func successfulBrowserFlowPersistsAppKeysAndAutoStartsSync() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.simulateFyersVaultAfterBegin = true
        let sync = FakeBrokerSyncControl()
        var opened: [URL] = []
        let suite = "StationTests.Fyers.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .fyersProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { opened.append($0) }
        )
        controller.updateFields(apiKey: "fyers-app-id", apiSecret: "fyers-secret-id")

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(runtime.beginFyersConnectCallCount == 1)
        #expect(opened.count == 1)
        #expect(opened[0].absoluteString.contains("api-t1.fyers.in"))
        #expect(opened[0].absoluteString.contains("response_type=code"))
        #expect(sync.startSyncCallCount == 1)
        #expect(store.saveCallCount == 1)
        let saved = try store.read(for: .fyersProd)
        #expect(saved?.authScheme == .fyersOAuthJsonAppIdHashSession)
        #expect(saved?.apiKey == "fyers-app-id")
        #expect(saved?.apiSecret == "fyers-secret-id")
    }

    @Test func redirectUriMismatchFailsWithoutStartingSync() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginFyersConnectResult = FyersConnectBeginResult(
            state: "s",
            loginURL: URL(
                string: "https://api-t1.fyers.in/api/v3/generate-authcode?response_type=code"
            )!,
            redirectURI: "http://127.0.0.1:9999/wrong/callback"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Fyers.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .fyersProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { _ in }
        )
        controller.updateFields(apiKey: "k", apiSecret: "s")

        let outcome = await controller.connect()

        guard case .validationTransientFailure(.fyersConnectRejected) = outcome else {
            Issue.record("expected fyers redirect mismatch failure, got \(outcome)")
            return
        }
        #expect(sync.startSyncCallCount == 0)
        #expect(store.hasCredentials(for: .fyersProd) == false)
    }

    @Test func beginRejectionMapsToPermanentFailureAndClearsSavedKeys() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginFyersConnectError = .fyersBeginFailed(
            errorClass: "invalid_credentials",
            message: "bad secret deadbeefdeadbeef"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Fyers.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .fyersProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateFields(apiKey: "k", apiSecret: "s")

        let outcome = await controller.connect()

        guard case let .validationPermanentFailure(.fyersConnectRejected(message)) = outcome else {
            Issue.record("expected permanent fyers failure, got \(outcome)")
            return
        }
        #expect(!message.contains("deadbeef"))
        #expect(store.hasCredentials(for: .fyersProd) == false)
    }

    @Test func plannedFyersCatalogRowIsNotEnabled() {
        let fyers = BrokerCatalog.descriptor(for: "fyers")
        #expect(fyers?.availability == .planned)
        #expect(fyers?.authScheme == .fyersOAuthJsonAppIdHashSession)
        #expect(fyers?.bookId == "fyers-nse-bse-cash")
    }
}
