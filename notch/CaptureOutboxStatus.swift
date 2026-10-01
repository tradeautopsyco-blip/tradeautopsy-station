import Foundation

public struct CaptureOutboxCounts: Codable, Equatable {
    public var enqueued: UInt64
    public var inflight: UInt64
    public var acked: UInt64
    public var deadLetter: UInt64

    enum CodingKeys: String, CodingKey {
        case enqueued
        case inflight
        case acked
        case deadLetter = "dead_letter"
    }
}

public struct CaptureOutboxDeadLetter: Codable, Equatable, Identifiable {
    public var outboxId: Int64
    public var requestId: String
    public var attempts: UInt64
    public var reason: String
    public var idempotencyKey: String?
    public var draftText: String?
    public var updatedAtMs: Int64

    public var id: String { "\(outboxId)" }

    enum CodingKeys: String, CodingKey {
        case outboxId
        case requestId
        case attempts
        case reason
        case idempotencyKey
        case draftText
        case updatedAtMs
    }
}

public struct CaptureOutboxActiveDelivery: Codable, Equatable, Identifiable {
    public var state: String
    public var idempotencyKey: String?
    public var lastError: String?
    public var pendingCaptureId: String?

    public var id: String { idempotencyKey ?? state }

    enum CodingKeys: String, CodingKey {
        case state
        case idempotencyKey
        case lastError
        case pendingCaptureId
    }
}

public struct CaptureOutboxStatusSnapshot: Codable, Equatable {
    public var counts: CaptureOutboxCounts
    public var deadLetters: [CaptureOutboxDeadLetter]
    public var activeDeliveries: [CaptureOutboxActiveDelivery]

    enum CodingKeys: String, CodingKey {
        case counts
        case deadLetters = "dead_letters"
        case activeDeliveries = "active_deliveries"
    }
}

public struct CaptureOutboxStatusResponse: Codable {
    public var success: Bool
    public var data: CaptureOutboxStatusSnapshot
}

public enum CaptureOutboxStatusClient {
    public static func decode(from data: Data) -> CaptureOutboxStatusSnapshot? {
        guard let envelope = try? JSONDecoder().decode(CaptureOutboxStatusResponse.self, from: data),
              envelope.success
        else { return nil }
        return envelope.data
    }

    public static func fetch(
        port: UInt16,
        daemonSecret: String,
        session: URLSession = .shared
    ) async -> CaptureOutboxStatusSnapshot? {
        let path = "/api/daemon/journal/toolbar-capture/outbox/status"
        var request = StationWireClient.signedRequest(
            method: "GET",
            path: path,
            body: Data(),
            daemonSecret: daemonSecret
        )
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200
        else { return nil }
        return decode(from: data)
    }

    /// Poll until the idempotency key is ACKed (returns pending capture id) or dead-lettered.
    public static func waitForDelivery(
        idempotencyKey: String,
        port: UInt16,
        daemonSecret: String,
        timeout: TimeInterval = 30,
        pollInterval: TimeInterval = 0.25
    ) async throws -> String {
        let key = idempotencyKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty else {
            throw CaptureOutboxDeliveryError.missingIdempotencyKey
        }
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            guard let snapshot = await fetch(port: port, daemonSecret: daemonSecret) else {
                try await Task.sleep(nanoseconds: UInt64(pollInterval * 1_000_000_000))
                continue
            }
            if let dead = snapshot.deadLetters.first(where: { $0.idempotencyKey == key }) {
                throw CaptureOutboxDeliveryError.deadLetter(reason: dead.reason)
            }
            if let active = snapshot.activeDeliveries.first(where: { $0.idempotencyKey == key }) {
                if active.state == "ACKED", let pid = active.pendingCaptureId, !pid.isEmpty {
                    return pid
                }
            }
            try await Task.sleep(nanoseconds: UInt64(pollInterval * 1_000_000_000))
        }
        throw CaptureOutboxDeliveryError.timedOut
    }
}

public enum CaptureOutboxDeliveryError: LocalizedError {
    case missingIdempotencyKey
    case deadLetter(reason: String)
    case timedOut

    public var errorDescription: String? {
        switch self {
        case .missingIdempotencyKey:
            return "Capture idempotency key missing."
        case .deadLetter(let reason):
            return reason
        case .timedOut:
            return "Capture delivery timed out — check Settings › Journal capture."
        }
    }
}
