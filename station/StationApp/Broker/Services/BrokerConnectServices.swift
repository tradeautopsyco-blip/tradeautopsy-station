import Foundation

public enum BrokerConnectServices {
    public static func identity(for slug: String, environment: TradeAutopsyEnvironment = .prod) -> BrokerConnectionIdentity {
        switch slug {
        case "binance_us":
            return .binanceUS(environment)
        case "binance_com":
            return .binanceCom(environment)
        default:
            return BrokerConnectionIdentity(
                brokerConnectionID: UUID(),
                brokerSlug: slug,
                assetClass: "crypto",
                environment: environment.rawValue
            )
        }
    }

    public static func validator(for slug: String) -> BrokerCredentialValidating {
        switch slug {
        case "binance_us":
            return BinanceUSCredentialValidator()
        case "binance_com":
            return BinanceComCredentialValidator()
        default:
            return UnsupportedBrokerCredentialValidator()
        }
    }

    public static func displayName(for slug: String) -> String {
        BrokerCatalog.v1.first { $0.slug == slug }?.displayName ?? slug
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
