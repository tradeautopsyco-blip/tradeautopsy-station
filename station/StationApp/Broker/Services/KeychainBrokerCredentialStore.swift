import Foundation
import Security

public enum BrokerCredentialStoreError: Error, Equatable {
    case encodingFailed
    case keychainError(OSStatus)
}

public final class KeychainBrokerCredentialStore: BrokerCredentialStoring, @unchecked Sendable {
    public static let serviceName = "in.tradeautopsy.station.broker-credentials"

    public init() {}

    public func save(credentials: BrokerCredentials, for identity: BrokerConnectionIdentity) throws {
        let data = try encode(credentials)
        let query = baseQuery(for: identity)
        SecItemDelete(query as CFDictionary)
        var addQuery = query
        addQuery[kSecValueData as String] = data
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw BrokerCredentialStoreError.keychainError(status)
        }
    }

    public func read(for identity: BrokerConnectionIdentity) throws -> BrokerCredentials? {
        var query = baseQuery(for: identity)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess, let data = item as? Data else {
            throw BrokerCredentialStoreError.keychainError(status)
        }
        return try decode(data)
    }

    public func delete(for identity: BrokerConnectionIdentity) throws {
        let status = SecItemDelete(baseQuery(for: identity) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw BrokerCredentialStoreError.keychainError(status)
        }
    }

    public func hasCredentials(for identity: BrokerConnectionIdentity) -> Bool {
        (try? read(for: identity)) != nil
    }

    private func baseQuery(for identity: BrokerConnectionIdentity) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.serviceName,
            kSecAttrAccount as String: account(for: identity),
        ]
    }

    private func account(for identity: BrokerConnectionIdentity) -> String {
        "\(identity.environment).\(identity.brokerSlug).\(identity.brokerConnectionID.uuidString)"
    }

    private func encode(_ credentials: BrokerCredentials) throws -> Data {
        let encoder = JSONEncoder()
        guard let data = try? encoder.encode(credentials) else {
            throw BrokerCredentialStoreError.encodingFailed
        }
        return data
    }

    private func decode(_ data: Data) throws -> BrokerCredentials {
        let decoder = JSONDecoder()
        guard let credentials = try? decoder.decode(BrokerCredentials.self, from: data) else {
            throw BrokerCredentialStoreError.encodingFailed
        }
        return credentials
    }
}
