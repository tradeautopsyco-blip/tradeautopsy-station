import Foundation
import Testing
@testable import Station

@MainActor
struct BrokersViewModelTests {
    private func makeViewModel(
        client: FakeBrokerControlClient = FakeBrokerControlClient()
    ) -> (BrokersViewModel, FakeBrokerControlClient, FakeBrokerCredentialStore, FakeBrokerSyncControl) {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let metadataStore = UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.BrokersVM.\(UUID().uuidString)")!
        )
        let viewModel = BrokersViewModel(
            brokerControl: client,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: sync
        )
        return (viewModel, client, store, sync)
    }

    @Test func loadsNotConfiguredFromFakeClient() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .notConfigured

        await viewModel.load()

        #expect(client.loadSnapshotCallCount == 1)
        let binance = viewModel.cards.first { $0.id == "binance_com" }
        #expect(binance?.status == .notConfigured)
    }

    @Test func loadsAgentOfflineStateFromFakeClient() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .agentOfflineWithCredentials

        await viewModel.load()

        let binance = viewModel.cards.first { $0.id == "binance_com" }
        #expect(binance?.status == .unavailableAgentOffline)
        #expect(binance?.isStartEnabled == false)
        #expect(binance?.isStopEnabled == false)
        #expect(binance?.lastSyncSummary == "Last sync: 2 fills imported")
    }

    @Test func neverInventsSyncingWhenClientReportsReadyToStart() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .readyToStart

        await viewModel.load()

        let binance = viewModel.cards.first { $0.id == "binance_com" }
        #expect(binance?.status == .readyToStart)
        #expect(binance?.status != .syncing)
    }

    @Test func connectDisclosureMentionsSyncAndUpload() {
        let (viewModel, _, _, _) = makeViewModel()
        viewModel.presentConnectSheet(for: "binance_com")
        #expect(viewModel.connectDisclosure.localizedCaseInsensitiveContains("broker sync"))
        #expect(viewModel.connectDisclosure.localizedCaseInsensitiveContains("behavioral"))
    }

    @Test func submitConnectWithEmptyFieldsSurfacesLocalValidation() async {
        let (viewModel, _, store, sync) = makeViewModel()
        viewModel.presentConnectSheet(for: "binance_com")

        await viewModel.submitConnect()

        #expect(viewModel.connectInvalidFields == [.apiKey, .apiSecret])
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func submitConnectSetsValidatingStatusOnBinanceCard() async {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let metadataStore = UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.BrokersVM.Validating.\(UUID().uuidString)")!
        )
        let client = FakeBrokerControlClient()
        client.scenario = .notConfigured
        let viewModel = BrokersViewModel(
            brokerControl: client,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: sync
        )
        await viewModel.load()

        viewModel.presentConnectSheet(for: "binance_com")
        viewModel.updateConnectFields(apiKey: "key", apiSecret: "secret")

        let connectTask = Task { await viewModel.submitConnect() }
        try? await Task.sleep(for: .milliseconds(50))

        let binance = viewModel.cards.first { $0.id == "binance_com" }
        #expect(binance?.status == .validating)

        await connectTask.value
    }

    @Test func binanceComAppearsInCatalog() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .notConfigured
        await viewModel.load()

        let binanceCom = viewModel.cards.first { $0.id == "binance_com" }
        #expect(binanceCom?.displayName == "Binance.com")
        #expect(binanceCom?.status == .notConfigured)
        #expect(binanceCom?.isConnectable == true)
    }

    @Test func confirmDeleteSurfacesTeardownFailureAndLeavesCardConfigured() async {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let suite = "StationTests.BrokersVM.DeleteFail.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let metadataStore = UserDefaultsBrokerMetadataStore(defaults: defaults)
        let client = FakeBrokerControlClient()
        client.scenario = .readyToStart
        client.deleteConnectionError = BrokerCredentialTeardownError.keychainDeleteFailed(
            service: BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            status: -25293
        )
        let viewModel = BrokersViewModel(
            brokerControl: client,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: sync
        )
        await viewModel.load()
        #expect(viewModel.cards.contains { $0.id == "binance_com" && $0.status == .readyToStart })

        viewModel.requestDelete(for: .binanceComProd)
        await viewModel.confirmDelete()

        #expect(client.deleteConnectionCallCount == 1)
        #expect(viewModel.syncActionMessage?.localizedCaseInsensitiveContains("keychain") == true)
        #expect(viewModel.cards.contains { $0.id == "binance_com" && $0.status == .readyToStart })
    }

    @Test func keychainGrantHintShowsGrantedWithoutSecretRead() async {
        let store = FakeBrokerCredentialStore()
        store.nextAccessGrant = .granted
        let (viewModel, client, _, _) = makeViewModelWithStore(store)
        client.scenario = .notConfigured
        await viewModel.load()
        await viewModel.load()
        #expect(viewModel.keychainGrantHint?.localizedCaseInsensitiveContains("granted") == true)
        #expect(store.readCallCount == 0)
    }

    @Test func keychainGrantHintAsksAlwaysAllowWhenLocked() async {
        let store = FakeBrokerCredentialStore()
        store.nextAccessGrant = .needsAlwaysAllow
        let (viewModel, client, _, _) = makeViewModelWithStore(store)
        client.scenario = .notConfigured
        await viewModel.load()
        #expect(viewModel.keychainGrantHint?.localizedCaseInsensitiveContains("always allow") == true)
        #expect(store.readCallCount == 0)
    }

    @Test func beginChangeKotakLoginDetailsNoOpWhenNotTotpOnlyMode() async {
        let (viewModel, _, _, _) = makeViewModel()
        viewModel.presentConnectSheet(for: "kotak_neo")
        #expect(viewModel.connectSheetMode == .full)

        viewModel.beginChangeKotakLoginDetails()

        #expect(viewModel.connectSheetMode == .full)
    }

    @Test func beginChangeKotakLoginDetailsNoOpWhenUnlockCleared() async {
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
        let suite = "StationTests.BrokersVM.ChangeLoginFailClosed.\(UUID().uuidString)"
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

        viewModel.noteConnectSheetDismissed()
        viewModel.beginChangeKotakLoginDetails()

        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        #expect(profiles.unlockCallCount == 1)
    }

    @Test func noteConnectSheetDismissedClearsUnlockSoChangeLoginNeedsFreshTouchID() async {
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
        let suite = "StationTests.BrokersVM.DismissUnlock.\(UUID().uuidString)"
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
        viewModel.beginChangeKotakLoginDetails()
        #expect(viewModel.connectSheetMode == .full)

        viewModel.noteConnectSheetDismissed()
        viewModel.isConnectSheetPresented = false

        viewModel.presentEditSheet(for: "kotak_neo")
        #expect(viewModel.connectSheetMode == .kotakTotpOnly)
        viewModel.beginChangeKotakLoginDetails()
        #expect(viewModel.connectSheetMode == .full)
        #expect(profiles.unlockCallCount == 2)
    }

    private func makeViewModelWithStore(
        _ store: FakeBrokerCredentialStore
    ) -> (BrokersViewModel, FakeBrokerControlClient, FakeBrokerCredentialStore, FakeBrokerSyncControl) {
        let sync = FakeBrokerSyncControl()
        let metadataStore = UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.BrokersVM.\(UUID().uuidString)")!
        )
        let client = FakeBrokerControlClient()
        let viewModel = BrokersViewModel(
            brokerControl: client,
            credentialStore: store,
            metadataStore: metadataStore,
            syncControl: sync
        )
        return (viewModel, client, store, sync)
    }
}
