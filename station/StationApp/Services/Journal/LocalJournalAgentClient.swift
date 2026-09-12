import Foundation

@MainActor
public struct LocalJournalAgentClient: JournalAgentClient {
    private let port: UInt16
    private let session: URLSession
    private let signRequest: StationWireClient.RequestSigner
    private let isAgentHealthy: () -> Bool

    public init(
        port: UInt16 = AgentLoopback.port,
        session: URLSession = .shared,
        daemonSecret: String? = nil,
        isAgentHealthy: @escaping () -> Bool = { true }
    ) {
        self.port = port
        self.session = session
        let secret = daemonSecret ?? AgentDaemonSecret.resolveForSession()
        self.signRequest = StationWireClient.requestSigner(daemonSecret: secret)
        self.isAgentHealthy = isAgentHealthy
    }

    public func fetchWeek() async -> JournalWeekPayload? {
        guard isAgentHealthy() else { return nil }
        let path = "/api/daemon/bar/declarations"
        var request = signRequest("GET", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)?scope=week")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200
        else {
            return nil
        }
        return JournalWeekWire.decode(data)
    }
}
