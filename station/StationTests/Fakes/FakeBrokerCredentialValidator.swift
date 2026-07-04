import Foundation
@testable import Station

final class FakeBrokerCredentialValidator: BrokerCredentialValidating, @unchecked Sendable {
    var nextResult: BrokerCredentialValidationResult = .success(permissionPosture: .readOnlyConfirmed)
    private(set) var validateCallCount = 0
    private(set) var lastValidatedCredentials: BrokerCredentials?

    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        validateCallCount += 1
        lastValidatedCredentials = credentials
        return nextResult
    }
}
