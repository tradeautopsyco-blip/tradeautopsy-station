import Foundation
import Testing
@testable import Notch

struct BarLiveStatePollingTargetTests {
    @Test func syncBarLiveStatePollingExtractsAgentNotHostedConsole() {
        let url = BarLiveStatePollingTarget.url(agentBase: "http://127.0.0.1:9137")
        #expect(url?.path == "/api/daemon/bar/live-state")
        #expect(url?.absoluteString.contains("/api/bar/v1/live-state") == false)
        #expect(BarLiveStatePollingTarget.path != "/api/bar/v1/live-state")
        #expect(BarLiveStatePollingTarget.isHostedConsoleLiveState(url!) == false)
    }
}
