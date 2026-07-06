import Foundation

/// Routes fake `TA_FAKE_COM_*` keys to the local adapter; all other keys use live Binance.com validation.
public struct BinanceComCredentialValidator: BrokerCredentialValidating, Sendable {
    private let local: LocalBinanceComCredentialValidator
    private let live: LiveBinanceComCredentialValidator

    public init(
        local: LocalBinanceComCredentialValidator = LocalBinanceComCredentialValidator(),
        live: LiveBinanceComCredentialValidator = LiveBinanceComCredentialValidator()
    ) {
        self.local = local
        self.live = live
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        if credentials.apiKey.hasPrefix("TA_FAKE_COM_") {
            return await local.validate(credentials: credentials, identity: identity)
        }
        return await live.validate(credentials: credentials, identity: identity)
    }
}
