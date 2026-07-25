import Foundation
@testable import Station

@MainActor
final class FakeBrokerAgentRuntimeClient: BrokerAgentRuntimeClient {
    var runtimeStatus: BrokerCardStatus = .readyToStart

    private(set) var startSyncCallCount = 0
    private(set) var stopSyncCallCount = 0
    private(set) var fetchRuntimeStatusCallCount = 0
    private(set) var lastStartedIdentity: BrokerConnectionIdentity?
    private(set) var lastStoppedIdentity: BrokerConnectionIdentity?

    func startSync(for identity: BrokerConnectionIdentity) async throws {
        startSyncCallCount += 1
        lastStartedIdentity = identity
        runtimeStatus = .syncing
    }

    func stopSync(for identity: BrokerConnectionIdentity) async throws {
        stopSyncCallCount += 1
        lastStoppedIdentity = identity
        runtimeStatus = .paused
    }

    func fetchRuntimeStatus(for identity: BrokerConnectionIdentity) async -> BrokerCardStatus? {
        fetchRuntimeStatusCallCount += 1
        _ = identity
        return runtimeStatus
    }
}
