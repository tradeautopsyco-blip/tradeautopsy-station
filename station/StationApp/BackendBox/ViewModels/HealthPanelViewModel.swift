import Combine
import Foundation
import Notch

@MainActor
public final class HealthPanelViewModel: ObservableObject {
    @Published private(set) var chrome: HealthPanelChrome

    private let port: UInt16
    private let session: URLSession
    private let signRequest: StationWireClient.RequestSigner
    private let agentHealthy: () -> Bool
    private let agentMessage: () -> String?
    private let killActive: () -> Bool

    public init(
        port: UInt16 = AgentLoopback.port,
        session: URLSession = .shared,
        daemonSecret: String? = nil,
        agentHealthy: @escaping () -> Bool,
        agentMessage: @escaping () -> String?,
        killActive: @escaping () -> Bool
    ) {
        self.port = port
        self.session = session
        let secret = daemonSecret ?? AgentDaemonSecret.resolveForSession()
        self.signRequest = StationWireClient.requestSigner(daemonSecret: secret)
        self.agentHealthy = agentHealthy
        self.agentMessage = agentMessage
        self.killActive = killActive
        self.chrome = HealthPanelChrome.compose(
            agentHealthy: agentHealthy(),
            agentMessage: agentMessage(),
            vendorRows: [],
            killActive: killActive()
        )
    }

    public func refresh() async {
        let json = await fetchHealthJSON()
        let vendors = json.map { VendorFence.rows(fromHealthJSON: $0) } ?? []
        chrome = HealthPanelChrome.compose(
            agentHealthy: agentHealthy(),
            agentMessage: agentMessage(),
            vendorRows: vendors,
            killActive: killActive()
        )
    }

    private func fetchHealthJSON() async -> [String: Any]? {
        let path = "/api/daemon/health"
        var request = signRequest("GET", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200,
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return nil
        }
        return json
    }
}
