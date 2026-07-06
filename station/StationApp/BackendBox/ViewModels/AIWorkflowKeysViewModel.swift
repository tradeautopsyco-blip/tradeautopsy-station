import Combine
import Foundation

public struct AIWorkflowKeyListItem: Identifiable, Equatable, Sendable {
    public let id: UUID
    public let provider: AIWorkflowProvider
    public let maskedValue: String
    public let validationState: ProviderKeyValidationState

    public init(
        id: UUID,
        provider: AIWorkflowProvider,
        maskedValue: String,
        validationState: ProviderKeyValidationState
    ) {
        self.id = id
        self.provider = provider
        self.maskedValue = maskedValue
        self.validationState = validationState
    }
}

@MainActor
public final class AIWorkflowKeysViewModel: ObservableObject {
    @Published public private(set) var keys: [AIWorkflowKeyListItem] = []
    @Published public var selectedProvider: AIWorkflowProvider = .openAI
    @Published public var draftAPIKey = ""
    @Published public private(set) var errorMessage: String?

    private let store: any ProviderAPIKeyStoring

    public init(store: (any ProviderAPIKeyStoring)? = nil) {
        self.store = store ?? KeychainProviderAPIKeyStore()
    }

    public func loadKeys() async {
        do {
            let identities = try store.listIdentities(in: .aiWorkflow)
            keys = identities.compactMap { identity in
                guard let provider = AIWorkflowProvider(rawValue: identity.providerSlug),
                      let record = try? store.read(for: identity)
                else { return nil }
                return AIWorkflowKeyListItem(
                    id: identity.keyID,
                    provider: provider,
                    maskedValue: ProviderAPIKeyMasking.maskedValue(for: record.apiKey),
                    validationState: record.validationState
                )
            }
            .sorted { $0.provider.rawValue < $1.provider.rawValue }
        } catch {
            errorMessage = "Could not load AI / workflow keys."
        }
    }

    public func addKey(provider: AIWorkflowProvider, apiKey: String) async throws {
        let trimmed = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }

        let keyID = UUID()
        let identity = ProviderAPIKeyIdentity(
            namespace: .aiWorkflow,
            providerSlug: provider.rawValue,
            keyID: keyID
        )
        let record = ProviderAPIKeyRecord(apiKey: trimmed, validationState: .notValidated)
        try store.save(record, for: identity)
        await loadKeys()
        draftAPIKey = ""
        selectedProvider = provider
    }

    public func deleteKey(id: UUID) async throws {
        guard let item = keys.first(where: { $0.id == id }) else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        let identity = ProviderAPIKeyIdentity(
            namespace: .aiWorkflow,
            providerSlug: item.provider.rawValue,
            keyID: id
        )
        try store.delete(for: identity)
        await loadKeys()
    }
}
