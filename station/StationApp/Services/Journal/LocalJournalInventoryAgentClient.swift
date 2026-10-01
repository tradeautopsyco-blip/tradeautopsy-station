import Foundation

@MainActor
public struct LocalJournalInventoryAgentClient: JournalInventoryAgentClient {
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

    public func fetchWeekTripCites(weekStart: String?, weekEnd: String?) async -> [JournalClosedTripInventoryRow] {
        guard isAgentHealthy() else { return [] }
        var query: [String] = ["scope=week"]
        if let start = JournalTripCiteInventory.localDayFromWeekBound(weekStart) {
            query.append("week_start=\(start)")
        }
        if let end = JournalTripCiteInventory.localDayFromWeekBound(weekEnd) {
            query.append("week_end=\(end)")
        }
        let path = "/api/daemon/journal/trip-cites"
        let qs = query.joined(separator: "&")
        var request = signRequest("GET", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)?\(qs)")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200
        else {
            return []
        }
        return JournalTripCiteWire.decode(data)
    }
}

enum JournalTripCiteWire {
    private struct Envelope: Decodable {
        var items: [Item]?
    }

    private struct Item: Decodable {
        var declarationId: String?
        var declaration_id: String?
        var net: Double?
        var currency: String?
    }

    static func decode(_ data: Data) -> [JournalClosedTripInventoryRow] {
        guard let env = try? JSONDecoder().decode(Envelope.self, from: data) else { return [] }
        return (env.items ?? []).compactMap { item in
            let id = (item.declarationId ?? item.declaration_id ?? "")
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard !id.isEmpty, let net = item.net, net.isFinite else { return nil }
            let ccy = (item.currency ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            guard !ccy.isEmpty else { return nil }
            return JournalClosedTripInventoryRow(declarationId: id, net: net, currency: ccy)
        }
    }
}
