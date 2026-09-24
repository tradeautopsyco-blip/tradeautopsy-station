import Foundation

/// Deterministic Phase 4 crypto spot validation for CI — never calls the network.
public struct LocalPhase4CryptoCredentialValidator: BrokerCredentialValidating, Sendable {
    public enum Scenario: String, Sendable, CaseIterable {
        case readOnly = "TA_FAKE_CRYPTO_READ_ONLY"
        case tradeEnabled = "TA_FAKE_CRYPTO_TRADE"
        case unverifiable = "TA_FAKE_CRYPTO_UNVERIFIABLE"
        case withdraw = "TA_FAKE_CRYPTO_WITHDRAW"
        case invalidCredentials = "TA_FAKE_CRYPTO_INVALID"
        case networkUnavailable = "TA_FAKE_CRYPTO_NETWORK"

        var apiKey: String { rawValue }
    }

    private let expectedSlug: String

    public init(expectedSlug: String) {
        self.expectedSlug = expectedSlug
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == expectedSlug else {
            return .permanentFailure(.invalidCredentials)
        }

        switch credentials.apiKey {
        case Scenario.readOnly.apiKey:
            return .success(permissionPosture: .readOnlyConfirmed)
        case Scenario.tradeEnabled.apiKey:
            return .success(permissionPosture: .tradeEnabled)
        case Scenario.unverifiable.apiKey:
            return .success(permissionPosture: .unverifiable)
        case Scenario.withdraw.apiKey:
            return .success(permissionPosture: .withdrawDetected)
        case Scenario.invalidCredentials.apiKey:
            return .permanentFailure(.invalidCredentials)
        case Scenario.networkUnavailable.apiKey:
            return .transientFailure(.networkUnavailable)
        default:
            return .permanentFailure(.invalidCredentials)
        }
    }
}

public struct BybitCredentialValidator: BrokerCredentialValidating, Sendable {
    private let local: LocalPhase4CryptoCredentialValidator

    public init(local: LocalPhase4CryptoCredentialValidator = LocalPhase4CryptoCredentialValidator(expectedSlug: "bybit")) {
        self.local = local
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "bybit",
              credentials.authScheme == .hmacApiKeySecret,
              !credentials.apiKey.isEmpty,
              !credentials.apiSecret.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        if credentials.apiKey.hasPrefix("TA_FAKE_CRYPTO_") {
            return await local.validate(credentials: credentials, identity: identity)
        }
        return .success(permissionPosture: .unverifiable)
    }
}

public struct KrakenCredentialValidator: BrokerCredentialValidating, Sendable {
    private let local: LocalPhase4CryptoCredentialValidator

    public init(local: LocalPhase4CryptoCredentialValidator = LocalPhase4CryptoCredentialValidator(expectedSlug: "kraken")) {
        self.local = local
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "kraken",
              credentials.authScheme == .krakenSpotNonceSession,
              !credentials.apiKey.isEmpty,
              !credentials.apiSecret.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        if credentials.apiKey.hasPrefix("TA_FAKE_CRYPTO_") {
            return await local.validate(credentials: credentials, identity: identity)
        }
        return .success(permissionPosture: .unverifiable)
    }
}

public struct OkxComCredentialValidator: BrokerCredentialValidating, Sendable {
    private let local: LocalPhase4CryptoCredentialValidator

    public init(local: LocalPhase4CryptoCredentialValidator = LocalPhase4CryptoCredentialValidator(expectedSlug: "okx_com")) {
        self.local = local
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "okx_com",
              credentials.authScheme == .okxPassphraseSession,
              !credentials.apiKey.isEmpty,
              !credentials.apiSecret.isEmpty,
              let passphrase = credentials.passphrase,
              !passphrase.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        if credentials.apiKey.hasPrefix("TA_FAKE_CRYPTO_") {
            return await local.validate(credentials: credentials, identity: identity)
        }
        return .success(permissionPosture: .unverifiable)
    }
}

public struct CoinbaseAdvancedCredentialValidator: BrokerCredentialValidating, Sendable {
    private let local: LocalPhase4CryptoCredentialValidator

    public init(
        local: LocalPhase4CryptoCredentialValidator = LocalPhase4CryptoCredentialValidator(
            expectedSlug: "coinbase_advanced"
        )
    ) {
        self.local = local
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "coinbase_advanced",
              credentials.authScheme == .coinbaseJwtEs256Session,
              !credentials.apiKey.isEmpty,
              let pem = credentials.pemPrivateKey,
              !pem.isEmpty
        else {
            return .permanentFailure(.invalidCredentials)
        }
        if credentials.apiKey.hasPrefix("TA_FAKE_CRYPTO_") {
            return await local.validate(credentials: credentials, identity: identity)
        }
        return .success(permissionPosture: .unverifiable)
    }
}
