import Foundation

public enum BrokerConnectServices {
    public static func identity(for slug: String, environment: TradeAutopsyEnvironment = .prod) -> BrokerConnectionIdentity {
        if let descriptor = BrokerCatalog.descriptor(for: slug) {
            switch slug {
            case "binance_us":
                return .binanceUS(environment)
            case "binance_com":
                return .binanceCom(environment)
            case "kotak_neo":
                return BrokerConnectionIdentity(
                    brokerConnectionID: BrokerConnectionIdentity.kotakNeoConnectionID,
                    brokerSlug: "kotak_neo",
                    assetClass: descriptor.assetClass,
                    environment: environment.rawValue
                )
            default:
                return BrokerConnectionIdentity(
                    brokerConnectionID: UUID(),
                    brokerSlug: slug,
                    assetClass: descriptor.assetClass,
                    environment: environment.rawValue
                )
            }
        }
        return BrokerConnectionIdentity(
            brokerConnectionID: UUID(),
            brokerSlug: slug,
            assetClass: "crypto_spot",
            environment: environment.rawValue
        )
    }

    public static func validator(for slug: String) -> BrokerCredentialValidating {
        switch slug {
        case "binance_us":
            return BinanceUSCredentialValidator()
        case "binance_com":
            return BinanceComCredentialValidator()
        case "kotak_neo":
            return KotakNeoSessionCredentialValidator()
        default:
            return UnsupportedBrokerCredentialValidator()
        }
    }

    public static func displayName(for slug: String) -> String {
        BrokerCatalog.v1.first { $0.slug == slug }?.displayName ?? slug
    }

    public static func authScheme(for slug: String) -> BrokerAuthScheme {
        BrokerCatalog.descriptor(for: slug)?.authScheme ?? .hmacApiKeySecret
    }
}

/// Phase 2: accept non-empty Kotak session fields; live venue check ships with Phase 3 Wasm.
private struct KotakNeoSessionCredentialValidator: BrokerCredentialValidating, Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "kotak_neo",
              credentials.authScheme == .kotakNeoTotpSession,
              let consumerKey = credentials.consumerKey, !consumerKey.isEmpty,
              let tradeToken = credentials.tradeToken, !tradeToken.isEmpty,
              let sid = credentials.sid, !sid.isEmpty,
              let baseUrl = credentials.baseUrl, !baseUrl.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        return .success(.readOnlyConfirmed)
    }
}

private struct UnsupportedBrokerCredentialValidator: BrokerCredentialValidating, Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        _ = credentials
        _ = identity
        return .permanentFailure(.invalidCredentials)
    }
}
