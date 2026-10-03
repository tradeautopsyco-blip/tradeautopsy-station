import CryptoKit
import Foundation

/// Single shared Wire v1 loopback signer for Notch ⇄ StationApp ⇄ agent (T1 — bridge harden).
///
/// **Wire ≠ identity (A8 Phase IV / station-wire freeze):**
/// - Wire v1 HMAC + `x-daemon-secret` authenticate that the caller is *this machine's*
///   Notch/StationApp process talking to the agent it spawned — **machine integrity
///   only**. It proves "same box, same launch", nothing about who the trader is.
/// - `x-user-id` on this loopback hop is the **fixed** `loopbackWireUserId` UUID — a
///   wire hint, never a Console / WorkOS / brain identity. It must never be swapped
///   for a real profile id.
/// - Console who-am-I flows exclusively through the agent's *upstream* hop as
///   `Authorization: Bearer <Station Caller JWT>` (`aud=station`) loaded from Keychain.
///   This type only signs the downward Notch/StationApp → agent hop and must never be
///   reused to build upstream Console requests.
///
/// Canonical string (UTF-8): `{METHOD}\n{path}\n{timestamp_rfc3339}\n{request_id}\n{lowercase_hex_sha256(body)}`
/// `x-signature` = base64(HMAC-SHA256(secret, canonical)) — see `agent/src/wire.rs`.
public enum StationWireClient {
    /// Fixed loopback wire hint UUID (unchanged since A8 IV). NOT a Console identity.
    public static let loopbackWireUserId = "00000000-0000-4000-8000-000000000002"

    /// Wire v1 protocol version — must match `agent/src/wire.rs::WIRE_PROTO_VERSION`.
    public static let protoVersion = "1"

    /// Signature for a loopback request to the given closure.
    public typealias RequestSigner = (
        _ method: String,
        _ path: String,
        _ body: Data
    ) -> URLRequest

    /// Builds a fully wire-v1-signed `URLRequest` for the local agent.
    ///
    /// Sets `x-proto-version`, `x-daemon-secret`, `x-user-id` (fixed wire hint),
    /// `x-request-id`, `x-timestamp`, `x-nonce`, and `x-signature`. The request's
    /// `url` is a placeholder — callers must set `request.url` (and `httpBody` /
    /// `Content-Type` for non-empty bodies) before sending.
    public static func signedRequest(
        method: String,
        path: String,
        body: Data,
        daemonSecret: String
    ) -> URLRequest {
        var request = URLRequest(url: placeholderURL)
        request.httpMethod = method

        let timestamp = wireTimestamp()
        let requestID = makeULID()
        let nonce = makeNonceBase64()
        let sig = signature(
            method: method,
            path: path,
            timestamp: timestamp,
            requestID: requestID,
            body: body,
            secret: daemonSecret
        )

        request.setValue(protoVersion, forHTTPHeaderField: "x-proto-version")
        request.setValue(daemonSecret, forHTTPHeaderField: "x-daemon-secret")
        request.setValue(loopbackWireUserId, forHTTPHeaderField: "x-user-id")
        request.setValue(requestID, forHTTPHeaderField: "x-request-id")
        request.setValue(timestamp, forHTTPHeaderField: "x-timestamp")
        request.setValue(nonce, forHTTPHeaderField: "x-nonce")
        request.setValue(sig, forHTTPHeaderField: "x-signature")
        return request
    }

    /// Returns a `RequestSigner` closure bound to `daemonSecret`, for callers that
    /// inject signing as a dependency (see `LocalAgentBrokerRuntimeClient`).
    public static func requestSigner(daemonSecret: String) -> RequestSigner {
        { method, path, body in
            signedRequest(method: method, path: path, body: body, daemonSecret: daemonSecret)
        }
    }

    /// Wire v1 canonical-string HMAC-SHA256, base64-encoded.
    public static func signature(
        method: String,
        path: String,
        timestamp: String,
        requestID: String,
        body: Data,
        secret: String
    ) -> String {
        let bodyHash = SHA256.hash(data: body).map { String(format: "%02x", $0) }.joined()
        let canonical = "\(method.uppercased())\n\(path)\n\(timestamp)\n\(requestID)\n\(bodyHash)"
        let key = SymmetricKey(data: Data(secret.utf8))
        let mac = HMAC<SHA256>.authenticationCode(for: Data(canonical.utf8), using: key)
        return Data(mac).base64EncodedString()
    }

    /// RFC 3339 UTC timestamp with millisecond precision, matching `agent/src/wire.rs`.
    public static func wireTimestamp() -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter.string(from: Date())
    }

    /// 16 random bytes, base64-encoded, for the `x-nonce` replay-guard header.
    public static func makeNonceBase64() -> String {
        var bytes = [UInt8](repeating: 0, count: 16)
        _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        return Data(bytes).base64EncodedString()
    }

    /// ULID generator for the `x-request-id` wire contract (also usable for
    /// general idempotency keys — it is a plain time-sortable unique id).
    public static func makeULID() -> String {
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
        precondition(bytes.count == 16)
        var out: [Character] = []
        out.reserveCapacity(26)
        var buffer = 0
        var bitCount = 0

        for byte in bytes {
            buffer = (buffer << 8) | Int(byte)
            bitCount += 8
            while bitCount >= 5 {
                let idx = (buffer >> (bitCount - 5)) & 0x1F
                out.append(alphabet[idx])
                bitCount -= 5
            }
        }

        if bitCount > 0 {
            let idx = (buffer << (5 - bitCount)) & 0x1F
            out.append(alphabet[idx])
        }

        if out.count < 26 {
            out = Array(repeating: "0", count: 26 - out.count) + out
        } else if out.count > 26 {
            out = Array(out.suffix(26))
        }

        return String(out)
    }

    private static let placeholderURL = URL(string: "http://127.0.0.1:0")!
}

/// Bare POSIX TCP probe for the loopback agent port.
///
/// A refused `connect()` produces **zero** os_log/stderr output, unlike
/// `URLSession`/`NWConnection`, which dump ~9 CFNetwork lines per refused
/// connection. Every "is anything listening" gate should run this first so a
/// down agent doesn't flood the console with refused-connection noise.
public enum LoopbackTCPProbe {
    /// True when `127.0.0.1:port` accepts a TCP connection within `timeoutMs`.
    public static func accepts(port: UInt16, timeoutMs: Int32 = 300) async -> Bool {
        await Task.detached(priority: .utility) {
            connectable(port: port, timeoutMs: timeoutMs)
        }.value
    }

    /// Synchronous nonblocking-connect variant — equally silent.
    public static func connectable(port: UInt16, timeoutMs: Int32 = 300) -> Bool {
        let fd = socket(AF_INET, SOCK_STREAM, 0)
        guard fd >= 0 else { return false }
        defer { close(fd) }

        let flags = fcntl(fd, F_GETFL)
        _ = fcntl(fd, F_SETFL, flags | O_NONBLOCK)

        var addr = sockaddr_in()
        addr.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = port.bigEndian
        addr.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))

        let result = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                Darwin.connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        if result == 0 { return true }
        guard errno == EINPROGRESS else { return false }

        var pfd = pollfd(fd: fd, events: Int16(POLLOUT), revents: 0)
        guard Darwin.poll(&pfd, 1, timeoutMs) > 0 else { return false }

        var socketError: Int32 = 0
        var len = socklen_t(MemoryLayout<Int32>.size)
        guard getsockopt(fd, SOL_SOCKET, SO_ERROR, &socketError, &len) == 0 else { return false }
        return socketError == 0
    }
}
