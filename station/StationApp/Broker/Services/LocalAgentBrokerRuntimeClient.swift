import Foundation

@MainActor
public protocol BrokerAgentRuntimeClient {
    /// Identity-only start — agent loads credentials from host Keychain vault (R6 / Phase 2).
    func startSync(for identity: BrokerConnectionIdentity) async throws
    func stopSync(for identity: BrokerConnectionIdentity) async throws
    func fetchRuntimeStatus(for identity: BrokerConnectionIdentity) async -> BrokerCardStatus?
    /// Mint Kotak Neo TOTP session on the agent; vault write happens host-side (no secrets in response).
    func mintKotakSession(
        for identity: BrokerConnectionIdentity,
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) async throws
    /// Mint Kite connect state + login URL (ADR 0005). Browser completes callback on agent.
    func beginZerodhaConnect(
        for identity: BrokerConnectionIdentity,
        apiKey: String,
        apiSecret: String
    ) async throws -> ZerodhaConnectBeginResult
    /// Mint Upstox connect state + authorize URL (ADR 0006). Browser completes callback on agent.
    func beginUpstoxConnect(
        for identity: BrokerConnectionIdentity,
        clientId: String,
        clientSecret: String
    ) async throws -> UpstoxConnectBeginResult
    /// Mint Fyers connect state + authorize URL (ADR 0007). Browser completes callback on agent.
    func beginFyersConnect(
        for identity: BrokerConnectionIdentity,
        appId: String,
        secretId: String
    ) async throws -> FyersConnectBeginResult
    /// Clear host vault entry for identity (Connect rollback / Delete). Identity-only body.
    func clearVaultCredentials(for identity: BrokerConnectionIdentity) async throws
    /// Whether the agent can see a vault blob for this identity (Start presence fallback).
    func vaultCredentialsPresent(for identity: BrokerConnectionIdentity) async -> Bool
    /// Post-Start honesty for Connect — syncState / lastError (kill switch, auth, etc.).
    func fetchSyncHealth(for identity: BrokerConnectionIdentity) async -> BrokerSyncHealthSnapshot?
}

/// Agent `/api/daemon/broker/sync-state` slice used after Connect Start and Brokers load.
public struct BrokerSyncHealthSnapshot: Equatable, Sendable {
    public let syncState: String
    public let runtimeStatus: String
    public let lastError: String?
    /// True only when `/etc/hosts` Kill DNS sinkhole is active (not UBI allowlist `host_blocked`).
    public let killDnsActive: Bool
    public let brokerSlug: String?
    public let lastSuccessAtMs: Int64?
    public let lastPollAtMs: Int64?

    public init(
        syncState: String,
        runtimeStatus: String,
        lastError: String?,
        killDnsActive: Bool = false,
        brokerSlug: String? = nil,
        lastSuccessAtMs: Int64? = nil,
        lastPollAtMs: Int64? = nil
    ) {
        self.syncState = syncState
        self.runtimeStatus = runtimeStatus
        self.lastError = lastError
        self.killDnsActive = killDnsActive
        self.brokerSlug = brokerSlug
        self.lastSuccessAtMs = lastSuccessAtMs
        self.lastPollAtMs = lastPollAtMs
    }

    /// Prefer last successful poll; fall back to last poll attempt.
    public var lastSyncedAtMs: Int64? {
        lastSuccessAtMs ?? lastPollAtMs
    }

    /// Card status from sync-state (prefer `syncState` over agent `runtimeStatus`).
    public var cardStatus: BrokerCardStatus? {
        BrokerAgentSyncStateMapping.cardStatus(syncState: syncState, runtimeStatus: runtimeStatus)
    }

    /// UBI adapter rejected the request host (`error_class=host_blocked`) — often allowlist/baseUrl, not Kill DNS.
    public var isUbiHostBlocked: Bool {
        (lastError ?? "").localizedCaseInsensitiveContains("host_blocked")
    }

    public var isDisconnected: Bool {
        let s = syncState.lowercased()
        return s == "disconnected" || s == "not_connected" || s.isEmpty
    }

    /// Connect may only claim success when the agent reports an active sync class.
    /// Transitional `stale` right after Start is not success.
    public var isLiveEnoughForConnectSuccess: Bool {
        switch syncState.lowercased() {
        case "synced", "syncing":
            return true
        default:
            return false
        }
    }
}

/// Maps agent sync-state JSON → Brokers card status (Notch-aligned).
public enum BrokerAgentSyncStateMapping {
    public static func cardStatus(syncState: String?, runtimeStatus: String) -> BrokerCardStatus? {
        switch (syncState ?? "").lowercased() {
        case "synced":
            return .connected
        case "syncing":
            return .syncing
        case "stale":
            return .degraded
        default:
            break
        }
        switch runtimeStatus.lowercased() {
        case "ready_to_start": return .readyToStart
        case "syncing": return .syncing
        case "degraded": return .degraded
        case "rate_limited": return .rateLimited
        case "paused": return .paused
        default: return nil
        }
    }
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
        await fetchSyncHealth(for: identity)?.cardStatus
    }

    public func mintKotakSession(
        for identity: BrokerConnectionIdentity,
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) async throws {
        let path = "/api/daemon/broker/kotak/session/mint"
        let payload = KotakSessionMintPayload(
            identity: identity,
            consumerKey: consumerKey,
            mobileNumber: mobileNumber,
            ucc: ucc,
            totp: totp,
            mpin: mpin
        )
        let body = try JSONEncoder().encode(payload)
        var request = signRequest("POST", path, body)
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw BrokerAgentRuntimeError.requestFailed
        }
        if http.statusCode == 200 {
            return
        }
        if let decoded = try? JSONDecoder().decode(KotakSessionMintErrorBody.self, from: data) {
            throw BrokerAgentRuntimeError.kotakMintFailed(
                errorClass: decoded.errorClass ?? "upstream",
                message: decoded.message ?? "kotak session mint failed"
            )
        }
        throw BrokerAgentRuntimeError.requestFailed
    }

    public func beginZerodhaConnect(
        for identity: BrokerConnectionIdentity,
        apiKey: String,
        apiSecret: String
    ) async throws -> ZerodhaConnectBeginResult {
        let path = "/api/daemon/broker/zerodha/connect/begin"
        let payload = ZerodhaConnectBeginPayload(
            identity: identity,
            apiKey: apiKey,
            apiSecret: apiSecret
        )
        let body = try JSONEncoder().encode(payload)
        var request = signRequest("POST", path, body)
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw BrokerAgentRuntimeError.requestFailed
        }
        if http.statusCode == 200 {
            guard let decoded = try? JSONDecoder().decode(ZerodhaConnectBeginResponse.self, from: data),
                  decoded.ok == true,
                  let loginURLString = decoded.loginUrl,
                  let loginURL = URL(string: loginURLString),
                  let state = decoded.state,
                  let redirectURI = decoded.redirectUri
            else {
                throw BrokerAgentRuntimeError.requestFailed
            }
            return ZerodhaConnectBeginResult(
                state: state,
                loginURL: loginURL,
                redirectURI: redirectURI
            )
        }
        if let decoded = try? JSONDecoder().decode(ZerodhaConnectBeginErrorBody.self, from: data) {
            throw BrokerAgentRuntimeError.zerodhaBeginFailed(
                errorClass: decoded.errorClass ?? "upstream",
                message: decoded.message ?? "zerodha connect begin failed"
            )
        }
        throw BrokerAgentRuntimeError.requestFailed
    }

    public func beginUpstoxConnect(
        for identity: BrokerConnectionIdentity,
        clientId: String,
        clientSecret: String
    ) async throws -> UpstoxConnectBeginResult {
        let path = "/api/daemon/broker/upstox/begin"
        let payload = UpstoxConnectBeginPayload(
            identity: identity,
            clientId: clientId,
            clientSecret: clientSecret
        )
        let body = try JSONEncoder().encode(payload)
        var request = signRequest("POST", path, body)
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw BrokerAgentRuntimeError.requestFailed
        }
        if http.statusCode == 200 {
            guard let decoded = try? JSONDecoder().decode(UpstoxConnectBeginResponse.self, from: data),
                  decoded.ok == true,
                  let loginURLString = decoded.loginUrl,
                  let loginURL = URL(string: loginURLString),
                  let state = decoded.state,
                  let redirectURI = decoded.redirectUri
            else {
                throw BrokerAgentRuntimeError.requestFailed
            }
            return UpstoxConnectBeginResult(
                state: state,
                loginURL: loginURL,
                redirectURI: redirectURI
            )
        }
        if let decoded = try? JSONDecoder().decode(UpstoxConnectBeginErrorBody.self, from: data) {
            throw BrokerAgentRuntimeError.upstoxBeginFailed(
                errorClass: decoded.errorClass ?? "upstream",
                message: decoded.message ?? "upstox connect begin failed"
            )
        }
        throw BrokerAgentRuntimeError.requestFailed
    }

    public func beginFyersConnect(
        for identity: BrokerConnectionIdentity,
        appId: String,
        secretId: String
    ) async throws -> FyersConnectBeginResult {
        let path = "/api/daemon/broker/fyers/begin"
        let payload = FyersConnectBeginPayload(
            identity: identity,
            appId: appId,
            secretId: secretId
        )
        let body = try JSONEncoder().encode(payload)
        var request = signRequest("POST", path, body)
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw BrokerAgentRuntimeError.requestFailed
        }
        if http.statusCode == 200 {
            guard let decoded = try? JSONDecoder().decode(FyersConnectBeginResponse.self, from: data),
                  decoded.ok == true,
                  let loginURLString = decoded.loginUrl,
                  let loginURL = URL(string: loginURLString),
                  let state = decoded.state,
                  let redirectURI = decoded.redirectUri
            else {
                throw BrokerAgentRuntimeError.requestFailed
            }
            return FyersConnectBeginResult(
                state: state,
                loginURL: loginURL,
                redirectURI: redirectURI
            )
        }
        if let decoded = try? JSONDecoder().decode(FyersConnectBeginErrorBody.self, from: data) {
            throw BrokerAgentRuntimeError.fyersBeginFailed(
                errorClass: decoded.errorClass ?? "upstream",
                message: decoded.message ?? "fyers connect begin failed"
            )
        }
        throw BrokerAgentRuntimeError.requestFailed
    }

    public func clearVaultCredentials(for identity: BrokerConnectionIdentity) async throws {
        let path = "/api/daemon/broker/credentials/clear"
        let payload = BrokerCredentialIdentityPayload(identity: identity)
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

    public func vaultCredentialsPresent(for identity: BrokerConnectionIdentity) async -> Bool {
        let path = "/api/daemon/broker/credentials/present"
        guard let body = try? JSONEncoder().encode(BrokerCredentialIdentityPayload(identity: identity)) else {
            return false
        }
        var request = signRequest("POST", path, body)
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200,
              let decoded = try? JSONDecoder().decode(VaultPresentResponse.self, from: data)
        else {
            return false
        }
        return decoded.present
    }

    public func fetchSyncHealth(for identity: BrokerConnectionIdentity) async -> BrokerSyncHealthSnapshot? {
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
        return BrokerSyncHealthSnapshot(
            syncState: decoded.syncState ?? "",
            runtimeStatus: decoded.runtimeStatus,
            lastError: decoded.lastError,
            killDnsActive: decoded.killDnsActive ?? false,
            brokerSlug: decoded.brokerSlug,
            lastSuccessAtMs: decoded.lastSuccessAtMs,
            lastPollAtMs: decoded.lastPollAtMs
        )
    }
}

public enum BrokerAgentRuntimeError: Error, Equatable {
    case requestFailed
    case kotakMintFailed(errorClass: String, message: String)
    case zerodhaBeginFailed(errorClass: String, message: String)
    case upstoxBeginFailed(errorClass: String, message: String)
    case fyersBeginFailed(errorClass: String, message: String)
}

/// Identity-only Start body (B2). Secrets stay in Keychain; never encoded here.
struct BrokerSyncStartPayload: Encodable {
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

    /// Test/helper: encode identity-only JSON and assert no secret keys.
    static func identityOnlyJSON(for identity: BrokerConnectionIdentity) throws -> Data {
        try JSONEncoder().encode(BrokerSyncStartPayload(identity: identity))
    }
}

struct ZerodhaConnectBeginPayload: Encodable {
    let brokerSlug: String
    let brokerConnectionId: String
    let environment: String
    let apiKey: String
    let apiSecret: String

    init(identity: BrokerConnectionIdentity, apiKey: String, apiSecret: String) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
        self.apiKey = apiKey
        self.apiSecret = apiSecret
    }
}

private struct ZerodhaConnectBeginResponse: Decodable {
    let ok: Bool?
    let state: String?
    let loginUrl: String?
    let redirectUri: String?
}

private struct ZerodhaConnectBeginErrorBody: Decodable {
    let errorClass: String?
    let message: String?

    enum CodingKeys: String, CodingKey {
        case errorClass = "error_class"
        case message
    }
}

struct UpstoxConnectBeginPayload: Encodable {
    let brokerSlug: String
    let brokerConnectionId: String
    let environment: String
    let clientId: String
    let clientSecret: String

    init(identity: BrokerConnectionIdentity, clientId: String, clientSecret: String) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
        self.clientId = clientId
        self.clientSecret = clientSecret
    }
}

private struct UpstoxConnectBeginResponse: Decodable {
    let ok: Bool?
    let state: String?
    let loginUrl: String?
    let redirectUri: String?
}

private struct UpstoxConnectBeginErrorBody: Decodable {
    let errorClass: String?
    let message: String?

    enum CodingKeys: String, CodingKey {
        case errorClass = "error_class"
        case message
    }
}

struct FyersConnectBeginPayload: Encodable {
    let brokerSlug: String
    let brokerConnectionId: String
    let environment: String
    let appId: String
    let secretId: String

    init(identity: BrokerConnectionIdentity, appId: String, secretId: String) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
        self.appId = appId
        self.secretId = secretId
    }
}

private struct FyersConnectBeginResponse: Decodable {
    let ok: Bool?
    let state: String?
    let loginUrl: String?
    let redirectUri: String?
}

private struct FyersConnectBeginErrorBody: Decodable {
    let errorClass: String?
    let message: String?

    enum CodingKeys: String, CodingKey {
        case errorClass = "error_class"
        case message
    }
}

struct KotakSessionMintPayload: Encodable {
    let brokerSlug: String
    let brokerConnectionId: String
    let environment: String
    let consumerKey: String
    let mobileNumber: String
    let ucc: String
    let totp: String
    let mpin: String

    init(
        identity: BrokerConnectionIdentity,
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
        self.consumerKey = consumerKey
        self.mobileNumber = mobileNumber
        self.ucc = ucc
        self.totp = totp
        self.mpin = mpin
    }
}

struct BrokerCredentialIdentityPayload: Encodable {
    let brokerSlug: String
    let brokerConnectionId: String
    let environment: String

    init(identity: BrokerConnectionIdentity) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
    }
}

private struct VaultPresentResponse: Decodable {
    let present: Bool
}

private struct KotakSessionMintErrorBody: Decodable {
    let errorClass: String?
    let message: String?

    enum CodingKeys: String, CodingKey {
        case errorClass = "error_class"
        case message
    }
}

private struct BrokerAgentSyncStateResponse: Decodable {
    let runtimeStatus: String
    let syncState: String?
    let lastError: String?
    let killDnsActive: Bool?
    let brokerSlug: String?
    let lastSuccessAtMs: Int64?
    let lastPollAtMs: Int64?
}

public enum AgentLoopback {
    public static let port: UInt16 = 9137
}

public struct ZerodhaConnectBeginResult: Equatable, Sendable {
    public let state: String
    public let loginURL: URL
    public let redirectURI: String

    public init(state: String, loginURL: URL, redirectURI: String) {
        self.state = state
        self.loginURL = loginURL
        self.redirectURI = redirectURI
    }
}

public struct UpstoxConnectBeginResult: Equatable, Sendable {
    public let state: String
    public let loginURL: URL
    public let redirectURI: String

    public init(state: String, loginURL: URL, redirectURI: String) {
        self.state = state
        self.loginURL = loginURL
        self.redirectURI = redirectURI
    }
}

public struct FyersConnectBeginResult: Equatable, Sendable {
    public let state: String
    public let loginURL: URL
    public let redirectURI: String

    public init(state: String, loginURL: URL, redirectURI: String) {
        self.state = state
        self.loginURL = loginURL
        self.redirectURI = redirectURI
    }
}
