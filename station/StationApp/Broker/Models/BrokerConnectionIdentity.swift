import Foundation

public struct BrokerConnectionIdentity: Equatable, Sendable, Codable {
    public let brokerConnectionID: UUID
    public let brokerSlug: String
    public let assetClass: String
    public let environment: String

    public init(
        brokerConnectionID: UUID,
        brokerSlug: String,
        assetClass: String,
        environment: String
    ) {
        self.brokerConnectionID = brokerConnectionID
        self.brokerSlug = brokerSlug
        self.assetClass = assetClass
        self.environment = environment
    }
}

public struct BrokerCredentials: Equatable, Sendable, Codable {
    public let apiKey: String
    public let apiSecret: String

    public init(apiKey: String, apiSecret: String) {
        self.apiKey = apiKey
        self.apiSecret = apiSecret
    }
}
