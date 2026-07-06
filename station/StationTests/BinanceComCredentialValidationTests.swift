import Foundation
import Testing
@testable import Station

struct BinanceComCredentialValidationTests {
    @Test func permissionClassifierReadOnly() {
        let posture = BinanceComPermissionClassifier.classify(
            BinanceComApiRestrictions(
                enableReading: true,
                enableSpotAndMarginTrading: false,
                enableWithdrawals: false
            )
        )
        #expect(posture == .readOnlyConfirmed)
    }

    @Test func permissionClassifierWithdrawTakesPrecedence() {
        let posture = BinanceComPermissionClassifier.classify(
            BinanceComApiRestrictions(
                enableReading: true,
                enableSpotAndMarginTrading: true,
                enableWithdrawals: true
            )
        )
        #expect(posture == .withdrawDetected)
    }

    @Test func liveValidatorMapsReadOnlyRestrictions() async {
        let body = Data("""
        {"enableReading":true,"enableSpotAndMarginTrading":false,"enableWithdrawals":false}
        """.utf8)
        let transport = StubBinanceComValidationTransport(
            response: BinanceComValidationHTTPResponse(statusCode: 200, body: body)
        )
        let validator = LiveBinanceComCredentialValidator(transport: transport)
        let result = await validator.validate(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            identity: .binanceComProd
        )
        #expect(result == .success(permissionPosture: .readOnlyConfirmed))
    }

    @Test func liveValidatorRejectsWrongBrokerSlug() async {
        let validator = LiveBinanceComCredentialValidator()
        let result = await validator.validate(
            credentials: BrokerCredentials(apiKey: "key", apiSecret: "secret"),
            identity: .binanceUSProd
        )
        #expect(result == .permanentFailure(.invalidCredentials))
    }

    @Test func localFakeReadOnlyScenario() async {
        let validator = LocalBinanceComCredentialValidator()
        let result = await validator.validate(
            credentials: BrokerCredentials(
                apiKey: LocalBinanceComCredentialValidator.Scenario.readOnly.apiKey,
                apiSecret: "secret"
            ),
            identity: .binanceComProd
        )
        #expect(result == .success(permissionPosture: .readOnlyConfirmed))
    }

    @Test func routerValidatorSelectsBinanceCom() async {
        let validator = BrokerConnectServices.validator(for: "binance_com")
        let result = await validator.validate(
            credentials: BrokerCredentials(
                apiKey: LocalBinanceComCredentialValidator.Scenario.withdraw.apiKey,
                apiSecret: "secret"
            ),
            identity: .binanceComProd
        )
        #expect(result == .success(permissionPosture: .withdrawDetected))
    }
}

private struct StubBinanceComValidationTransport: BinanceComValidationTransport {
    let response: BinanceComValidationHTTPResponse

    func fetchApiRestrictions(apiKey: String, apiSecret: String) async throws -> BinanceComValidationHTTPResponse {
        _ = apiKey
        _ = apiSecret
        return response
    }
}
