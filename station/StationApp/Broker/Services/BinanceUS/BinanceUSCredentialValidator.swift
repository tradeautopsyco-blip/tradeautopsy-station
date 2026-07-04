import Foundation

/// Routes fake `TA_FAKE_*` keys to the local adapter; all other keys use live Binance.US validation.
public struct BinanceUSCredentialValidator: BrokerCredentialValidating, Sendable {
    private let local: LocalBinanceUSCredentialValidator
    private let live: LiveBinanceUSCredentialValidator

    public init(
        local: LocalBinanceUSCredentialValidator = LocalBinanceUSCredentialValidator(),
        live: LiveBinanceUSCredentialValidator = LiveBinanceUSCredentialValidator()
    ) {
        self.local = local
        self.live = live
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        if credentials.apiKey.hasPrefix("TA_FAKE_") {
            return await local.validate(credentials: credentials, identity: identity)
        }
        return await live.validate(credentials: credentials, identity: identity)
    }
}
