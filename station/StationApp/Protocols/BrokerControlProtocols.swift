import Foundation

@MainActor
public protocol BrokerControlling: AnyObject {
    func loadSnapshot() async -> BrokerControlSnapshot
    func startSync(for identity: BrokerConnectionIdentity) async throws
    func stopSync(for identity: BrokerConnectionIdentity) async throws
    func deleteConnection(
        for identity: BrokerConnectionIdentity,
        connectController: BrokerConnectController
    ) async throws
}
