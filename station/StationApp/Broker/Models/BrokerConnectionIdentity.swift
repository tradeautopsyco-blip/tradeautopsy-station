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
    public let hsServerId: String?
    public let expiresAt: Date?
    /// OKX API passphrase (vault-only).
    public let passphrase: String?
    /// Coinbase Advanced Trade EC private key PEM (vault-only).
    public let pemPrivateKey: String?

    public init(apiKey: String, apiSecret: String) {
        self.authScheme = .hmacApiKeySecret
        self.apiKey = apiKey
        self.apiSecret = apiSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    /// Kraken spot API key + base64 secret (HMAC vault fields; distinct auth tag for host signing).
    public init(krakenApiKey: String, krakenApiSecret: String) {
        self.authScheme = .krakenSpotNonceSession
        self.apiKey = krakenApiKey
        self.apiSecret = krakenApiSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    /// OKX global REST credentials (Station Keychain). All three fields required by the agent blob.
    public init(okxApiKey: String, okxApiSecret: String, passphrase: String) {
        self.authScheme = .okxPassphraseSession
        self.apiKey = okxApiKey
        self.apiSecret = okxApiSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = passphrase
        self.pemPrivateKey = nil
    }

    /// Coinbase Advanced Trade CDP key name + PEM EC private key (Station Keychain).
    public init(coinbaseApiKey: String, pemPrivateKey: String) {
        self.authScheme = .coinbaseJwtEs256Session
        self.apiKey = coinbaseApiKey
        self.apiSecret = ""
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = pemPrivateKey
    }

    /// Kite Connect app credentials (Station Keychain). Session access token stays agent vault-only.
    public init(kiteApiKey: String, kiteApiSecret: String) {
        self.authScheme = .kiteChecksumSession
        self.apiKey = kiteApiKey
        self.apiSecret = kiteApiSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    /// Upstox OAuth app credentials (Station Keychain). Bearer session stays agent vault-only.
    public init(upstoxClientId: String, upstoxClientSecret: String) {
        self.authScheme = .upstoxOAuthBearerSession
        self.apiKey = upstoxClientId
        self.apiSecret = upstoxClientSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    /// Fyers OAuth app credentials (Station Keychain). JWT session stays agent vault-only.
    public init(fyersAppId: String, fyersSecretId: String) {
        self.authScheme = .fyersOAuthJsonAppIdHashSession
        self.apiKey = fyersAppId
        self.apiSecret = fyersSecretId
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    /// Groww checksum-session app credentials. Written to the Keychain blob by
    /// the agent over signed loopback (ADR 0014); the minted `token` + `expiry`
    /// + `tokenRefId` stay agent vault-only alongside them.
    public init(growwApiKey: String, growwApiSecret: String) {
        self.authScheme = .growwChecksumSession
        self.apiKey = growwApiKey
        self.apiSecret = growwApiSecret
        self.consumerKey = nil
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    /// Dhan consent app credentials (Station metadata). Access token stays agent vault-only (ADR 0009).
    public init(dhanClientId: String, dhanAppId: String, dhanAppSecret: String) {
        self.authScheme = .dhanConsentSession
        self.apiKey = dhanAppId
        self.apiSecret = dhanAppSecret
        self.consumerKey = dhanClientId
        self.tradeToken = nil
        self.sid = nil
        self.baseUrl = nil
        self.hsServerId = nil
        self.expiresAt = nil
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    public init(
        consumerKey: String,
        tradeToken: String,
        sid: String,
        baseUrl: String,
        hsServerId: String = "",
        expiresAt: Date? = nil
    ) {
        self.authScheme = .kotakNeoTotpSession
        self.apiKey = ""
        self.apiSecret = ""
        self.consumerKey = consumerKey
        self.tradeToken = tradeToken
        self.sid = sid
        self.baseUrl = baseUrl
        self.hsServerId = hsServerId
        self.expiresAt = expiresAt
        self.passphrase = nil
        self.pemPrivateKey = nil
    }

    private enum CodingKeys: String, CodingKey {
        case authScheme
        case apiKey
        case apiSecret
        case consumerKey
        case tradeToken
        case sid
        case baseUrl
        case hsServerId
        case expiresAt
        case passphrase
        case pemPrivateKey
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
        hsServerId = try container.decodeIfPresent(String.self, forKey: .hsServerId)
        expiresAt = try container.decodeIfPresent(Date.self, forKey: .expiresAt)
        passphrase = try container.decodeIfPresent(String.self, forKey: .passphrase)
        pemPrivateKey = try container.decodeIfPresent(String.self, forKey: .pemPrivateKey)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(authScheme, forKey: .authScheme)
        switch authScheme {
        case .hmacApiKeySecret, .kiteChecksumSession, .upstoxOAuthBearerSession,
             .fyersOAuthJsonAppIdHashSession, .growwChecksumSession, .krakenSpotNonceSession:
            try container.encode(apiKey, forKey: .apiKey)
            try container.encode(apiSecret, forKey: .apiSecret)
        case .okxPassphraseSession:
            try container.encode(apiKey, forKey: .apiKey)
            try container.encode(apiSecret, forKey: .apiSecret)
            try container.encodeIfPresent(passphrase, forKey: .passphrase)
        case .coinbaseJwtEs256Session:
            try container.encode(apiKey, forKey: .apiKey)
            try container.encodeIfPresent(pemPrivateKey, forKey: .pemPrivateKey)
        case .dhanConsentSession:
            try container.encodeIfPresent(consumerKey, forKey: .consumerKey)
            try container.encode(apiKey, forKey: .apiKey)
            try container.encode(apiSecret, forKey: .apiSecret)
        case .kotakNeoTotpSession:
            try container.encodeIfPresent(consumerKey, forKey: .consumerKey)
            try container.encodeIfPresent(tradeToken, forKey: .tradeToken)
            try container.encodeIfPresent(sid, forKey: .sid)
            try container.encodeIfPresent(baseUrl, forKey: .baseUrl)
            try container.encodeIfPresent(hsServerId, forKey: .hsServerId)
            try container.encodeIfPresent(expiresAt, forKey: .expiresAt)
        }
    }
}
