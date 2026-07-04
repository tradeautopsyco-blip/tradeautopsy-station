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
        if value.contains(credentials.apiKey) { return true }
        if value.contains(credentials.apiSecret) { return true }
        return false
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
