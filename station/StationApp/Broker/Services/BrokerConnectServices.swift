import Foundation

public enum BrokerConnectServices {
    public static func identity(for slug: String, environment: TradeAutopsyEnvironment = .prod) -> BrokerConnectionIdentity {
        switch slug {
        case "binance_com":
            return .binanceCom(environment)
        case "kotak_neo":
            return .kotakNeo(environment)
        case "zerodha_kite":
            return .zerodhaKite(environment)
        case "upstox":
            return .upstox(environment)
        case "fyers":
            return .fyers(environment)
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
        case "zerodha_kite":
            return ZerodhaKiteSessionCredentialValidator()
        case "upstox":
            return UpstoxOAuthSessionCredentialValidator()
        case "fyers":
            return FyersOAuthSessionCredentialValidator()
        case "groww":
            return GrowwChecksumSessionCredentialValidator()
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

/// Phase 2: Kite session vault is agent-owned after browser callback; Station keeps app keys only.
private struct ZerodhaKiteSessionCredentialValidator: BrokerCredentialValidating, Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "zerodha_kite",
              credentials.authScheme == .kiteChecksumSession,
              !credentials.apiKey.isEmpty,
              !credentials.apiSecret.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        return .success(permissionPosture: .readOnlyConfirmed)
    }
}

/// Fyers JWT session vault is agent-owned after browser callback; Station keeps app id/secret only.
private struct FyersOAuthSessionCredentialValidator: BrokerCredentialValidating, Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "fyers",
              credentials.authScheme == .fyersOAuthJsonAppIdHashSession,
              !credentials.apiKey.isEmpty,
              !credentials.apiSecret.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        return .success(permissionPosture: .readOnlyConfirmed)
    }
}

/// Groww session vault is agent-owned after checksum mint; Station keeps api key marker only.
private struct GrowwChecksumSessionCredentialValidator: BrokerCredentialValidating, Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "groww",
              credentials.authScheme == .growwChecksumSession,
              !credentials.apiKey.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        return .success(permissionPosture: .readOnlyConfirmed)
    }
}

/// Upstox bearer session vault is agent-owned after browser callback; Station keeps client id/secret only.
private struct UpstoxOAuthSessionCredentialValidator: BrokerCredentialValidating, Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "upstox",
              credentials.authScheme == .upstoxOAuthBearerSession,
              !credentials.apiKey.isEmpty,
              !credentials.apiSecret.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
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
