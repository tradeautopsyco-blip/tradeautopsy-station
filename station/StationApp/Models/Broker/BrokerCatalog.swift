import Foundation

public enum PlannedBrokerAvailability: Equatable, Sendable {
    case enabled
    case parked
    case planned
}

public enum BrokerAuthScheme: String, Equatable, Sendable, Codable {
    case hmacApiKeySecret = "hmac_api_key_secret"
    case kotakNeoTotpSession = "kotak_neo_totp_session"
    case kiteChecksumSession = "kite_checksum_session"
    case upstoxOAuthBearerSession = "upstox_oauth_bearer_session"
    case fyersOAuthJsonAppIdHashSession = "fyers_oauth_json_app_id_hash_session"
    case growwChecksumSession = "groww_checksum_session"
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
    /// Versioned SourceManifest id. Capabilities come from the manifest, not the slug.
    public let manifestId: String
    /// Lock book id (locks/binance-com-spot.md, locks/kotak-nse-bse-cash.md; fetch 2026-08-22 IST).
    public let bookId: String

    public init(
        slug: String,
        displayName: String,
        assetClass: String,
        quoteCurrency: String,
        authScheme: BrokerAuthScheme,
        calcProfileId: String,
        complianceProfileId: String,
        availability: PlannedBrokerAvailability,
        manifestId: String,
        bookId: String,
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
        self.manifestId = manifestId
        self.bookId = bookId
    }
}

public enum BrokerCatalog {
    /// Live first pair only (dogfood). Next brokers live in issues B6 sheets until SIGNED — not in this catalog.
    public static let v1: [PlannedBrokerDescriptor] = [
        PlannedBrokerDescriptor(
            slug: "binance_com",
            displayName: "Binance.com",
            assetClass: "crypto_spot",
            quoteCurrency: "USD",
            authScheme: .hmacApiKeySecret,
            calcProfileId: "crypto_spot_usd",
            complianceProfileId: "binance_com_compliance",
            availability: .enabled,
            manifestId: "binance_com.s1.v1",
            bookId: "binance-com-spot"
        ),
        PlannedBrokerDescriptor(
            slug: "kotak_neo",
            displayName: "Kotak Neo",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .kotakNeoTotpSession,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "kotak_neo_compliance",
            availability: .enabled,
            manifestId: "kotak_neo.s1k.v1",
            bookId: "kotak-nse-bse-cash"
        ),
        PlannedBrokerDescriptor(
            slug: "zerodha_kite",
            displayName: "Zerodha Kite",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .kiteChecksumSession,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "zerodha_kite_compliance",
            availability: .enabled,
            manifestId: "tradeautopsy:zerodha-kite-cash@0.1.0",
            bookId: "zerodha-nse-bse-cash"
        ),
        PlannedBrokerDescriptor(
            slug: "upstox",
            displayName: "Upstox",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .upstoxOAuthBearerSession,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "upstox_compliance",
            availability: .enabled,
            manifestId: "tradeautopsy:upstox-cash@0.1.0",
            bookId: "upstox-nse-bse-cash"
        ),
        PlannedBrokerDescriptor(
            slug: "fyers",
            displayName: "Fyers",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .fyersOAuthJsonAppIdHashSession,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "fyers_compliance",
            availability: .enabled,
            manifestId: "tradeautopsy:fyers-cash@0.1.0",
            bookId: "fyers-nse-bse-cash"
        ),
        PlannedBrokerDescriptor(
            slug: "groww",
            displayName: "Groww",
            assetClass: "equities",
            quoteCurrency: "INR",
            authScheme: .growwChecksumSession,
            calcProfileId: "equities_inr_cash",
            complianceProfileId: "groww_compliance",
            availability: .enabled,
            manifestId: "tradeautopsy:groww-cash@0.1.0",
            bookId: "groww-nse-bse-cash"
        ),
    ]

    public static func descriptor(for slug: String) -> PlannedBrokerDescriptor? {
        v1.first { $0.slug == slug }
    }
}
