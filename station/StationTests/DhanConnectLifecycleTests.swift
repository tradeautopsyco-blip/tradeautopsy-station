import Foundation
import Testing
@testable import Station

@MainActor
struct DhanConnectLifecycleTests {
    @Test func loopbackRedirectMatchesAgentContract() {
        #expect(
            DhanConnectContract.loopbackRedirectURI
                == "https://127.0.0.1:9140/api/daemon/broker/dhan/callback"
        )
    }

    @Test func consentLoginURLValidationAcceptsOnlyOfficialDhanPage() {
        #expect(
            DhanConnectContract.isValidConsentLoginURL(
                URL(string: "https://auth.dhan.co/login/consentApp-login?consentAppId=abc")!
            )
        )
        #expect(
            !DhanConnectContract.isValidConsentLoginURL(
                URL(string: "https://evil.example.com/login/consentApp-login?consentAppId=abc")!
            )
        )
        #expect(
            !DhanConnectContract.isValidConsentLoginURL(
                URL(string: "http://auth.dhan.co/login/consentApp-login?consentAppId=abc")!
            )
        )
        #expect(
            !DhanConnectContract.isValidConsentLoginURL(
                URL(string: "https://auth.dhan.co/app/generate-consent?client_id=abc")!
            )
        )
    }

    @Test func emptyDhanFieldsFailLocalValidationWithoutBegin() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateDhanFields(dhanClientId: "", apiKey: "", apiSecret: "")

        let outcome = await controller.connect()

        #expect(
            outcome == .localValidationFailed(
                invalidFields: [.consumerKey, .apiKey, .apiSecret]
            )
        )
        #expect(runtime.beginDhanConnectCallCount == 0)
        #expect(store.saveCallCount == 0)
    }

    @Test func missingClientIdAloneFailsLocalValidation() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateDhanFields(dhanClientId: "  ", apiKey: "app-id", apiSecret: "app-secret")

        let outcome = await controller.connect()

        #expect(outcome == .localValidationFailed(invalidFields: [.consumerKey]))
        #expect(runtime.beginDhanConnectCallCount == 0)
    }

    @Test func successfulBrowserFlowPersistsAppKeysAndAutoStartsSync() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.simulateDhanVaultAfterBegin = true
        let sync = FakeBrokerSyncControl()
        var opened: [URL] = []
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { opened.append($0) }
        )
        controller.updateDhanFields(
            dhanClientId: "dhan-client-id",
            apiKey: "dhan-app-id",
            apiSecret: "dhan-app-secret"
        )

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(runtime.beginDhanConnectCallCount == 1)
        #expect(opened.count == 1)
        #expect(opened[0].absoluteString.contains("auth.dhan.co"))
        #expect(opened[0].absoluteString.contains("consentApp-login"))
        #expect(sync.startSyncCallCount == 1)
        #expect(store.saveCallCount == 1)
        let saved = try store.read(for: .dhanProd)
        #expect(saved?.authScheme == .dhanConsentSession)
        #expect(saved?.consumerKey == "dhan-client-id")
        #expect(saved?.apiKey == "dhan-app-id")
        #expect(saved?.apiSecret == "dhan-app-secret")
    }

    @Test func unexpectedConsentURLFailsWithoutStartingSync() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginDhanConnectResult = DhanConnectBeginResult(
            loginURL: URL(string: "https://evil.example.com/login/consentApp-login")!,
            connectionId: "conn-1"
        )
        let sync = FakeBrokerSyncControl()
        var opened: [URL] = []
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { opened.append($0) }
        )
        controller.updateDhanFields(dhanClientId: "cid", apiKey: "app-id", apiSecret: "app-secret")

        let outcome = await controller.connect()

        guard case .validationTransientFailure(.dhanConnectRejected) = outcome else {
            Issue.record("expected dhan consent URL mismatch failure, got \(outcome)")
            return
        }
        #expect(opened.isEmpty)
        #expect(sync.startSyncCallCount == 0)
        #expect(store.hasCredentials(for: .dhanProd) == false)
    }

    @Test func beginRejectionMapsToPermanentFailureAndClearsSavedKeys() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.beginDhanConnectError = .dhanBeginFailed(
            errorClass: "invalid_credentials",
            message: "bad secret deadbeefdeadbeef"
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore()
        )
        controller.updateDhanFields(dhanClientId: "cid", apiKey: "app-id", apiSecret: "app-secret")

        let outcome = await controller.connect()

        guard case let .validationPermanentFailure(.dhanConnectRejected(message)) = outcome else {
            Issue.record("expected permanent dhan failure, got \(outcome)")
            return
        }
        #expect(!message.contains("deadbeef"))
        #expect(store.hasCredentials(for: .dhanProd) == false)
    }

    @Test func connectSyncStopReconnectRoundTrip() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.simulateDhanVaultAfterBegin = true
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { _ in }
        )
        controller.updateDhanFields(
            dhanClientId: "dhan-client-id",
            apiKey: "dhan-app-id",
            apiSecret: "dhan-app-secret"
        )

        let first = await controller.connect()
        #expect(first == .connected(permissionWarning: nil))
        #expect(sync.startSyncCallCount == 1)

        try? await runtime.stopSync(for: .dhanProd)
        #expect(runtime.stopSyncCallCount == 1)

        let second = await controller.connect()
        #expect(second == .connected(permissionWarning: nil))
        #expect(runtime.beginDhanConnectCallCount == 2)
        #expect(sync.startSyncCallCount == 2)
    }

    @Test func connectPersistsNoSecretsToPrefs() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.simulateDhanVaultAfterBegin = true
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.Dhan.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let controller = BrokerConnectController(
            identity: .dhanProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: FakeKotakLoginProfileStore(),
            openBrowserURL: { _ in }
        )
        controller.updateDhanFields(
            dhanClientId: "pref-probe-client",
            apiKey: "pref-probe-app-id",
            apiSecret: "pref-probe-app-secret-value"
        )

        let outcome = await controller.connect()
        #expect(outcome == .connected(permissionWarning: nil))

        let domain = defaults.persistentDomain(forName: suite) ?? [:]
        #expect(!domain.isEmpty)
        let dumped = domain.values.compactMap { value -> String? in
            if let data = value as? Data {
                return String(data: data, encoding: .utf8)
            }
            return String(describing: value)
        }.joined(separator: "\n")
        #expect(!dumped.contains("pref-probe-app-secret-value"))
        #expect(!dumped.contains("pref-probe-app-id"))
    }

    @Test func dhanPresentationNeverContainsSecretMaterial() {
        let credentials = BrokerCredentials(
            dhanClientId: "probe-client-id",
            dhanAppId: "probe-app-id",
            dhanAppSecret: "probe-app-secret-value"
        )
        let presentation = BrokerConnectPresentation(
            identity: .dhanProd,
            status: "connected",
            permissionWarning: nil,
            behavioralAnalysisOptedOut: false
        )
        #expect(BrokerSecretGuard.presentationIsSafe(presentation, credentials: credentials))
        #expect(
            BrokerSecretGuard.containsSecretMaterial("leak probe-app-secret-value", credentials: credentials)
        )
        #expect(
            BrokerSecretGuard.containsSecretMaterial("leak probe-client-id", credentials: credentials)
        )
    }

    @Test func beginPayloadCarriesDhanIdentityFields() throws {
        let payload = DhanConnectBeginPayload(
            identity: .dhanProd,
            dhanClientId: "cid",
            appId: "app-id",
            appSecret: "app-secret"
        )
        let data = try JSONEncoder().encode(payload)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        #expect(json?["brokerSlug"] as? String == "dhan")
        #expect(json?["dhanClientId"] as? String == "cid")
        #expect(json?["appId"] as? String == "app-id")
        #expect(json?["appSecret"] as? String == "app-secret")
    }

    @Test func plannedDhanCatalogRowIsNotEnabled() {
        let dhan = BrokerCatalog.descriptor(for: "dhan")
        #expect(dhan?.availability == .planned)
        #expect(dhan?.displayName == "Dhan")
        #expect(dhan?.assetClass == "equities")
        #expect(dhan?.quoteCurrency == "INR")
        #expect(dhan?.authScheme == .dhanConsentSession)
        #expect(dhan?.calcProfileId == "equities_inr_cash")
        #expect(dhan?.bookId == "dhan-nse-bse-cash")
    }
}
