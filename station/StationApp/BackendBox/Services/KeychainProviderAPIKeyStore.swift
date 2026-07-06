import Foundation
import Security

public final class KeychainProviderAPIKeyStore: ProviderAPIKeyStoring, @unchecked Sendable {
    public static let baseServiceName = "in.tradeautopsy.station.provider-api-keys"

    private let serviceName: String

    public init(serviceNameSuffix: String? = nil) {
        if let serviceNameSuffix, !serviceNameSuffix.isEmpty {
            self.serviceName = "\(Self.baseServiceName).\(serviceNameSuffix)"
        } else {
            self.serviceName = Self.baseServiceName
        }
    }

    public func save(_ record: ProviderAPIKeyRecord, for identity: ProviderAPIKeyIdentity) throws {
        let data = try encode(record)
        let query = baseQuery(for: identity)
        SecItemDelete(query as CFDictionary)
        var addQuery = query
        addQuery[kSecValueData as String] = data
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw ProviderAPIKeyStoreError.keychainError(status)
        }
    }

    public func read(for identity: ProviderAPIKeyIdentity) throws -> ProviderAPIKeyRecord? {
        var query = baseQuery(for: identity)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess, let data = item as? Data else {
            throw ProviderAPIKeyStoreError.keychainError(status)
        }
        return try decode(data)
    }

    public func delete(for identity: ProviderAPIKeyIdentity) throws {
        let status = SecItemDelete(baseQuery(for: identity) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw ProviderAPIKeyStoreError.keychainError(status)
        }
    }

    public func listIdentities(in namespace: ProviderKeyNamespace) throws -> [ProviderAPIKeyIdentity] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitAll,
        ]
        var items: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &items)
        if status == errSecItemNotFound {
            return []
        }
        guard status == errSecSuccess, let entries = items as? [[String: Any]] else {
            throw ProviderAPIKeyStoreError.keychainError(status)
        }

        return entries.compactMap { entry in
            guard let account = entry[kSecAttrAccount as String] as? String else { return nil }
            return parseAccount(account, expectedNamespace: namespace)
        }
    }

    public func hasKey(for identity: ProviderAPIKeyIdentity) -> Bool {
        (try? read(for: identity)) != nil
    }

    private func baseQuery(for identity: ProviderAPIKeyIdentity) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: account(for: identity),
        ]
    }

    private func account(for identity: ProviderAPIKeyIdentity) -> String {
        "\(identity.namespace.rawValue).\(identity.providerSlug).\(identity.keyID.uuidString)"
    }

    private func parseAccount(_ account: String, expectedNamespace: ProviderKeyNamespace) -> ProviderAPIKeyIdentity? {
        let parts = account.split(separator: ".", maxSplits: 2).map(String.init)
        guard parts.count == 3,
              parts[0] == expectedNamespace.rawValue,
              let keyID = UUID(uuidString: parts[2])
        else { return nil }
        return ProviderAPIKeyIdentity(
            namespace: expectedNamespace,
            providerSlug: parts[1],
            keyID: keyID
        )
    }

    private func encode(_ record: ProviderAPIKeyRecord) throws -> Data {
        let encoder = JSONEncoder()
        guard let data = try? encoder.encode(record) else {
            throw ProviderAPIKeyStoreError.encodingFailed
        }
        return data
    }

    private func decode(_ data: Data) throws -> ProviderAPIKeyRecord {
        let decoder = JSONDecoder()
        guard let record = try? decoder.decode(ProviderAPIKeyRecord.self, from: data) else {
            throw ProviderAPIKeyStoreError.encodingFailed
        }
        return record
    }
}
