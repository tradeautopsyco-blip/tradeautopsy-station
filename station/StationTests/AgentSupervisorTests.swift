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

    @Test func isKillLatchedReadsSignedAgentState() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .healthyAgent)
        MockLoopbackURLProtocol.killSwitchLatched = true
        let supervisor = makeSupervisor(attachOnly: true)
        #expect(await supervisor.isKillLatched())
    }

    @Test func stationQuitDoesNotKillLatchedAgent() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .healthyAgent)
        MockLoopbackURLProtocol.killSwitchLatched = true
        let supervisor = makeSupervisor(attachOnly: true)
        await supervisor.start()
        let requestsBeforeShutdown = MockLoopbackURLProtocol.requestCount
        await supervisor.shutdown()
        #expect(MockLoopbackURLProtocol.requestCount > requestsBeforeShutdown)
    }

    @Test func restartAgentBlockedWhenKillLatched() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .healthyAgent)
        MockLoopbackURLProtocol.killSwitchLatched = true
        let supervisor = makeSupervisor(attachOnly: true)
        await supervisor.start()
        let outcome = await supervisor.restartAgent()
        #expect(outcome == .blockedKillLatched)
        #expect(supervisor.isHealthy)
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

    // POSIX probe accepts a bound loopback listener and refuses a dead port —
    // and produces no CFNetwork stderr noise either way.
    @Test func tcpProbeAcceptsListenerAndRefusesDeadPort() throws {
        let fd = socket(AF_INET, SOCK_STREAM, 0)
        #expect(fd >= 0)
        defer { close(fd) }

        var yes: Int32 = 1
        setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, socklen_t(MemoryLayout<Int32>.size))
        var addr = sockaddr_in()
        addr.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = 0
        addr.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))
        let bound = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                Darwin.bind(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        #expect(bound == 0)
        #expect(listen(fd, 1) == 0)

        var actual = sockaddr_in()
        var len = socklen_t(MemoryLayout<sockaddr_in>.size)
        withUnsafeMutablePointer(to: &actual) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                _ = getsockname(fd, $0, &len)
            }
        }
        let port = UInt16(bigEndian: actual.sin_port)

        #expect(LoopbackTCPProbe.connectable(port: port))
        close(fd)
        // Port released: connect must now refuse.
        #expect(LoopbackTCPProbe.connectable(port: port) == false)
    }

    // Probe gate: with the port dead, no signed request is ever attempted.
    @Test func deadPortSkipsSignedRequests() async {
        MockLoopbackURLProtocol.reset(defaultResponse: .healthyAgent)
        let probeCalls = ProbeCounter()
        let supervisor = AgentSupervisor(
            session: .shared,
            daemonSecret: "test-secret",
            launchTimeout: 0.4,
            attachOnly: true,
            tcpProbe: { _ in
                probeCalls.increment()
                return false
            }
        )
        await supervisor.start()
        #expect(probeCalls.count > 0)
        #expect(MockLoopbackURLProtocol.requestCount == 0)
        #expect(supervisor.isHealthy == false)
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
            attachOnly: attachOnly,
            // MockLoopbackURLProtocol simulates the port; no real listener needed.
            tcpProbe: { _ in true }
        )
    }
}

private enum MockLoopbackResponse {
    case healthyAgent
    case agentWireRejected
    case foreignProcess
}

private final class ProbeCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var _count = 0
    var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return _count
    }
    func increment() {
        lock.lock()
        _count += 1
        lock.unlock()
    }
}

private final class MockLoopbackURLProtocol: URLProtocol {
    nonisolated(unsafe) static var responses: [MockLoopbackResponse] = []
    nonisolated(unsafe) static var defaultResponse: MockLoopbackResponse = .foreignProcess
    nonisolated(unsafe) static var requestCount = 0
    nonisolated(unsafe) static var refuseUntil: Date?
    nonisolated(unsafe) static var killSwitchLatched = false

    nonisolated static func reset(defaultResponse: MockLoopbackResponse = .foreignProcess) {
        responses = []
        self.defaultResponse = defaultResponse
        requestCount = 0
        refuseUntil = nil
        killSwitchLatched = false
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

        if let url = request.url?.absoluteString,
           url.contains("/api/daemon/kill-switch/state") {
            let latched = MockLoopbackURLProtocol.killSwitchLatched
            let body = #"{"active":\#(latched ? "true" : "false"),"dns_active":false}"#
                .data(using: .utf8)!
            let http = HTTPURLResponse(
                url: request.url!,
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Content-Type": "application/json"]
            )!
            client?.urlProtocol(self, didReceive: http, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: body)
            client?.urlProtocolDidFinishLoading(self)
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
