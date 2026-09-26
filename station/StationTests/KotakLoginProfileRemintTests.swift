import Foundation
import Testing
@testable import Station

@MainActor
struct KotakLoginProfileRemintTests {
    @Test func successfulMintPersistsLoginProfileWithoutTotp() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.credentialStore = store
        let profiles = FakeKotakLoginProfileStore()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakProfile.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let controller = BrokerConnectController(
            identity: .kotakNeoProd,
            credentialStore: store,
            validator: FakeBrokerCredentialValidator(),
            syncControl: sync,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        controller.updateKotakLoginFields(
            consumerKey: "ck-live",
            mobileNumber: "+917992202041",
            ucc: "XZ7R9",
            totp: "123456",
            mpin: "1212"
        )

        let outcome = await controller.connect()
        #expect(outcome == .connected(permissionWarning: nil))
        #expect(profiles.saveCallCount == 1)
        let saved = try profiles.unlock(for: .kotakNeoProd)
        #expect(saved?.consumerKey == "ck-live")
        #expect(saved?.mobileNumber == "+917992202041")
        #expect(saved?.ucc == "XZ7R9")
        #expect(saved?.mpin == "1212")
        #expect(profiles.hasProfile(for: .kotakNeoProd))
    }

    @Test func editSheetUnlocksProfileIntoTotpOnlyMode() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakEdit.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )
        viewModel.presentEditSheet(for: "kotak_neo")

        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        #expect(viewModel.connectConsumerKey == "ck")
        #expect(viewModel.connectMobileNumber == "+919999999999")
        #expect(viewModel.connectUcc == "UCC1")
        #expect(viewModel.connectMpin == "9999")
        #expect(viewModel.connectTotp.isEmpty)
        #expect(profiles.unlockCallCount == 1)
    }

    @Test func changeLoginDetailsAfterUnlockOpensFullFormWithoutSecondUnlock() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakChangeLogin.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )
        viewModel.presentEditSheet(for: "kotak_neo")
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)

        viewModel.beginChangeKotakLoginDetails()

        #expect(viewModel.connectSheetMode == .full)
        #expect(viewModel.connectConsumerKey == "ck")
        #expect(viewModel.connectMobileNumber == "+919999999999")
        #expect(viewModel.connectUcc == "UCC1")
        #expect(viewModel.connectMpin == "9999")
        #expect(profiles.unlockCallCount == 1)
    }

    @Test func editWithoutProfileOpensEmptyFullFormWithoutConsumerKeyPrefill() async throws {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        // Session vault has consumer key — Edit-fallback must NOT prefill it.
        try store.save(
            credentials: BrokerCredentials(
                consumerKey: "should-not-prefill",
                tradeToken: "t",
                sid: "s",
                baseUrl: "https://example.com",
                hsServerId: ""
            ),
            for: .kotakNeoProd
        )
        let profiles = FakeKotakLoginProfileStore()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakEditEmpty.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )
        viewModel.presentEditSheet(for: "kotak_neo")

        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .full)
        #expect(viewModel.connectConsumerKey.isEmpty)
        #expect(viewModel.connectMobileNumber.isEmpty)
        #expect(viewModel.connectUcc.isEmpty)
        #expect(viewModel.connectMpin.isEmpty)
        #expect(viewModel.connectMessage?.contains("Login not saved") == true)
    }

    @Test func remintMethodMatchesEditSheetTotpOnlyParity() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakRemintParity.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )
        viewModel.presentKotakTotpRemint(for: "kotak_neo")

        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        #expect(viewModel.connectConsumerKey == "ck")
        #expect(profiles.unlockCallCount == 1)
    }

    @Test func startSyncMissingCredentialsOpensKotakTotpRemint() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        broker.startSyncError = BrokerSyncStartError.missingCredentials
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakStartRemint.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )

        await viewModel.startSync(for: .kotakNeoProd)

        #expect(broker.startSyncCallCount == 1)
        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        #expect(viewModel.syncActionMessage?.contains("fresh TOTP") == true)
        #expect(profiles.unlockCallCount == 1)
    }

    @Test func startSyncMissingCredentialsKeepsHmacMessageWithoutOpeningTotpSheet() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        broker.startSyncError = BrokerSyncStartError.missingCredentials
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.HmacStartMsg.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )

        await viewModel.startSync(for: .binanceComProd)

        #expect(broker.startSyncCallCount == 1)
        #expect(viewModel.isConnectSheetPresented == false)
        #expect(viewModel.syncActionMessage?.contains("Use Edit") == true)
        #expect(profiles.unlockCallCount == 0)
    }

    @Test func beginConnectStartsWithoutSheetWhenVaultOk() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck-saved",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let runtime = FakeBrokerAgentRuntimeClient()
        let suite = "StationTests.KotakBeginConnectVaultOk.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(broker.startSyncCallCount == 1)
        #expect(viewModel.isConnectSheetPresented == false)
        #expect(viewModel.syncActionMessage?.contains("session vault OK") == true)
        #expect(profiles.unlockCallCount == 0)
        #expect(runtime.fetchSyncHealthCallCount == 1)
    }

    @Test func beginConnectRemintsWhenVaultMissing() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        broker.startSyncError = BrokerSyncStartError.missingCredentials
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck-saved",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let runtime = FakeBrokerAgentRuntimeClient()
        let suite = "StationTests.KotakBeginConnectVaultMiss.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(broker.startSyncCallCount == 1)
        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        #expect(viewModel.connectConsumerKey == "ck-saved")
        #expect(profiles.unlockCallCount == 1)
        #expect(runtime.fetchSyncHealthCallCount == 0)
    }

    @Test func beginConnectSurfacesKillSwitchWhenDnsActive() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck-saved",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.syncHealthOverride = BrokerSyncHealthSnapshot(
            syncState: "disconnected",
            runtimeStatus: "degraded",
            lastError: "adapter: kotak_neo host_blocked (http 0)",
            killDnsActive: true
        )
        let suite = "StationTests.KotakBeginConnectHostBlocked.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(broker.startSyncCallCount == 1)
        #expect(viewModel.isConnectSheetPresented == false)
        #expect(viewModel.syncActionMessage?.contains("Kill switch") == true)
        #expect(profiles.unlockCallCount == 0)
    }

    @Test func beginConnectRemintsOnUbiHostBlockedWithoutKillDns() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck-saved",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let runtime = FakeBrokerAgentRuntimeClient()
        // DNS already clear — UBI allowlist rejected session baseUrl.
        runtime.syncHealthOverride = BrokerSyncHealthSnapshot(
            syncState: "disconnected",
            runtimeStatus: "degraded",
            lastError: "adapter: kotak_neo host_blocked (http 0)",
            killDnsActive: false
        )
        let suite = "StationTests.KotakBeginConnectUbiHostBlocked.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(viewModel.syncActionMessage?.contains("Kill switch") != true)
        #expect(viewModel.syncActionMessage?.contains("allowlist") == true)
        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
    }

    @Test func beginConnectDoesNotClaimSuccessOnTransitionalStale() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck-saved",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let runtime = FakeBrokerAgentRuntimeClient()
        // Mirrors post-Start race: stale + no error before host_blocked lands.
        runtime.syncHealthOverride = BrokerSyncHealthSnapshot(
            syncState: "stale",
            runtimeStatus: "degraded",
            lastError: nil
        )
        let suite = "StationTests.KotakBeginConnectStale.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(viewModel.syncActionMessage?.contains("session vault OK") != true)
        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
    }

    @Test func beginConnectRemintsWhenStillDisconnectedAfterStart() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        try? profiles.save(
            KotakLoginProfile(
                consumerKey: "ck-saved",
                mobileNumber: "+919999999999",
                ucc: "UCC1",
                mpin: "9999"
            ),
            for: .kotakNeoProd
        )
        let sync = FakeBrokerSyncControl()
        let runtime = FakeBrokerAgentRuntimeClient()
        runtime.syncHealthOverride = BrokerSyncHealthSnapshot(
            syncState: "disconnected",
            runtimeStatus: "degraded",
            lastError: "adapter: unauthorized"
        )
        let suite = "StationTests.KotakBeginConnectStillOffline.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            runtimeClient: runtime,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(broker.startSyncCallCount == 1)
        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        #expect(viewModel.syncActionMessage?.contains("still offline") == true)
        #expect(profiles.unlockCallCount == 1)
    }

    @Test func beginConnectOpensFullFormWhenNoKotakProfile() async {
        let broker = FakeBrokerControlClient()
        broker.scenario = .notConfigured
        let store = FakeBrokerCredentialStore()
        let profiles = FakeKotakLoginProfileStore()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.KotakBeginConnectFull.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let viewModel = BrokersViewModel(
            brokerControl: broker,
            credentialStore: store,
            metadataStore: UserDefaultsBrokerMetadataStore(defaults: defaults),
            syncControl: sync,
            loginProfileStore: profiles
        )
        await viewModel.beginConnect(for: "kotak_neo")

        #expect(broker.startSyncCallCount == 0)
        #expect(viewModel.isConnectSheetPresented)
        #expect(viewModel.connectSheetMode == .full)
        #expect(viewModel.connectConsumerKey.isEmpty)
        #expect(profiles.unlockCallCount == 0)
    }
}
