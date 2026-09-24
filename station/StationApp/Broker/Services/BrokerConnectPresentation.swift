import Foundation

public struct BrokerConnectPresentation: Equatable, Sendable, Codable {
    public let brokerSlug: String
    public let assetClass: String
    public let environment: String
    public let status: String
    public let permissionWarning: BrokerPermissionWarning?
    public let connectDisclosure: String

    public init(
        identity: BrokerConnectionIdentity,
        status: String,
        permissionWarning: BrokerPermissionWarning?,
        behavioralAnalysisOptedOut: Bool
    ) {
        brokerSlug = identity.brokerSlug
        assetClass = identity.assetClass
        environment = identity.environment
        self.status = status
        self.permissionWarning = permissionWarning
        connectDisclosure = BrokerConnectDisclosure.message(behavioralAnalysisOptedOut: behavioralAnalysisOptedOut)
    }
}

public enum BrokerSecretGuard {
    public static func containsSecretMaterial(_ value: String, credentials: BrokerCredentials) -> Bool {
        guard !value.isEmpty else { return false }
        if !credentials.apiKey.isEmpty, value.contains(credentials.apiKey) { return true }
        if !credentials.apiSecret.isEmpty, value.contains(credentials.apiSecret) { return true }
        if let consumerKey = credentials.consumerKey, !consumerKey.isEmpty, value.contains(consumerKey) {
            return true
        }
        if let tradeToken = credentials.tradeToken, !tradeToken.isEmpty, value.contains(tradeToken) {
            return true
        }
        if let sid = credentials.sid, !sid.isEmpty, value.contains(sid) {
            return true
        }
        if let hsServerId = credentials.hsServerId, !hsServerId.isEmpty, value.contains(hsServerId) {
            return true
        }
        if let passphrase = credentials.passphrase, !passphrase.isEmpty, value.contains(passphrase) {
            return true
        }
        if let pemPrivateKey = credentials.pemPrivateKey, !pemPrivateKey.isEmpty, value.contains(pemPrivateKey) {
            return true
        }
        return false
    }

    /// Drop long token-like tokens from broker error text before UI display.
    public static func sanitizeConnectMessage(_ message: String) -> String {
        sanitizeKotakMintMessage(message)
    }

    /// Drop long token-like tokens from Kotak error text before UI display.
    public static func sanitizeKotakMintMessage(_ message: String) -> String {
        let trimmed = message.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "Kotak login failed." }
        let parts = trimmed.split(whereSeparator: \.isWhitespace).map { word -> String in
            let s = String(word)
            let hexish = s.filter(\.isHexDigit).count > s.count / 2
            if s.count >= 24 || (s.count >= 8 && hexish) {
                return "[redacted]"
            }
            return s
        }
        return parts.joined(separator: " ")
    }

    public static func presentationIsSafe(
        _ presentation: BrokerConnectPresentation,
        credentials: BrokerCredentials
    ) -> Bool {
        let encoder = JSONEncoder()
        guard let data = try? encoder.encode(presentation),
              let json = String(data: data, encoding: .utf8)
        else {
            return true
        }
        return !containsSecretMaterial(json, credentials: credentials)
    }
}
