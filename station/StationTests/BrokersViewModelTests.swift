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
}
