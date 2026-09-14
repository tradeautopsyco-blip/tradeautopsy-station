import Foundation
import Security

/// Reads Station Caller JWTs written by the agent Keychain store (A8 II.6).
/// Service/account must match `agent/src/station_tokens.rs`.
public enum StationCallerKeychain {
    public static let serviceName = "TradeAutopsy"
    public static let accountName = "station_caller_tokens"

    public struct Tokens: Equatable, Sendable, Codable {
        public let accessToken: String
        public let refreshToken: String
        public let expiresIn: UInt64
        public let refreshExpiresIn: UInt64?

        enum CodingKeys: String, CodingKey {
            case accessToken = "access_token"
            case refreshToken = "refresh_token"
            case expiresIn = "expires_in"
            case refreshExpiresIn = "refresh_expires_in"
        }
    }

    public static func loadTokens() -> Tokens? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: accountName,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else {
            return nil
        }
        return try? JSONDecoder().decode(Tokens.self, from: data)
    }

    public static func bearerAuthorization() -> String? {
        guard let token = loadTokens()?.accessToken, !token.isEmpty else { return nil }
        return "Bearer \(token)"
    }
}
