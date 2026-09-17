import Foundation
import Testing
@testable import Station

@MainActor
struct AIWorkflowKeysViewModelTests {
    @Test func addKeyListsMaskedEntryWithNotValidatedState() async throws {
        let store = FakeProviderAPIKeyStore()
        let viewModel = AIWorkflowKeysViewModel(store: store)

        try await viewModel.addKey(provider: .openAI, apiKey: "sk-openai-secret-abcd")

        #expect(viewModel.keys.count == 1)
        #expect(viewModel.keys[0].provider == .openAI)
        #expect(viewModel.keys[0].maskedValue.hasSuffix("abcd"))
        #expect(viewModel.keys[0].validationState == .notValidated)
        #expect(store.saveCallCount == 1)
    }

    @Test func deleteKeyRemovesEntryFromList() async throws {
        let store = FakeProviderAPIKeyStore()
        let viewModel = AIWorkflowKeysViewModel(store: store)
        try await viewModel.addKey(provider: .anthropic, apiKey: "anthropic-key-wxyz")
        let keyID = viewModel.keys[0].id

        try await viewModel.deleteKey(id: keyID)

        #expect(viewModel.keys.isEmpty)
        #expect(store.deleteCallCount == 1)
    }

    @Test func marketDataKeysAreNotVisibleInAIWorkflowNamespace() async throws {
        let store = FakeProviderAPIKeyStore()
        let marketDataIdentity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: MarketDataProvider.licensedHistory.rawValue,
            keyID: UUID()
        )
        try store.save(
            ProviderAPIKeyRecord(apiKey: "polygon-only-key", validationState: .notValidated),
            for: marketDataIdentity
        )

        let viewModel = AIWorkflowKeysViewModel(store: store)
        await viewModel.loadKeys()

        #expect(viewModel.keys.isEmpty)
    }
}
