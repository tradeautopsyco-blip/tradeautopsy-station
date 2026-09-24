import Foundation
import Testing
@testable import Station

@MainActor
struct UpstoxConnectLifecycleTests {
    @Test func loopbackRedirectMatchesAgentContract() {
        #expect(
            UpstoxConnectContract.loopbackRedirectURI
                == "https://127.0.0.1:9140/api/daemon/broker/upstox/callback"
        )
    }

    @Test func emptyUpstoxFieldsFailLocalValidationWithoutBegin() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Upstox.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .upstoxProd,
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
        #expect(runtime.beginUpstoxConnectCallCount == 0)
        #expect(store.saveCallCount == 0)
    }

    @Test func successfulBrowserFlowPersistsAppKeysAndAutoStartsSync() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.simulateUpstoxVaultAfterBegin = true
        let sync = FakeBrokerSyncControl()
        var opened: [URL] = []
        let suite = "StationTests.Upstox.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .upstoxProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { opened.append($0) }
        )
        controller.updateFields(apiKey: "upstox-client-id", apiSecret: "upstox-client-secret")

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(runtime.beginUpstoxConnectCallCount == 1)
        #expect(opened.count == 1)
        #expect(opened[0].absoluteString.contains("api.upstox.com"))
        #expect(opened[0].absoluteString.contains("response_type=code"))
        #expect(sync.startSyncCallCount == 1)
        #expect(store.saveCallCount == 1)
        let saved = try store.read(for: .upstoxProd)
        #expect(saved?.authScheme == .upstoxOAuthBearerSession)
        #expect(saved?.apiKey == "upstox-client-id")
        #expect(saved?.apiSecret == "upstox-client-secret")
    }

    @Test func redirectUriMismatchFailsWithoutStartingSync() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginUpstoxConnectResult = UpstoxConnectBeginResult(
            state: "s",
            loginURL: URL(
                string: "https://api.upstox.com/v2/login/authorization/dialog?response_type=code"
            )!,
            redirectURI: "http://127.0.0.1:9999/wrong/callback"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Upstox.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .upstoxProd,
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

        guard case .validationTransientFailure(.upstoxConnectRejected) = outcome else {
            Issue.record("expected upstox redirect mismatch failure, got \(outcome)")
            return
        }
        #expect(sync.startSyncCallCount == 0)
        #expect(store.hasCredentials(for: .upstoxProd) == false)
    }

    @Test func beginRejectionMapsToPermanentFailureAndClearsSavedKeys() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginUpstoxConnectError = .upstoxBeginFailed(
            errorClass: "invalid_credentials",
            message: "bad key deadbeefdeadbeef"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Upstox.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .upstoxProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateFields(apiKey: "k", apiSecret: "s")

        let outcome = await controller.connect()

        guard case let .validationPermanentFailure(.upstoxConnectRejected(message)) = outcome else {
            Issue.record("expected permanent upstox failure, got \(outcome)")
            return
        }
        #expect(!message.contains("deadbeef"))
        #expect(store.hasCredentials(for: .upstoxProd) == false)
    }

    @Test func plannedUpstoxCatalogRowIsNotEnabled() {
        let upstox = BrokerCatalog.descriptor(for: "upstox")
        #expect(upstox?.availability == .planned)
        #expect(upstox?.authScheme == .upstoxOAuthBearerSession)
        #expect(upstox?.bookId == "upstox-nse-bse-cash")
    }
}
