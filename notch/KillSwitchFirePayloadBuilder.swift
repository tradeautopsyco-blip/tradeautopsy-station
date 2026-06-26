import Foundation

enum KillSwitchFirePayloadError: Error, LocalizedError, Equatable {
    case brokerNotConfigured

    var errorDescription: String? {
        switch self {
        case .brokerNotConfigured:
            return "Connect a broker before kill switch."
        }
    }
}

/// L3 fire body — broker from live-state when known; omit for daemon resolution (#192).
enum KillSwitchFirePayloadBuilder {
    static func build(level: Int = 3, protectiveBrokerSlug: String, reason: String = "notch_manual") -> Result<[String: Any], KillSwitchFirePayloadError> {
        let broker = protectiveBrokerSlug.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        var payload: [String: Any] = [
            "level": level,
            "reason": reason,
        ]
        if !broker.isEmpty, broker != "mockbroker" {
            payload["broker"] = broker
        }
        return .success(payload)
    }
}
