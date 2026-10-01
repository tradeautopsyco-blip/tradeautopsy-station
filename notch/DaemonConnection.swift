import Foundation

enum DaemonConnectionState: String {
    case idle
    case connecting
    case connected
    case reconnecting
    case disconnected
}

enum DaemonProtocolErrorClass: String, Decodable {
    case protoVersion = "PROTO_VERSION"
    case sigInvalid = "SIG_INVALID"
    case validation = "VALIDATION"
    case deadLetter = "DEAD_LETTER"
    case rateLimited = "RATE_LIMITED"
    case unknown = "UNKNOWN"
}

struct DaemonErrorEnvelope: Decodable {
    let errorClass: DaemonProtocolErrorClass
    let message: String
    let retryAfterMs: Int?
    let requestId: String?

    enum CodingKeys: String, CodingKey {
        case errorClass = "error_class"
        case message
        case retryAfterMs = "retry_after_ms"
        case requestId = "request_id"
    }
}

/// Small FSM for Phase 1 heartbeat-based connection health.
struct DaemonConnectionFSM {
    var reconnectingThreshold: TimeInterval = 15
    var disconnectedThreshold: TimeInterval = 25
    private(set) var state: DaemonConnectionState = .idle
    private var lastHeartbeatAt: Date?

    mutating func onConnectStart() {
        state = .connecting
    }

    mutating func onStreamOpened(at: Date = Date()) {
        state = .connected
        lastHeartbeatAt = at
    }

    mutating func onHeartbeat(at: Date = Date()) {
        lastHeartbeatAt = at
        state = .connected
    }

    mutating func onTransportFailure() {
        switch state {
        case .idle, .connecting, .connected, .reconnecting:
            state = .reconnecting
        case .disconnected:
            break
        }
    }

    mutating func onTick(now: Date = Date()) -> DaemonConnectionState {
        guard let lastHeartbeatAt else {
            return state
        }
        let silence = now.timeIntervalSince(lastHeartbeatAt)
        if silence >= disconnectedThreshold {
            state = .disconnected
        } else if silence >= reconnectingThreshold, state == .connected {
            state = .reconnecting
        }
        return state
    }
}
