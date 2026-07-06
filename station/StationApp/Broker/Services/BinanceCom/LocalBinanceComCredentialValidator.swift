import Foundation

/// Deterministic Binance Global validation for CI and local development — never calls the network.
public struct LocalBinanceComCredentialValidator: BrokerCredentialValidating, Sendable {
    public enum Scenario: String, Sendable, CaseIterable {
        case readOnly = "TA_FAKE_COM_READ_ONLY"
        case tradeEnabled = "TA_FAKE_COM_TRADE"
        case unverifiable = "TA_FAKE_COM_UNVERIFIABLE"
        case withdraw = "TA_FAKE_COM_WITHDRAW"
        case invalidCredentials = "TA_FAKE_COM_INVALID"
        case networkUnavailable = "TA_FAKE_COM_NETWORK"
        case rateLimited = "TA_FAKE_COM_RATE_LIMIT"
        case brokerUnavailable = "TA_FAKE_COM_UNAVAILABLE"

        var apiKey: String { rawValue }
    }

    public init() {}

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "binance_com" else {
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
        case Scenario.rateLimited.apiKey:
            return .transientFailure(.rateLimited)
        case Scenario.brokerUnavailable.apiKey:
            return .transientFailure(.brokerUnavailable)
        default:
            return .permanentFailure(.invalidCredentials)
        }
    }
}
