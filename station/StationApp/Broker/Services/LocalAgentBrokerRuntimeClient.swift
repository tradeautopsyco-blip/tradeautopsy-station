import Foundation
import Notch

@MainActor
public protocol BrokerAgentRuntimeClient {
    /// Identity-only start — agent loads credentials from host Keychain vault (R6 / Phase 2).
    func startSync(for identity: BrokerConnectionIdentity) async throws
    func stopSync(for identity: BrokerConnectionIdentity) async throws
    func fetchRuntimeStatus(for identity: BrokerConnectionIdentity) async -> BrokerCardStatus?
}

/// Loopback wire-v1 client for broker runtime control (#13/#14).
///
/// Signing is delegated to the shared `StationWireClient` (T1 — bridge harden):
/// wire HMAC + `x-daemon-secret` are machine-integrity-only, `x-user-id` is a
/// fixed wire hint — neither is ever a Console identity.
@MainActor
public struct LocalAgentBrokerRuntimeClient: BrokerAgentRuntimeClient {
    public typealias RequestSigner = StationWireClient.RequestSigner

    private let port: UInt16
    private let session: URLSession
    private let signRequest: RequestSigner

    public init(
        port: UInt16 = AgentLoopback.port,
        session: URLSession = .shared,
        daemonSecret: String? = nil
    ) {
        self.port = port
        self.session = session
        let secret = daemonSecret ?? AgentDaemonSecret.resolveForSession()
        self.signRequest = StationWireClient.requestSigner(daemonSecret: secret)
    }

    public init(port: UInt16, session: URLSession, signRequest: @escaping RequestSigner) {
        self.port = port
        self.session = session
        self.signRequest = signRequest
    }

    public func startSync(for identity: BrokerConnectionIdentity) async throws {
        let path = "/api/daemon/broker/sync/start"
        let payload = BrokerSyncStartPayload(identity: identity)
        let body = try JSONEncoder().encode(payload)
        var request = signRequest("POST", path, body)
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (_, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            throw BrokerAgentRuntimeError.requestFailed
        }
    }

    public func stopSync(for identity: BrokerConnectionIdentity) async throws {
        _ = identity
        let path = "/api/daemon/broker/sync/stop"
        var request = signRequest("POST", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (_, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            throw BrokerAgentRuntimeError.requestFailed
        }
    }

    public func fetchRuntimeStatus(for identity: BrokerConnectionIdentity) async -> BrokerCardStatus? {
        _ = identity
        let path = "/api/daemon/broker/sync-state"
        var request = signRequest("GET", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200,
              let decoded = try? JSONDecoder().decode(BrokerAgentSyncStateResponse.self, from: data)
        else {
            return nil
        }
        return decoded.cardStatus
    }
}

public enum BrokerAgentRuntimeError: Error, Equatable {
    case requestFailed
}

private struct BrokerSyncStartPayload: Encodable {
    let brokerSlug: String
    let brokerConnectionId: String
    let environment: String
    let assetClass: String

    init(identity: BrokerConnectionIdentity) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
        assetClass = identity.assetClass
    }
}

private struct BrokerAgentSyncStateResponse: Decodable {
    let runtimeStatus: String

    var cardStatus: BrokerCardStatus? {
        switch runtimeStatus {
        case "ready_to_start": return .readyToStart
        case "syncing": return .syncing
        case "degraded": return .degraded
        case "rate_limited": return .rateLimited
        case "paused": return .paused
        default: return nil
        }
    }
}

public enum AgentLoopback {
    public static let port: UInt16 = 9137
}
