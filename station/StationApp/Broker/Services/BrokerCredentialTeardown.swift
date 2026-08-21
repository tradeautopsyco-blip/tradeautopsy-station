import Foundation
import Security

/// Fail-closed Keychain goodbye for a broker connection identity.
///
/// Shared account contract with the agent: `{env}.{broker_slug}.{connection_id}`
/// on contracted services (broker-credentials + kotak-session-vault). Login profile
/// uses `{env}.{broker_slug}.login` via `KotakLoginProfileStoring`.
///
/// Station owns Keychain teardown. Agent `clearVaultCredentials` is cache-only and
/// runs only after Keychain accounts for the identity are gone.
public enum BrokerCredentialTeardownError: Error, Equatable, Sendable {
    case keychainDeleteFailed(service: String, status: OSStatus)
}

/// Injectable SecItem seam — production uses Security framework; tests inject a fake.
public protocol BrokerKeychainItemStoring: Sendable {
    func deleteItem(service: String, account: String) throws
    func hasItem(service: String, account: String) -> Bool
    func accounts(forService service: String) -> [String]
}

public enum BrokerKeychainContract {
    /// Account = `{env}.{broker_slug}.{connection_id}` (matches agent `CredentialBlob::account_key`).
    public static func connectionAccount(for identity: BrokerConnectionIdentity) -> String {
        "\(identity.environment).\(identity.brokerSlug).\(identity.brokerConnectionID.uuidString)"
    }

    /// Services that must have no leftover connection account after successful teardown.
    /// Session vault first — fail before touching other Keychain services when vault delete fails.
    public static var contractedConnectionServices: [String] {
        [
            BrokerCredentialOrphanCleanup.kotakSessionVaultService,
            KeychainBrokerCredentialStore.serviceName,
        ]
    }
}

@MainActor
public final class BrokerCredentialTeardown {
    private let keychainItems: BrokerKeychainItemStoring
    private let credentialStore: BrokerCredentialStoring
    private let loginProfileStore: KotakLoginProfileStoring
    private let metadataStore: BrokerConnectionMetadataStoring
    private let runtimeClient: BrokerAgentRuntimeClient

    public init(
        keychainItems: BrokerKeychainItemStoring = SecItemBrokerKeychainItemStore(),
        credentialStore: BrokerCredentialStoring,
        loginProfileStore: KotakLoginProfileStoring,
        metadataStore: BrokerConnectionMetadataStoring,
        runtimeClient: BrokerAgentRuntimeClient
    ) {
        self.keychainItems = keychainItems
        self.credentialStore = credentialStore
        self.loginProfileStore = loginProfileStore
        self.metadataStore = metadataStore
        self.runtimeClient = runtimeClient
    }

    public func teardown(for identity: BrokerConnectionIdentity) async throws {
        let account = BrokerKeychainContract.connectionAccount(for: identity)

        for service in BrokerKeychainContract.contractedConnectionServices {
            try keychainItems.deleteItem(service: service, account: account)
        }

        try loginProfileStore.delete(for: identity)
        try credentialStore.delete(for: identity)
        metadataStore.delete(for: identity)

        // Agent HTTP clear is cache-only after Keychain is gone — not source of truth.
        try? await runtimeClient.clearVaultCredentials(for: identity)
    }
}

/// Production Keychain adapter for contracted broker services.
public struct SecItemBrokerKeychainItemStore: BrokerKeychainItemStoring {
    public init() {}

    public func deleteItem(service: String, account: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw BrokerCredentialTeardownError.keychainDeleteFailed(service: service, status: status)
        }
    }

    public func hasItem(service: String, account: String) -> Bool {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecUseAuthenticationUI as String: kSecUseAuthenticationUIFail,
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        return status == errSecSuccess
    }

    public func accounts(forService service: String) -> [String] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecUseAuthenticationUI as String: kSecUseAuthenticationUIFail,
        ]
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            return []
        }
        return items.compactMap { $0[kSecAttrAccount as String] as? String }
    }
}
