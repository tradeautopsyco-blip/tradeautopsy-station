import Combine
import Foundation
import Notch

public struct MarketDataKeyListItem: Identifiable, Equatable, Sendable {
    public let id: UUID
    public let provider: MarketDataProvider
    public let maskedValue: String
    public let validationState: ProviderKeyValidationState
    public let enabled: Bool
    public let fetchMode: VendorFetchMode
    public let lastObtainStatus: String?

    public init(
        id: UUID,
        provider: MarketDataProvider,
        maskedValue: String,
        validationState: ProviderKeyValidationState,
        enabled: Bool,
        fetchMode: VendorFetchMode = .off,
        lastObtainStatus: String? = nil
    ) {
        self.id = id
        self.provider = provider
        self.maskedValue = maskedValue
        self.validationState = validationState
        self.enabled = enabled
        self.fetchMode = fetchMode
        self.lastObtainStatus = lastObtainStatus
    }
}

@MainActor
public final class MarketDataKeysViewModel: ObservableObject {
    @Published public private(set) var keys: [MarketDataKeyListItem] = []
    @Published public var selectedProvider: MarketDataProvider = .licensedHistory
    @Published public var draftAPIKey = ""
    @Published public private(set) var errorMessage: String?

    private let store: any ProviderAPIKeyStoring
    private let bindingClient: (any VendorBindingClienting)?
    private let obtainSession: URLSession
    private let obtainPort: UInt16
    private var amfiEnabled: Bool {
        didSet {
            UserDefaults.standard.set(amfiEnabled, forKey: Self.amfiEnabledKey)
        }
    }
    private var lastObtainStatus: [String: String] = [:]

    public init(
        store: (any ProviderAPIKeyStoring)? = nil,
        bindingClient: (any VendorBindingClienting)? = nil,
        obtainSession: URLSession = .shared,
        obtainPort: UInt16 = AgentLoopback.port
    ) {
        self.store = store ?? KeychainProviderAPIKeyStore()
        self.bindingClient = bindingClient
        self.obtainSession = obtainSession
        self.obtainPort = obtainPort
        self.amfiEnabled = UserDefaults.standard.object(forKey: Self.amfiEnabledKey) as? Bool ?? true
    }

    public func loadKeys() async {
        do {
            let identities = try store.listIdentities(in: .marketData)
            var loaded: [MarketDataKeyListItem] = identities.compactMap { identity in
                guard let provider = MarketDataProvider(rawValue: identity.providerSlug),
                      provider.requiresKey,
                      let record = try? store.read(for: identity)
                else { return nil }
                return MarketDataKeyListItem(
                    id: identity.keyID,
                    provider: provider,
                    maskedValue: ProviderAPIKeyMasking.maskedValue(for: record.apiKey),
                    validationState: record.validationState,
                    enabled: record.enabled,
                    fetchMode: VendorFetchModeStore.mode(for: provider.rawValue),
                    lastObtainStatus: lastObtainStatus[provider.rawValue]
                )
            }
            loaded.append(amfiRow())
            keys = loaded.sorted { $0.provider.rawValue < $1.provider.rawValue }
        } catch {
            errorMessage = "Could not load market data keys."
        }
    }

    public func addKey(provider: MarketDataProvider, apiKey: String) async throws {
        guard provider.requiresKey else { return }
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
        do {
            try await pushBinding(
                VendorBindingPut(
                    adapterId: provider.rawValue,
                    enabled: true,
                    apiKey: trimmed,
                    historyBudget: 60
                )
            )
        } catch {
            try? store.save(
                ProviderAPIKeyRecord(apiKey: trimmed, validationState: .notValidated, enabled: false),
                for: identity
            )
            errorMessage = "Agent did not accept the vendor key."
            await loadKeys()
            throw error
        }
        await loadKeys()
        draftAPIKey = ""
        selectedProvider = provider
        errorMessage = nil
    }

    public func deleteKey(id: UUID) async throws {
        guard let item = keys.first(where: { $0.id == id }), item.provider.requiresKey else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        try? await pushBinding(
            VendorBindingPut(adapterId: item.provider.rawValue, enabled: false)
        )
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

    public func setFetchMode(_ mode: VendorFetchMode, id: UUID) async {
        guard let item = keys.first(where: { $0.id == id }) else { return }
        VendorFetchModeStore.set(mode, for: item.provider.rawValue)
        await loadKeys()
    }

    public func obtainNow(id: UUID) async {
        guard let item = keys.first(where: { $0.id == id }) else { return }
        VendorFetchModeStore.armObtain(item.provider.rawValue)
        let query: String
        switch item.provider {
        case .amfi:
            query = "adapter=amfi&operation=amfi_nav"
        case .licensedHistory:
            query = "adapter=kotak_neo&operation=history"
        }
        guard let url = URL(string: "http://127.0.0.1:\(obtainPort)/api/station/obtain?\(query)") else {
            return
        }
        do {
            let (data, _) = try await obtainSession.data(from: url)
            if let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
               let status = json["status"] as? String
            {
                lastObtainStatus[item.provider.rawValue] = status
            } else {
                lastObtainStatus[item.provider.rawValue] = "unavailable"
            }
        } catch {
            lastObtainStatus[item.provider.rawValue] = "unavailable"
        }
        await loadKeys()
    }

    private func setEnabled(_ enabled: Bool, id: UUID) async throws {
        guard let item = keys.first(where: { $0.id == id }) else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        if item.provider == .amfi {
            amfiEnabled = enabled
            try await pushBinding(
                VendorBindingPut(adapterId: MarketDataProvider.amfi.rawValue, enabled: enabled)
            )
            await loadKeys()
            return
        }
        let identity = ProviderAPIKeyIdentity(
            namespace: .marketData,
            providerSlug: item.provider.rawValue,
            keyID: id
        )
        guard let existing = try store.read(for: identity) else {
            throw ProviderAPIKeyStoreError.keyNotFound
        }
        try await pushBinding(
            VendorBindingPut(
                adapterId: item.provider.rawValue,
                enabled: enabled,
                apiKey: enabled ? existing.apiKey : nil,
                historyBudget: enabled ? 60 : nil
            )
        )
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

    private func pushBinding(_ body: VendorBindingPut) async throws {
        guard let bindingClient else { return }
        try await bindingClient.putBinding(body)
    }

    private func amfiRow() -> MarketDataKeyListItem {
        MarketDataKeyListItem(
            id: MarketDataKeysViewModel.amfiRowID,
            provider: .amfi,
            maskedValue: "public",
            validationState: .notValidated,
            enabled: amfiEnabled,
            fetchMode: VendorFetchModeStore.mode(for: MarketDataProvider.amfi.rawValue),
            lastObtainStatus: lastObtainStatus[MarketDataProvider.amfi.rawValue]
        )
    }

    public func provenanceStrip(for provider: MarketDataProvider) -> String {
        provider.provenanceStrip
    }

    static let amfiRowID = UUID(uuidString: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee")!
    private static let amfiEnabledKey = "station.vendor.amfi.enabled"
}
