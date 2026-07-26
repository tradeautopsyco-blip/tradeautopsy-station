import Foundation

public enum PlannedBrokerAvailability: Equatable, Sendable {
    case enabled
    case parked
    case planned
}

public enum BrokerAuthScheme: String, Equatable, Sendable, Codable {
    case hmacApiKeySecret = "hmac_api_key_secret"
    case kotakNeoTotpSession = "kotak_neo_totp_session"
}

public enum BrokerAdapterOrigin: String, Equatable, Sendable, Codable {
    case firstParty = "first_party"
    case communityReviewed = "community_reviewed"
    case communityUnreviewed = "community_unreviewed"
}

/// Catalog / UBI descriptor (CONTEXT BrokerDescriptor, R7).
public struct PlannedBrokerDescriptor: Equatable, Identifiable, Sendable {
    public var id: String { slug }
    public let slug: String
    public let displayName: String
    public let assetClass: String
    public let quoteCurrency: String
    public let authScheme: BrokerAuthScheme
    public let calcProfileId: String
    public let complianceProfileId: String
    public let availability: PlannedBrokerAvailability
    public let origin: BrokerAdapterOrigin

    public init(
        slug: String,
        displayName: String,
        assetClass: String,
        quoteCurrency: String,
        authScheme: BrokerAuthScheme,
        calcProfileId: String,
        complianceProfileId: String,
        availability: PlannedBrokerAvailability,
        origin: BrokerAdapterOrigin = .firstParty
    ) {
        self.slug = slug
        self.displayName = displayName
        self.assetClass = assetClass
        self.quoteCurrency = quoteCurrency
        self.authScheme = authScheme
        self.calcProfileId = calcProfileId
        self.complianceProfileId = complianceProfileId
        self.availability = availability
        self.origin = origin
    }
}

public enum BrokerCatalog {
    /// First pair enabled; binance_us parked; v1 origin = first_party only (R7).
    /// Named next broker (T2.1): `zerodha_kite` — B6 sheet stub only; stays `.planned`
    /// until `/Users/bishnu/issues/brokers/sheets/zerodha_kite.md` is SIGNED.
    public static let v1: [PlannedBrokerDescriptor] = [
        PlannedBrokerDescriptor(
            slug: "binance_com",
            displayName: "Binance.com",
            assetClass: "crypto_spot",
            quoteCurrency: "USD",
            authScheme: .hmacApiKeySecret,
            calcProfileId: "crypto_spot_usd",
            complianceProfileId: "binance_com_compliance",
            availability: .enabled
        ),
        PlannedBrokerDescriptor(
            slug: "kotak_neo",
            displayName: "Kotak Neo",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .kotakNeoTotpSession,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "kotak_neo_compliance",
            availability: .enabled
        ),
        PlannedBrokerDescriptor(
            slug: "binance_us",
            displayName: "Binance.US",
            assetClass: "crypto_spot",
            quoteCurrency: "USD",
            authScheme: .hmacApiKeySecret,
            calcProfileId: "crypto_spot_usd",
            complianceProfileId: "binance_us_compliance",
            availability: .parked
        ),
        PlannedBrokerDescriptor(
            slug: "interactive_brokers",
            displayName: "Interactive Brokers",
            assetClass: "equities",
            quoteCurrency: "USD",
            authScheme: .hmacApiKeySecret,
            calcProfileId: "equities_usd",
            complianceProfileId: "",
            availability: .planned
        ),
        PlannedBrokerDescriptor(
            slug: "zerodha_kite",
            displayName: "Zerodha Kite",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .hmacApiKeySecret,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "",
            availability: .planned
        ),
    ]

    public static func descriptor(for slug: String) -> PlannedBrokerDescriptor? {
        v1.first { $0.slug == slug }
    }
}
