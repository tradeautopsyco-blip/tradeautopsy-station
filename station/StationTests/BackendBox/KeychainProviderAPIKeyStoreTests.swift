import Foundation
import Testing
@testable import Station

struct KeychainProviderAPIKeyStoreTests {
    private func makeStore(suffix: String) -> KeychainProviderAPIKeyStore {
        KeychainProviderAPIKeyStore(serviceNameSuffix: suffix)
    }

    @Test func keychainAddListDeleteRoundTrip() throws {
        let store = makeStore(suffix: UUID().uuidString)
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: MarketDataProvider.polygon.rawValue,
            keyID: UUID()
        )
        let record = ProviderAPIKeyRecord(
            apiKey: "round-trip-secret-key",
            validationState: .notValidated
        )

        try store.save(record, for: identity)
        #expect(store.hasKey(for: identity))

        let listed = try store.listIdentities(in: .marketData)
        #expect(listed.contains(identity))

        let read = try store.read(for: identity)
        #expect(read?.apiKey == record.apiKey)
        #expect(read?.validationState == .notValidated)

        try store.delete(for: identity)
        #expect(store.hasKey(for: identity) == false)
        #expect(try store.listIdentities(in: .marketData).contains(identity) == false)
    }

    @Test func keychainNamespacesAreIsolated() throws {
        let store = makeStore(suffix: UUID().uuidString)
        let marketIdentity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: MarketDataProvider.openBB.rawValue,
            keyID: UUID()
        )
        let aiIdentity = ProviderAPIKeyIdentity(
            namespace: .aiWorkflow,
            providerSlug: AIWorkflowProvider.openAI.rawValue,
            keyID: UUID()
        )

        try store.save(
            ProviderAPIKeyRecord(apiKey: "market-only", validationState: .notValidated),
            for: marketIdentity
        )
        try store.save(
            ProviderAPIKeyRecord(apiKey: "ai-only", validationState: .notValidated),
            for: aiIdentity
        )

        let marketList = try store.listIdentities(in: .marketData)
        let aiList = try store.listIdentities(in: .aiWorkflow)

        #expect(marketList.contains(marketIdentity))
        #expect(marketList.contains(aiIdentity) == false)
        #expect(aiList.contains(aiIdentity))
        #expect(aiList.contains(marketIdentity) == false)
    }
}
