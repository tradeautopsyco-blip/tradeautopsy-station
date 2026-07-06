import Foundation
import Testing
@testable import Station

@MainActor
struct MarketDataKeysViewModelTests {
    @Test func addKeyListsMaskedEntryWithNotValidatedState() async throws {
        let store = FakeProviderAPIKeyStore()
        let viewModel = MarketDataKeysViewModel(store: store)

        try await viewModel.addKey(provider: .polygon, apiKey: "polygon-secret-key-1234")

        #expect(viewModel.keys.count == 1)
        #expect(viewModel.keys[0].provider == .polygon)
        #expect(viewModel.keys[0].maskedValue.hasSuffix("1234"))
        #expect(viewModel.keys[0].maskedValue.contains("•"))
        #expect(viewModel.keys[0].validationState == .notValidated)
        #expect(store.saveCallCount == 1)
    }

    @Test func deleteKeyRemovesEntryFromList() async throws {
        let store = FakeProviderAPIKeyStore()
        let viewModel = MarketDataKeysViewModel(store: store)
        try await viewModel.addKey(provider: .openBB, apiKey: "openbb-key-5678")
        let keyID = viewModel.keys[0].id

        try await viewModel.deleteKey(id: keyID)

        #expect(viewModel.keys.isEmpty)
        #expect(store.deleteCallCount == 1)
    }

    @Test func loadKeysRestoresPersistedEntries() async throws {
        let store = FakeProviderAPIKeyStore()
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: MarketDataProvider.alphaVantage.rawValue,
            keyID: UUID()
        )
        try store.save(
            ProviderAPIKeyRecord(apiKey: "alpha-key-9999", validationState: .notValidated),
            for: identity
        )

        let viewModel = MarketDataKeysViewModel(store: store)
        await viewModel.loadKeys()

        #expect(viewModel.keys.count == 1)
        #expect(viewModel.keys[0].provider == .alphaVantage)
        #expect(viewModel.keys[0].maskedValue.hasSuffix("9999"))
    }
}
