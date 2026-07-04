import Foundation
@testable import Station

@MainActor
final class FakeBrokerSyncControl: BrokerSyncControlling {
    private(set) var startSyncCallCount = 0
    private(set) var lastStartedIdentity: BrokerConnectionIdentity?

    func startSync(for identity: BrokerConnectionIdentity) async throws {
        startSyncCallCount += 1
        lastStartedIdentity = identity
    }
}
