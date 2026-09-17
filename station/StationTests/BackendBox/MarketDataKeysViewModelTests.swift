import Foundation
import Notch
import Testing
@testable import Station

@MainActor
struct MarketDataKeysViewModelTests {
    private func licensed(_ viewModel: MarketDataKeysViewModel) -> MarketDataKeyListItem? {
        viewModel.keys.first { $0.provider == .licensedHistory }
    }

    private func amfi(_ viewModel: MarketDataKeysViewModel) -> MarketDataKeyListItem? {
        viewModel.keys.first { $0.provider == .amfi }
    }

    @Test func shippingAllowlistIsLicensedHistoryAndAmfi() {
        let slugs = MarketDataProvider.allCases.map(\.rawValue)
        #expect(slugs.contains("licensed_history"))
        #expect(slugs.contains("amfi"))
        #expect(slugs.contains("OpenBB") == false)
        #expect(slugs.contains("Polygon") == false)
        #expect(slugs.contains("Alpha Vantage") == false)
        #expect(slugs.contains("finnhub") == false)
        #expect(MarketDataProvider.amfi.requiresKey == false)
        #expect(MarketDataProvider.licensedHistory.requiresKey == true)
    }

    @Test func addKeyRefusesURLAndDoesNotSave() async {
        let store = FakeProviderAPIKeyStore()
        let client = FakeVendorBindingClient()
        let viewModel = MarketDataKeysViewModel(store: store, bindingClient: client)

        await #expect(throws: MarketDataKeyError.urlIsNotAKey) {
            try await viewModel.addKey(
                provider: .licensedHistory,
                apiKey: "https://evil.example/klines"
            )
        }

        #expect(licensed(viewModel) == nil)
        #expect(store.saveCallCount == 0)
        #expect(client.puts.isEmpty)
    }

    @Test func addKeySavesLicensedHistoryAndPushesEnableToAgent() async throws {
        let store = FakeProviderAPIKeyStore()
        let client = FakeVendorBindingClient()
        let viewModel = MarketDataKeysViewModel(store: store, bindingClient: client)

        try await viewModel.addKey(provider: .licensedHistory, apiKey: "lh-fixture-key")

        let row = try #require(licensed(viewModel))
        #expect(row.maskedValue != "lh-fixture-key")
        #expect(row.maskedValue.contains("•"))
        #expect(row.enabled == true)
        #expect(store.saveCallCount == 1)
        #expect(client.puts.count == 1)
        #expect(client.puts[0].adapterId == "licensed_history")
        #expect(client.puts[0].enabled == true)
        #expect(client.puts[0].apiKey == "lh-fixture-key")
        #expect(amfi(viewModel) != nil)
    }

    @Test func disableThenEnablePushesAgent() async throws {
        let store = FakeProviderAPIKeyStore()
        let client = FakeVendorBindingClient()
        let viewModel = MarketDataKeysViewModel(store: store, bindingClient: client)
        try await viewModel.addKey(provider: .licensedHistory, apiKey: "lh-fixture-key")
        let keyID = try #require(licensed(viewModel)?.id)

        try await viewModel.disable(id: keyID)
        #expect(licensed(viewModel)?.enabled == false)
        #expect(client.puts.last?.enabled == false)

        try await viewModel.enable(id: keyID)
        #expect(licensed(viewModel)?.enabled == true)
        #expect(client.puts.last?.enabled == true)
    }

    @Test func provenanceStripNamesLicensedHistoryAndKotakHasNone() {
        let viewModel = MarketDataKeysViewModel(store: FakeProviderAPIKeyStore())
        let strip = viewModel.provenanceStrip(for: .licensedHistory)
        #expect(strip.contains("licensed_history"))
        #expect(strip.contains("Kotak has none"))
        #expect(viewModel.provenanceStrip(for: .amfi).contains("labs"))
    }

    @Test func addKeyListsMaskedEntryWithNotValidatedState() async throws {
        let store = FakeProviderAPIKeyStore()
        let viewModel = MarketDataKeysViewModel(store: store)

        try await viewModel.addKey(provider: .licensedHistory, apiKey: "polygon-secret-key-1234")

        let row = try #require(licensed(viewModel))
        #expect(row.maskedValue.hasSuffix("1234"))
        #expect(row.maskedValue.contains("•"))
        #expect(row.validationState == .notValidated)
        #expect(store.saveCallCount == 1)
    }

    @Test func deleteKeyRemovesLicensedHistoryAndPushesDisable() async throws {
        let store = FakeProviderAPIKeyStore()
        let client = FakeVendorBindingClient()
        let viewModel = MarketDataKeysViewModel(store: store, bindingClient: client)
        try await viewModel.addKey(provider: .licensedHistory, apiKey: "openbb-key-5678")
        let keyID = try #require(licensed(viewModel)?.id)

        try await viewModel.deleteKey(id: keyID)

        #expect(licensed(viewModel) == nil)
        #expect(amfi(viewModel) != nil)
        #expect(store.deleteCallCount == 1)
        #expect(client.puts.last?.enabled == false)
    }

    @Test func loadKeysRestoresPersistedEntriesAndAmfiBinding() async throws {
        let store = FakeProviderAPIKeyStore()
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: MarketDataProvider.licensedHistory.rawValue,
            keyID: UUID()
        )
        try store.save(
            ProviderAPIKeyRecord(apiKey: "alpha-key-9999", validationState: .notValidated),
            for: identity
        )

        let viewModel = MarketDataKeysViewModel(store: store)
        await viewModel.loadKeys()

        let row = try #require(licensed(viewModel))
        #expect(row.maskedValue.hasSuffix("9999"))
        #expect(amfi(viewModel) != nil)
    }

    @Test func setFetchModePersistsPerIdentity() async throws {
        let store = FakeProviderAPIKeyStore()
        let viewModel = MarketDataKeysViewModel(store: store)
        await viewModel.loadKeys()
        let amfiID = try #require(amfi(viewModel)?.id)

        await viewModel.setFetchMode(.paneAuto, id: amfiID)
        #expect(amfi(viewModel)?.fetchMode == .paneAuto)
        #expect(VendorFetchModeStore.mode(for: "amfi") == .paneAuto)
        VendorFetchModeStore.set(.off, for: "amfi")
    }
}
