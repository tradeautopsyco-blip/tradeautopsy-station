import Foundation
import Testing
@testable import Station

@MainActor
@Suite(.serialized)
struct AgentSupervisorTests {
    // Backoff intervals increase exponentially between restart attempts
    @Test func backoffIntervalsIncreaseExponentially() {
        var tracker = AgentRestartTracker()

        #expect(tracker.nextBackoffDelay() == 1)
        #expect(tracker.nextBackoffDelay() == 2)
        #expect(tracker.nextBackoffDelay() == 4)
        #expect(tracker.nextBackoffDelay() == 8)
    }

    // Restart count resets after successful attach
    @Test func restartCountResetsAfterSuccessfulAttach() {
        var tracker = AgentRestartTracker()
        _ = tracker.nextBackoffDelay()
        _ = tracker.nextBackoffDelay()
        tracker.recordCrash(at: Date())

        tracker.resetOnSuccessfulAttach()

        #expect(tracker.restartAttemptCount == 0)
        #expect(tracker.crashTimestamps.isEmpty)
        #expect(tracker.nextBackoffDelay() == 1)
    }

    // Crash loop window is rolling 60s (not cumulative)
    @Test func crashLoopWindowIsRollingSixtySeconds() {
        var tracker = AgentRestartTracker()
        let base = Date(timeIntervalSinceReferenceDate: 100_000)

        tracker.recordCrash(at: base)
        tracker.recordCrash(at: base.addingTimeInterval(20))
        tracker.recordCrash(at: base.addingTimeInterval(40))
        #expect(tracker.crashLoopExceeded(at: base.addingTimeInterval(40)))

        var agedTracker = AgentRestartTracker()
        agedTracker.recordCrash(at: base)
        agedTracker.recordCrash(at: base.addingTimeInterval(20))
        agedTracker.recordCrash(at: base.addingTimeInterval(61))
        #expect(agedTracker.crashLoopExceeded(at: base.addingTimeInterval(61)) == false)
    }

    @Test func attachWaitsForAgentHealthBeforeWarning() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .agentWireRejected)
        MockLoopbackURLProtocol.responses = [.agentWireRejected, .healthyAgent]

        let supervisor = makeSupervisor()
        await supervisor.start()

        #expect(supervisor.isHealthy)
        #expect(supervisor.currentWarning == nil)
        #expect(MockLoopbackURLProtocol.requestCount >= 2)
    }

    @Test func agentWireRejectionIsNotPortCollision() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .agentWireRejected)

        let supervisor = makeSupervisor()
        await supervisor.start()

        #expect(supervisor.isHealthy == false)
        #expect(supervisor.currentWarning?.reason != .portCollisionNonAgent)
    }

    @Test func foreignHTTPResponseSurfacesPortCollision() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .foreignProcess)

        let supervisor = makeSupervisor()
        await supervisor.start()

        #expect(supervisor.isHealthy == false)
        #expect(supervisor.currentWarning?.reason == .portCollisionNonAgent)
    }

    @Test func runtimeSupervisionClearsStalePortCollisionWarning() async throws {
        MockLoopbackURLProtocol.reset(defaultResponse: .foreignProcess)

        let supervisor = makeSupervisor()
        await supervisor.start()
        #expect(supervisor.currentWarning?.reason == .portCollisionNonAgent)

        MockLoopbackURLProtocol.defaultResponse = .healthyAgent
        try await Task.sleep(for: .seconds(2.5))
        await Task.yield()

        #expect(supervisor.isHealthy)
        #expect(supervisor.currentWarning == nil)
    }

    /// Independent measurement 2026-09-10: bundled agent accepted 9137 at 10.05s.
    @Test func launchWaitExceedsMeasuredAgentHttpBind() {
        #expect(AgentSupervisor.launchTimeout > 10)
        #expect(AgentSupervisor.launchTimeout >= 25)
    }

    @Test func slowLoopbackHealthWithinLaunchWaitIsHealthy() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .healthyAgent)
        MockLoopbackURLProtocol.refuseUntil = Date().addingTimeInterval(0.35)
        let supervisor = makeSupervisor(launchTimeout: 1.0, attachOnly: true)
        await supervisor.start()
        #expect(supervisor.isHealthy)
        #expect(supervisor.currentWarning == nil)
        await supervisor.shutdown()
    }

    @Test func slowLoopbackHealthPastLaunchWaitStaysOffline() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .healthyAgent)
        MockLoopbackURLProtocol.refuseUntil = Date().addingTimeInterval(2.0)
        let supervisor = makeSupervisor(launchTimeout: 0.4, attachOnly: true)
        await supervisor.start()
        #expect(supervisor.isHealthy == false)
        #expect(supervisor.currentWarning?.reason == .launchTimeout)
        await supervisor.shutdown()
    }

    private func makeSupervisor(
        launchTimeout: TimeInterval = AgentSupervisor.launchTimeout,
        attachOnly: Bool? = nil
    ) -> AgentSupervisor {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [MockLoopbackURLProtocol.self]
        let session = URLSession(configuration: config)
        return AgentSupervisor(
            session: session,
            daemonSecret: "test-secret",
            launchTimeout: launchTimeout,
            attachOnly: attachOnly
        )
    }
}

private enum MockLoopbackResponse {
    case healthyAgent
    case agentWireRejected
    case foreignProcess
}

private final class MockLoopbackURLProtocol: URLProtocol {
    nonisolated(unsafe) static var responses: [MockLoopbackResponse] = []
    nonisolated(unsafe) static var defaultResponse: MockLoopbackResponse = .foreignProcess
    nonisolated(unsafe) static var requestCount = 0
    nonisolated(unsafe) static var refuseUntil: Date?

    nonisolated static func reset(defaultResponse: MockLoopbackResponse = .foreignProcess) {
        responses = []
        self.defaultResponse = defaultResponse
        requestCount = 0
        refuseUntil = nil
    }

    override class func canInit(with request: URLRequest) -> Bool {
        request.url?.host == "127.0.0.1"
    }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest {
        request
    }

    override func startLoading() {
        MockLoopbackURLProtocol.requestCount += 1
        if let until = MockLoopbackURLProtocol.refuseUntil, Date() < until {
            client?.urlProtocol(self, didFailWithError: URLError(.cannotConnectToHost))
            return
        }
        let responseKind = MockLoopbackURLProtocol.responses.isEmpty
            ? MockLoopbackURLProtocol.defaultResponse
            : MockLoopbackURLProtocol.responses.removeFirst()

        let (http, data) = Self.payload(for: responseKind)
        client?.urlProtocol(self, didReceive: http, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: data)
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}

    private static func payload(for kind: MockLoopbackResponse) -> (HTTPURLResponse, Data) {
        let url = URL(string: "http://127.0.0.1:9137/api/daemon/health")!
        switch kind {
        case .healthyAgent:
            let body = #"{"status":"ok","daemon":"agent"}"#.data(using: .utf8)!
            let http = HTTPURLResponse(
                url: url,
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Content-Type": "application/json"]
            )!
            return (http, body)
        case .agentWireRejected:
            let body = #"{"error_class":"SIG_INVALID","message":"x-daemon-secret mismatch or missing"}"#
                .data(using: .utf8)!
            let http = HTTPURLResponse(
                url: url,
                statusCode: 401,
                httpVersion: nil,
                headerFields: ["Content-Type": "application/json"]
            )!
            return (http, body)
        case .foreignProcess:
            let body = "<html>not an agent</html>".data(using: .utf8)!
            let http = HTTPURLResponse(
                url: url,
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Content-Type": "text/html"]
            )!
            return (http, body)
        }
    }
}
