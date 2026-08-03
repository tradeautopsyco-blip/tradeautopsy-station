import Foundation

public enum BrokerConnectServices {
    public static func identity(for slug: String, environment: TradeAutopsyEnvironment = .prod) -> BrokerConnectionIdentity {
        switch slug {
        case "binance_com":
            return .binanceCom(environment)
        case "kotak_neo":
            return .kotakNeo(environment)
        default:
            // Never mint random UUIDs for vault keys — unknown slugs get a nil-safe fixed namespace.
            return BrokerConnectionIdentity(
                brokerConnectionID: UUID(uuidString: "00000000-0000-4000-8000-00000000ffff")!,
                brokerSlug: slug,
                assetClass: BrokerCatalog.descriptor(for: slug)?.assetClass ?? "crypto_spot",
                environment: environment.rawValue
            )
        }
    }

    public static func validator(for slug: String) -> BrokerCredentialValidating {
        switch slug {
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

/// Phase 3: Kotak TOTP mint is the live venue check (agent session mint).
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
        _ = credentials.hsServerId
        return .success(permissionPosture: .readOnlyConfirmed)
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
