import Foundation
import Testing
@testable import Station

@MainActor
struct AgentBrokerSyncControlTests {
    @Test func startSyncRequiresCredentialsButDoesNotForwardSecrets() async throws {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let fresh = BrokerCredentials(apiKey: "fresh-key", apiSecret: "fresh-secret")
        try store.save(credentials: fresh, for: .binanceComProd)

        let control = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)

        try await control.startSync(for: .binanceComProd)

        #expect(store.readCallCount == 1)
        #expect(runtime.lastStartedIdentity == .binanceComProd)
        #expect(runtime.startSyncCallCount == 1)
    }

    @Test func startSyncFailsWhenKeychainEmpty() async {
        let store = FakeBrokerCredentialStore()
        let runtime = FakeBrokerAgentRuntimeClient()
        let control = AgentBrokerSyncControl(credentialStore: store, runtimeClient: runtime)

        await #expect(throws: BrokerSyncStartError.missingCredentials) {
            try await control.startSync(for: .binanceComProd)
        }
        #expect(runtime.startSyncCallCount == 0)
    }

    @Test func startWirePayloadIsIdentityOnly() throws {
        let data = try BrokerSyncStartPayload.identityOnlyJSON(for: .binanceComProd)
        let json = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
        #expect(json["brokerSlug"] as? String == "binance_com")
        #expect(json["brokerConnectionId"] != nil)
        #expect(json["apiKey"] == nil)
        #expect(json["apiSecret"] == nil)
        #expect(json["tradeToken"] == nil)
        #expect(json["consumerKey"] == nil)
    }
}
