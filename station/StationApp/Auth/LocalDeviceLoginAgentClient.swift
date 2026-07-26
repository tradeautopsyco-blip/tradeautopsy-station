import Foundation
import Notch

/// Loopback client for A8 Station device-login (`/api/daemon/auth/station/*`).
@MainActor
public struct LocalDeviceLoginAgentClient: DeviceLoginClient {
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

    public func begin() async throws -> DeviceLoginChallenge {
        try await postEmpty("/api/daemon/auth/station/begin")
    }

    public func complete() async throws -> StationSessionIdentity {
        try await postEmpty("/api/daemon/auth/station/complete")
    }

    public func currentSession() async throws -> StationSessionIdentity? {
        guard isAgentHealthy() else {
            throw DeviceLoginClientError.unavailable("Agent is not healthy")
        }
        let path = "/api/daemon/auth/station/session"
        var request = signRequest("GET", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw DeviceLoginClientError.unavailable("Invalid session response")
        }
        if http.statusCode == 401 {
            return nil
        }
        guard http.statusCode == 200 else {
            throw DeviceLoginClientError.rejected(Self.message(from: data) ?? "Session check failed")
        }
        let payload = try JSONDecoder().decode(StationSessionPayload.self, from: data)
        guard payload.signedIn != false, let email = payload.email, let profileID = payload.profileID else {
            return nil
        }
        return StationSessionIdentity(
            profileID: profileID,
            email: email,
            aud: payload.aud ?? "station"
        )
    }

    public func signOut() async throws {
        let _: SignedOutPayload = try await postEmpty("/api/daemon/auth/station/sign-out")
    }

    private func postEmpty<T: Decodable>(_ path: String) async throws -> T {
        guard isAgentHealthy() else {
            throw DeviceLoginClientError.unavailable("Agent is not healthy")
        }
        var request = signRequest("POST", path, Data())
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw DeviceLoginClientError.unavailable("Invalid response")
        }
        guard (200...299).contains(http.statusCode) else {
            throw DeviceLoginClientError.rejected(Self.message(from: data) ?? "Request failed (\(http.statusCode))")
        }
        return try JSONDecoder().decode(T.self, from: data)
    }

    private static func message(from data: Data) -> String? {
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return nil
        }
        return object["message"] as? String
    }
}

private struct StationSessionPayload: Decodable {
    let signedIn: Bool?
    let profileID: String?
    let email: String?
    let aud: String?

    enum CodingKeys: String, CodingKey {
        case signedIn = "signed_in"
        case profileID = "profile_id"
        case email
        case aud
    }
}

private struct SignedOutPayload: Decodable {
    let signedIn: Bool?

    enum CodingKeys: String, CodingKey {
        case signedIn = "signed_in"
    }
}
