import Foundation
import Testing
@testable import Station

@MainActor
struct BinanceUSCredentialValidationTests {
    private let identity = BrokerConnectionIdentity.binanceUSProd

    private func localResult(for scenario: LocalBinanceUSCredentialValidator.Scenario) async
        -> BrokerCredentialValidationResult
    {
        let validator = LocalBinanceUSCredentialValidator()
        return await validator.validate(
            credentials: BrokerCredentials(apiKey: scenario.apiKey, apiSecret: "secret"),
            identity: identity
        )
    }

    @Test func localFakeReadOnlyKeyReturnsReadOnlyConfirmedPosture() async {
        let result = await localResult(for: .readOnly)
        #expect(result == .success(permissionPosture: .readOnlyConfirmed))
    }

    @Test func localFakeTradeKeyReturnsTradeEnabledPosture() async {
        let result = await localResult(for: .tradeEnabled)
        #expect(result == .success(permissionPosture: .tradeEnabled))
    }

    @Test func localFakeUnverifiableKeyReturnsUnverifiablePosture() async {
        let result = await localResult(for: .unverifiable)
        #expect(result == .success(permissionPosture: .unverifiable))
    }

    @Test func localFakeWithdrawKeyReturnsWithdrawDetectedPosture() async {
        let result = await localResult(for: .withdraw)
        #expect(result == .success(permissionPosture: .withdrawDetected))
    }

    @Test func localFakeInvalidKeyReturnsPermanentCredentialFailure() async {
        let result = await localResult(for: .invalidCredentials)
        #expect(result == .permanentFailure(.invalidCredentials))
    }

    @Test func localFakeNetworkKeyReturnsTransientNetworkFailure() async {
        let result = await localResult(for: .networkUnavailable)
        #expect(result == .transientFailure(.networkUnavailable))
    }

    @Test func localFakeRateLimitKeyReturnsTransientRateLimitFailure() async {
        let result = await localResult(for: .rateLimited)
        #expect(result == .transientFailure(.rateLimited))
    }

    @Test func localFakeUnavailableKeyReturnsTransientAvailabilityFailure() async {
        let result = await localResult(for: .brokerUnavailable)
        #expect(result == .transientFailure(.brokerUnavailable))
    }

    @Test func permissionClassifierMapsWithdrawBeforeTrade() {
        let posture = BinanceUSPermissionClassifier.classify(
            BinanceUSApiRestrictions(
                enableReading: true,
                enableSpotAndMarginTrading: true,
                enableWithdrawals: true
            )
        )
        #expect(posture == .withdrawDetected)
    }

    @Test func permissionClassifierMapsReadOnlyWhenOnlyReadingEnabled() {
        let posture = BinanceUSPermissionClassifier.classify(
            BinanceUSApiRestrictions(
                enableReading: true,
                enableSpotAndMarginTrading: false,
                enableWithdrawals: false
            )
        )
        #expect(posture == .readOnlyConfirmed)
    }

    @Test func liveValidatorMapsApiRestrictionsResponseToReadOnlyPosture() async {
        let body = """
        {"enableReading":true,"enableSpotAndMarginTrading":false,"enableWithdrawals":false}
        """.data(using: .utf8)!
        let transport = StubBinanceUSValidationTransport(
            response: BinanceUSValidationHTTPResponse(statusCode: 200, body: body)
        )
        let validator = LiveBinanceUSCredentialValidator(transport: transport)
        let result = await validator.validate(
            credentials: BrokerCredentials(apiKey: "live-key", apiSecret: "live-secret"),
            identity: identity
        )
        #expect(result == .success(permissionPosture: .readOnlyConfirmed))
    }

    @Test func liveValidatorClassifies401AsInvalidCredentials() async {
        let body = """
        {"code":-2015,"msg":"Invalid API-key, IP, or permissions for action."}
        """.data(using: .utf8)!
        let transport = StubBinanceUSValidationTransport(
            response: BinanceUSValidationHTTPResponse(statusCode: 401, body: body)
        )
        let validator = LiveBinanceUSCredentialValidator(transport: transport)
        let result = await validator.validate(
            credentials: BrokerCredentials(apiKey: "bad", apiSecret: "bad"),
            identity: identity
        )
        #expect(result == .permanentFailure(.invalidCredentials))
    }

    @Test func liveValidatorClassifies429AsRateLimited() async {
        let transport = StubBinanceUSValidationTransport(
            response: BinanceUSValidationHTTPResponse(statusCode: 429, body: Data())
        )
        let validator = LiveBinanceUSCredentialValidator(transport: transport)
        let result = await validator.validate(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            identity: identity
        )
        #expect(result == .transientFailure(.rateLimited))
    }

    @Test func compositeValidatorRoutesFakeKeysToLocalAdapter() async {
        let validator = BinanceUSCredentialValidator()
        let result = await validator.validate(
            credentials: BrokerCredentials(
                apiKey: LocalBinanceUSCredentialValidator.Scenario.tradeEnabled.apiKey,
                apiSecret: "secret"
            ),
            identity: identity
        )
        #expect(result == .success(permissionPosture: .tradeEnabled))
    }

    @Test func localFakeConnectFlowPersistsReadOnlyCredentials() async {
        let store = FakeBrokerCredentialStore()
        let sync = FakeBrokerSyncControl()
        let metadataStore = UserDefaultsBrokerMetadataStore(
            defaults: UserDefaults(suiteName: "StationTests.BinanceUS.Connect.\(UUID().uuidString)")!
        )
        let controller = BrokerConnectController(
            identity: identity,
            credentialStore: store,
            validator: BinanceUSCredentialValidator(),
            syncControl: sync,
            metadataStore: metadataStore
        )
        controller.updateFields(
            apiKey: LocalBinanceUSCredentialValidator.Scenario.readOnly.apiKey,
            apiSecret: "secret"
        )

        let outcome = await controller.connect()

        #expect(outcome == .connected(permissionWarning: nil))
        #expect(store.saveCallCount == 1)
        #expect(sync.startSyncCallCount == 1)
    }
}

private struct StubBinanceUSValidationTransport: BinanceUSValidationTransport {
    let response: BinanceUSValidationHTTPResponse

    func fetchApiRestrictions(apiKey: String, apiSecret: String) async throws -> BinanceUSValidationHTTPResponse {
        _ = apiKey
        _ = apiSecret
        return response
    }
}

private struct FailingBinanceUSValidationTransport: BinanceUSValidationTransport {
    enum TransportFailure: Error { case offline }

    func fetchApiRestrictions(apiKey: String, apiSecret: String) async throws -> BinanceUSValidationHTTPResponse {
        _ = apiKey
        _ = apiSecret
        throw TransportFailure.offline
    }
}
