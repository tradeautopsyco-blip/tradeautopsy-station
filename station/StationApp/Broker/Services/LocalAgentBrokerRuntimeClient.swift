import CryptoKit
import Foundation

@MainActor
public protocol BrokerAgentRuntimeClient {
    func startSync(for identity: BrokerConnectionIdentity, credentials: BrokerCredentials) async throws
    func stopSync(for identity: BrokerConnectionIdentity) async throws
    func fetchRuntimeStatus(for identity: BrokerConnectionIdentity) async -> BrokerCardStatus?
}

/// Loopback wire-v1 client for broker runtime control (#13/#14).
@MainActor
public struct LocalAgentBrokerRuntimeClient: BrokerAgentRuntimeClient {
    public typealias RequestSigner = (
        _ method: String,
        _ path: String,
        _ body: Data
    ) -> URLRequest

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
        let secret = daemonSecret ?? ProcessInfo.processInfo.environment["AGENT_SECRET"] ?? ""
        self.signRequest = AgentWireSigner(secret: secret).signedRequest
    }

    public init(port: UInt16, session: URLSession, signRequest: @escaping RequestSigner) {
        self.port = port
        self.session = session
        self.signRequest = signRequest
    }

    public func startSync(for identity: BrokerConnectionIdentity, credentials: BrokerCredentials) async throws {
        let path = "/api/daemon/broker/sync/start"
        let payload = BrokerSyncStartPayload(identity: identity, credentials: credentials)
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
    let apiKey: String
    let apiSecret: String

    init(identity: BrokerConnectionIdentity, credentials: BrokerCredentials) {
        brokerSlug = identity.brokerSlug
        brokerConnectionId = identity.brokerConnectionID.uuidString
        environment = identity.environment
        assetClass = identity.assetClass
        apiKey = credentials.apiKey
        apiSecret = credentials.apiSecret
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

struct AgentWireSigner {
    let secret: String

    func signedRequest(method: String, path: String, body: Data) -> URLRequest {
        var request = URLRequest(url: URL(string: "http://127.0.0.1:0")!)
        request.httpMethod = method
        let timestamp = AgentWireSigner.wireTimestamp()
        let requestID = AgentWireSigner.makeULID()
        let nonce = AgentWireSigner.makeNonceBase64()
        let signature = makeSignature(
            method: method,
            path: path,
            timestamp: timestamp,
            requestID: requestID,
            body: body
        )

        request.setValue("1", forHTTPHeaderField: "x-proto-version")
        request.setValue(secret, forHTTPHeaderField: "x-daemon-secret")
        request.setValue("00000000-0000-4000-8000-000000000002", forHTTPHeaderField: "x-user-id")
        request.setValue(requestID, forHTTPHeaderField: "x-request-id")
        request.setValue(timestamp, forHTTPHeaderField: "x-timestamp")
        request.setValue(nonce, forHTTPHeaderField: "x-nonce")
        request.setValue(signature, forHTTPHeaderField: "x-signature")
        return request
    }

    private func makeSignature(
        method: String,
        path: String,
        timestamp: String,
        requestID: String,
        body: Data
    ) -> String {
        let bodyHash = SHA256.hash(data: body).map { String(format: "%02x", $0) }.joined()
        let canonical = "\(method.uppercased())\n\(path)\n\(timestamp)\n\(requestID)\n\(bodyHash)"
        let key = SymmetricKey(data: Data(secret.utf8))
        let signature = HMAC<SHA256>.authenticationCode(for: Data(canonical.utf8), using: key)
        return Data(signature).base64EncodedString()
    }

    private static func wireTimestamp() -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter.string(from: Date())
    }

    private static func makeNonceBase64() -> String {
        var bytes = [UInt8](repeating: 0, count: 16)
        _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        return Data(bytes).base64EncodedString()
    }

    private static func makeULID() -> String {
        let ms = UInt64(Date().timeIntervalSince1970 * 1000.0)
        var randomness = [UInt8](repeating: 0, count: 10)
        _ = SecRandomCopyBytes(kSecRandomDefault, randomness.count, &randomness)

        var bytes = [UInt8](repeating: 0, count: 16)
        bytes[0] = UInt8((ms >> 40) & 0xFF)
        bytes[1] = UInt8((ms >> 32) & 0xFF)
        bytes[2] = UInt8((ms >> 24) & 0xFF)
        bytes[3] = UInt8((ms >> 16) & 0xFF)
        bytes[4] = UInt8((ms >> 8) & 0xFF)
        bytes[5] = UInt8(ms & 0xFF)
        for index in 0..<10 { bytes[6 + index] = randomness[index] }
        return encodeULID(bytes)
    }

    private static func encodeULID(_ bytes: [UInt8]) -> String {
        let alphabet = Array("0123456789ABCDEFGHJKMNPQRSTVWXYZ")
        var out: [Character] = []
        out.reserveCapacity(26)
        var value: UInt64 = 0
        var bits = 0
        for byte in bytes {
            value = (value << 8) | UInt64(byte)
            bits += 8
            while bits >= 5 {
                bits -= 5
                let index = Int((value >> UInt64(bits)) & 0x1F)
                out.append(alphabet[index])
            }
        }
        if bits > 0 {
            let index = Int((value << UInt64(5 - bits)) & 0x1F)
            out.append(alphabet[index])
        }
        while out.count < 26 { out.insert("0", at: 0) }
        return String(out.prefix(26))
    }
}

public enum AgentLoopback {
    public static let port: UInt16 = 9137
}
