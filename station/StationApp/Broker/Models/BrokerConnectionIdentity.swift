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

/// Tagged Keychain credential blob (R6). Legacy flat `{apiKey,apiSecret}` still decodes as HMAC.
public struct BrokerCredentials: Equatable, Sendable, Codable {
    public let authScheme: BrokerAuthScheme
    public let apiKey: String
    public let apiSecret: String
    public let consumerKey: String?
    public let tradeToken: String?
    public let sid: String?
    public let baseUrl: String?
    public let expiresAt: Date?

    public init(apiKey: String, apiSecret: String) {
        self.authScheme = .hmacApiKeySecret
        self.apiKey = apiKey
        self.apiSecret = apiSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.expiresAt = nil
    }

    public init(
        consumerKey: String,
        tradeToken: String,
        sid: String,
        baseUrl: String,
        expiresAt: Date? = nil
    ) {
        self.authScheme = .kotakNeoTotpSession
        self.apiKey = ""
        self.apiSecret = ""
        self.consumerKey = consumerKey
        self.tradeToken = tradeToken
        self.sid = sid
        self.baseUrl = baseUrl
        self.expiresAt = expiresAt
    }

    private enum CodingKeys: String, CodingKey {
        case authScheme
        case apiKey
        case apiSecret
        case consumerKey
        case tradeToken
        case sid
        case baseUrl
        case expiresAt
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let scheme = try container.decodeIfPresent(BrokerAuthScheme.self, forKey: .authScheme)
            ?? .hmacApiKeySecret
        authScheme = scheme
        apiKey = try container.decodeIfPresent(String.self, forKey: .apiKey) ?? ""
        apiSecret = try container.decodeIfPresent(String.self, forKey: .apiSecret) ?? ""
        consumerKey = try container.decodeIfPresent(String.self, forKey: .consumerKey)
        tradeToken = try container.decodeIfPresent(String.self, forKey: .tradeToken)
        sid = try container.decodeIfPresent(String.self, forKey: .sid)
        baseUrl = try container.decodeIfPresent(String.self, forKey: .baseUrl)
        expiresAt = try container.decodeIfPresent(Date.self, forKey: .expiresAt)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(authScheme, forKey: .authScheme)
        switch authScheme {
        case .hmacApiKeySecret:
            try container.encode(apiKey, forKey: .apiKey)
            try container.encode(apiSecret, forKey: .apiSecret)
        case .kotakNeoTotpSession:
            try container.encodeIfPresent(consumerKey, forKey: .consumerKey)
            try container.encodeIfPresent(tradeToken, forKey: .tradeToken)
            try container.encodeIfPresent(sid, forKey: .sid)
            try container.encodeIfPresent(baseUrl, forKey: .baseUrl)
            try container.encodeIfPresent(expiresAt, forKey: .expiresAt)
        }
    }
}
