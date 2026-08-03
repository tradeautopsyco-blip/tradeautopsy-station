import Foundation

/// Maps hosted declare HTTP failures to Notch-readable copy (#183).
enum BarDeclareHTTPErrorPresentation {
    static func message(httpStatus: Int, body: Data) -> String {
        if let parsed = parseJSON(body) {
            if httpStatus == 403, activationMessage(from: parsed) != nil {
                return activationMessage(from: parsed)!
            }
            if httpStatus == 409, let block = interventionBlockMessage(from: parsed) {
                return block
            }
            if let serverError = parsed["error"] as? String, !serverError.isEmpty {
                let prefix = httpStatus == 409 ? "Declaration blocked" : "Declaration failed"
                return "\(prefix) (\(httpStatus)) — \(serverError)"
            }
            if let errObj = parsed["error"] as? [String: Any],
               let code = errObj["code"] as? String
            {
                return fallbackForCode(code, keys: errObj["keys"] as? [String])
            }
        }
        // Shared seam: allow-listed fields only — never paste raw agent body bytes.
        if let detail = AgentHTTPErrorPresentation.allowListedDetail(from: body) {
            return "Declaration failed (\(httpStatus)) — \(detail)"
        }
        return "Declaration failed (\(httpStatus)) — try again or check web Bar."
    }

    private static func parseJSON(_ body: Data) -> [String: Any]? {
        guard let obj = try? JSONSerialization.jsonObject(with: body) as? [String: Any] else {
            return nil
        }
        return obj
    }

    private static func activationMessage(from parsed: [String: Any]) -> String? {
        guard let errObj = parsed["error"] as? [String: Any],
              let code = errObj["code"] as? String,
              code == "bar_activation_required"
        else { return nil }
        let keys = errObj["keys"] as? [String] ?? []
        if keys.contains("bar.loss_limits.not_acknowledged") {
            return "Acknowledge daily and weekly loss limits in web Bar settings before declaring."
        }
        if keys.contains("bar.loss_limits.missing_daily")
            || keys.contains("bar.loss_limits.missing_weekly")
            || keys.contains("bar.loss_limits.missing_margin_cap")
        {
            return "Set daily, weekly, and margin loss limits in web Bar settings before declaring."
        }
        if !keys.isEmpty {
            return "Complete Bar activation (loss limits) in web Bar settings before declaring."
        }
        return "Complete Bar activation in web Bar settings before declaring."
    }

    private static func interventionBlockMessage(from parsed: [String: Any]) -> String? {
        let kind = (parsed["intervention"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let serverMsg = (parsed["error"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !kind.isEmpty || !serverMsg.isEmpty else { return nil }

        let kindLabel = humanInterventionKind(kind)
        if !serverMsg.isEmpty, !kind.isEmpty {
            return "\(serverMsg) (\(kindLabel))"
        }
        if !serverMsg.isEmpty { return serverMsg }
        return "Declaration blocked — \(kindLabel)"
    }

    private static func humanInterventionKind(_ raw: String) -> String {
        switch raw.lowercased() {
        case "bar_kill_switch", "kill_switch":
            return "kill switch"
        case "bar_composite_red", "composite_red":
            return "composite RED"
        case "bar_stop_me", "stop_me":
            return "stop-me latch"
        case "bar_composite_amber", "soft_block":
            return "soft block"
        case "hard_block":
            return "hard block"
        default:
            return raw.replacingOccurrences(of: "bar_", with: "").replacingOccurrences(of: "_", with: " ")
        }
    }

    private static func fallbackForCode(_ code: String, keys: [String]?) -> String {
        switch code {
        case "bar_activation_required":
            return activationMessage(from: ["error": ["code": code, "keys": keys ?? []]]) ?? code
        case "validation_error":
            return "Fix declaration fields and try again."
        case "persist_failed":
            return "Server could not save declaration — try again."
        default:
            return "Declaration failed — \(code)"
        }
    }
}
