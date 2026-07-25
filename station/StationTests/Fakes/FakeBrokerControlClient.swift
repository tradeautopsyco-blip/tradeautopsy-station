import Foundation
@testable import Station

@MainActor
final class FakeBrokerControlClient: BrokerControlling {
    enum Scenario {
        case notConfigured
        case agentOfflineWithCredentials
        case readyToStart
        case syncing
    }

    var scenario: Scenario = .notConfigured

    private(set) var loadSnapshotCallCount = 0
    private(set) var startSyncCallCount = 0
    private(set) var stopSyncCallCount = 0
    private(set) var lastStartedIdentity: BrokerConnectionIdentity?
    private(set) var lastStoppedIdentity: BrokerConnectionIdentity?

    func loadSnapshot() async -> BrokerControlSnapshot {
        loadSnapshotCallCount += 1
        switch scenario {
        case .notConfigured:
            return BrokerControlSnapshot(
                configuredConnections: [],
                agentAvailable: true,
                runtimeStatusByConnectionID: [:]
            )
        case .agentOfflineWithCredentials:
            return BrokerControlSnapshot(
                configuredConnections: [
                    BrokerConfiguredConnection(
                        identity: .binanceComProd,
                        displayName: "Binance.com",
                        lastValidatedAt: Date(timeIntervalSince1970: 1_700_000_000),
                        lastSyncSummary: "Last sync: 2 fills imported"
                    )
                ],
                agentAvailable: false,
                runtimeStatusByConnectionID: [:]
            )
        case .readyToStart:
            return BrokerControlSnapshot(
                configuredConnections: [
                    BrokerConfiguredConnection(
                        identity: .binanceComProd,
                        displayName: "Binance.com",
                        lastValidatedAt: Date(timeIntervalSince1970: 1_700_000_000)
                    )
                ],
                agentAvailable: true,
                runtimeStatusByConnectionID: [:]
            )
        case .syncing:
            let identity = BrokerConnectionIdentity.binanceComProd
            return BrokerControlSnapshot(
                configuredConnections: [
                    BrokerConfiguredConnection(
                        identity: identity,
                        displayName: "Binance.com",
                        lastValidatedAt: Date(timeIntervalSince1970: 1_700_000_000)
                    )
                ],
                agentAvailable: true,
                runtimeStatusByConnectionID: [
                    identity.brokerConnectionID.uuidString: .syncing
                ]
            )
        }
    }

    func startSync(for identity: BrokerConnectionIdentity) async throws {
        startSyncCallCount += 1
        lastStartedIdentity = identity
    }

    func stopSync(for identity: BrokerConnectionIdentity) async throws {
        stopSyncCallCount += 1
        lastStoppedIdentity = identity
    }

    private(set) var deleteConnectionCallCount = 0

    func deleteConnection(
        for identity: BrokerConnectionIdentity,
        connectController: BrokerConnectController
    ) async throws {
        deleteConnectionCallCount += 1
        try connectController.deleteSavedCredentials()
    }
}
