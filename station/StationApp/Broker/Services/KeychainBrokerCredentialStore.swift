import Foundation
import Security

public enum BrokerCredentialStoreError: Error, Equatable {
    case encodingFailed
    case keychainError(OSStatus)
    case accessControlUnavailable
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

        // Prefer ACL that trusts Station + bundled agent so Start does not prompt for login password.
        if let access = try? makeTrustedAccessIncludingAgent() {
            addQuery[kSecAttrAccess as String] = access
        } else {
            addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        }

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
        accessGrant(for: identity) == .granted
    }

    /// Attributes-only + fail-closed UI. Does not fetch the secret blob and does not
    /// present the login-password sheet. How we tell Always Allow stuck, without changing ACL.
    public func accessGrant(for identity: BrokerConnectionIdentity) -> BrokerKeychainAccessGrant {
        var query = baseQuery(for: identity)
        query[kSecReturnAttributes as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        query[kSecUseAuthenticationUI as String] = kSecUseAuthenticationUIFail
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        switch status {
        case errSecSuccess:
            return .granted
        case errSecItemNotFound:
            return .missing
        default:
            return .needsAlwaysAllow
        }
    }

    /// Best-effort SecAccess trusting this app + `tradeautopsy-agent` next to the executable.
    private func makeTrustedAccessIncludingAgent() throws -> SecAccess {
        var trusted: [SecTrustedApplication] = []

        var selfApp: SecTrustedApplication?
        let selfStatus = SecTrustedApplicationCreateFromPath(nil, &selfApp)
        if selfStatus == errSecSuccess, let selfApp {
            trusted.append(selfApp)
        }

        if let agentPath = Self.bundledAgentPath() {
            var agentApp: SecTrustedApplication?
            let agentStatus = SecTrustedApplicationCreateFromPath(agentPath, &agentApp)
            if agentStatus == errSecSuccess, let agentApp {
                trusted.append(agentApp)
            }
        }

        guard !trusted.isEmpty else {
            throw BrokerCredentialStoreError.accessControlUnavailable
        }

        var access: SecAccess?
        let status = SecAccessCreate(
            "TradeAutopsy broker credentials" as CFString,
            trusted as CFArray,
            &access
        )
        guard status == errSecSuccess, let access else {
            throw BrokerCredentialStoreError.accessControlUnavailable
        }
        return access
    }

    private static func bundledAgentPath() -> String? {
        guard let exe = Bundle.main.executableURL?.deletingLastPathComponent() else {
            return nil
        }
        let agent = exe.appendingPathComponent("tradeautopsy-agent")
        return FileManager.default.isExecutableFile(atPath: agent.path) ? agent.path : nil
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
