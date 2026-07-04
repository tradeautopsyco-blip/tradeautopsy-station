import Foundation
@testable import Station

final class SlowFakeBrokerCredentialValidator: BrokerCredentialValidating, @unchecked Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        try? await Task.sleep(for: .milliseconds(200))
        return .success(permissionPosture: .readOnlyConfirmed)
    }
}
