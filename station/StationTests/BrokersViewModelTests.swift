import Foundation
import Testing
@testable import Station

@MainActor
struct BrokersViewModelTests {
    private func makeViewModel(
        client: FakeBrokerControlClient = FakeBrokerControlClient(),
        connectController: BrokerConnectController? = nil
    ) -> (BrokersViewModel, FakeBrokerControlClient, FakeBrokerCredentialStore, FakeBrokerSyncControl) {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let validator = FakeBrokerCredentialValidator()
        let metadataStore = UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.BrokersVM.\(UUID().uuidString)")!
        )
        let controller = connectController ?? BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        let viewModel = BrokersViewModel(brokerControl: client, connectController: controller)
        return (viewModel, client, store, sync)
    }

    @Test func loadsNotConfiguredFromFakeClient() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .notConfigured

        await viewModel.load()

        #expect(client.loadSnapshotCallCount == 1)
        let binance = viewModel.cards.first { $0.id == "binance_us" }
        #expect(binance?.status == .notConfigured)
    }

    @Test func loadsAgentOfflineStateFromFakeClient() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .agentOfflineWithCredentials

        await viewModel.load()

        let binance = viewModel.cards.first { $0.id == "binance_us" }
        #expect(binance?.status == .unavailableAgentOffline)
        #expect(binance?.isStartEnabled == false)
        #expect(binance?.isStopEnabled == false)
        #expect(binance?.lastSyncSummary == "Last sync: 2 fills imported")
    }

    @Test func neverInventsSyncingWhenClientReportsReadyToStart() async {
        let (viewModel, client, _, _) = makeViewModel()
        client.scenario = .readyToStart

        await viewModel.load()

        let binance = viewModel.cards.first { $0.id == "binance_us" }
        #expect(binance?.status == .readyToStart)
        #expect(binance?.status != .syncing)
    }

    @Test func connectDisclosureMentionsSyncAndUpload() {
        let (viewModel, _, _, _) = makeViewModel()
        #expect(viewModel.connectDisclosure.localizedCaseInsensitiveContains("broker sync"))
        #expect(viewModel.connectDisclosure.localizedCaseInsensitiveContains("behavioral"))
    }

    @Test func submitConnectWithEmptyFieldsSurfacesLocalValidation() async {
        let (viewModel, _, store, sync) = makeViewModel()

        await viewModel.submitConnect()

        #expect(viewModel.connectInvalidFields == [.apiKey, .apiSecret])
        #expect(store.saveCallCount == 0)
        #expect(sync.startSyncCallCount == 0)
    }

    @Test func submitConnectSetsValidatingStatusOnBinanceCard() async {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let validator = SlowFakeBrokerCredentialValidator()
        let metadataStore = UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.BrokersVM.Validating.\(UUID().uuidString)")!
        )
        let controller = BrokerConnectController(
            identity: .binanceUSProd,
            credentialStore: store,
            validator: validator,
            syncControl: sync,
            metadataStore: metadataStore
        )
        let client = FakeBrokerControlClient()
        client.scenario = .notConfigured
        let viewModel = BrokersViewModel(brokerControl: client, connectController: controller)
        await viewModel.load()

        controller.updateFields(apiKey: "key", apiSecret: "secret")
        viewModel.updateConnectFields(apiKey: "key", apiSecret: "secret")

        let connectTask = Task { await viewModel.submitConnect() }
        try? await Task.sleep(for: .milliseconds(50))

        let binance = viewModel.cards.first { $0.id == "binance_us" }
        #expect(binance?.status == .validating)
        #expect(controller.isValidating == true)

        await connectTask.value
        #expect(controller.isValidating == false)
    }
}
