import Combine
import Foundation

public struct MarketDataKeyListItem: Identifiable, Equatable, Sendable {
    public let id: UUID
    public let provider: MarketDataProvider
    public let maskedValue: String
    public let validationState: ProviderKeyValidationState
    public let enabled: Bool

    public init(
        id: UUID,
        provider: MarketDataProvider,
        maskedValue: String,
        validationState: ProviderKeyValidationState,
        enabled: Bool
    ) {
        self.id = id
        self.provider = provider
        self.maskedValue = maskedValue
        self.validationState = validationState
        self.enabled = enabled
    }
}

@MainActor
public final class MarketDataKeysViewModel: ObservableObject {
    @Published public private(set) var keys: [MarketDataKeyListItem] = []
    @Published public var selectedProvider: MarketDataProvider = .licensedHistory
    @Published public var draftAPIKey = ""
    @Published public private(set) var errorMessage: String?

    private let store: any ProviderAPIKeyStoring

    public init(store: (any ProviderAPIKeyStoring)? = nil) {
        self.store = store ?? KeychainProviderAPIKeyStore()
    }

    public func loadKeys() async {
        do {
            let identities = try store.listIdentities(in: .marketData)
            keys = identities.compactMap { identity in
                guard let provider = MarketDataProvider(rawValue: identity.providerSlug),
                      let record = try? store.read(for: identity)
                else { return nil }
                return MarketDataKeyListItem(
                    id: identity.keyID,
                    provider: provider,
                    maskedValue: ProviderAPIKeyMasking.maskedValue(for: record.apiKey),
                    validationState: record.validationState,
                    enabled: record.enabled
                )
            }
            .sorted { $0.provider.rawValue < $1.provider.rawValue }
        } catch {
            errorMessage = "Could not load market data keys."
        }
    }

    public func addKey(provider: MarketDataProvider, apiKey: String) async throws {
        let trimmed = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let lowered = trimmed.lowercased()
        if lowered.hasPrefix("http://") || lowered.hasPrefix("https://") {
            errorMessage = "URL is not a key."
            throw MarketDataKeyError.urlIsNotAKey
        }

        let keyID = UUID()
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: provider.rawValue,
            keyID: keyID
        )
        let record = ProviderAPIKeyRecord(apiKey: trimmed, validationState: .notValidated, enabled: true)
        try store.save(record, for: identity)
        await loadKeys()
        draftAPIKey = ""
        selectedProvider = provider
        errorMessage = nil
    }

    public func deleteKey(id: UUID) async throws {
        guard let item = keys.first(where: { $0.id == id }) else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: item.provider.rawValue,
            keyID: id
        )
        try store.delete(for: identity)
        await loadKeys()
    }

    public func disable(id: UUID) async throws {
        try await setEnabled(false, id: id)
    }

    public func enable(id: UUID) async throws {
        try await setEnabled(true, id: id)
    }

    private func setEnabled(_ enabled: Bool, id: UUID) async throws {
        guard let item = keys.first(where: { $0.id == id }) else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: item.provider.rawValue,
            keyID: id
        )
        guard let existing = try store.read(for: identity) else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        try store.save(
            ProviderAPIKeyRecord(
                apiKey: existing.apiKey,
                validationState: existing.validationState,
                enabled: enabled
            ),
            for: identity
        )
        await loadKeys()
    }

    public func provenanceStrip(for provider: MarketDataProvider) -> String {
        "History: \(provider.rawValue) (Kotak has none)"
    }
}
