import Foundation
import Testing
@testable import Station

@MainActor
struct AgentBrokerSyncControlTests {
    @Test func startSyncReadsCredentialsFreshFromStore() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let stale = BrokerCredentials(apiKey: "stale-key", apiSecret: "stale-secret")
        let fresh = BrokerCredentials(apiKey: "fresh-key", apiSecret: "fresh-secret")
        try store.save(credentials: stale, for: .binanceUSProd)
        try store.save(credentials: fresh, for: .binanceUSProd)

        let control = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)

        try await control.startSync(for: .binanceUSProd)

        #expect(store.readCallCount == 1)
        #expect(runtime.lastStartedCredentials == fresh)
    }
}
