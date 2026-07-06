import CryptoKit
import Foundation

@MainActor
public struct LocalTodayAgentClient: TodayAgentClient {
    private let port: UInt16
    private let session: URLSession
    private let signRequest: LocalAgentBrokerRuntimeClient.RequestSigner
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
        self.signRequest = AgentWireSigner(secret: secret).signedRequest
        self.isAgentHealthy = isAgentHealthy
    }

    public func fetchToday() async -> TodayAgentPayload? {
        guard isAgentHealthy() else { return nil }
        let path = "/api/daemon/today"
        var request = signRequest("GET", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200,
              let decoded = try? JSONDecoder().decode(TodayAgentPayload.self, from: data)
        else {
            return nil
        }
        return decoded
    }
}
