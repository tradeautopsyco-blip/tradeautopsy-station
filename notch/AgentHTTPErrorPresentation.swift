import Foundation

/// Maps agent HTTP failure bodies to Notch-readable copy.
///
/// Only allow-listed JSON fields become user-facing text — never raw response
/// body bytes (which can contain secrets, tokens, or upstream dumps).
enum AgentHTTPErrorPresentation {
    static func message(httpStatus: Int, body: Data) -> String {
        guard let detail = allowListedDetail(from: body) else {
            return "Request failed (\(httpStatus)) — try again"
        }
        return "Request failed (\(httpStatus)) — \(detail)"
    }

    /// Allow-listed JSON fields only (`error_class`, `message`, `error`, `code`).
    static func allowListedDetail(from body: Data) -> String? {
        guard let parsed = try? JSONSerialization.jsonObject(with: body) as? [String: Any] else {
            return nil
        }
        let errorClass = trimmedString(parsed["error_class"])
        let message = trimmedString(parsed["message"])
        if !errorClass.isEmpty, !message.isEmpty {
            return "\(errorClass): \(message)"
        }
        if !message.isEmpty { return message }
        if !errorClass.isEmpty { return errorClass }

        if let errorString = parsed["error"] as? String {
            let t = errorString.trimmingCharacters(in: .whitespacesAndNewlines)
            if !t.isEmpty { return t }
        } else if let errObj = parsed["error"] as? [String: Any] {
            let nestedMessage = trimmedString(errObj["message"])
            if !nestedMessage.isEmpty { return nestedMessage }
            let nestedCode = trimmedString(errObj["code"])
            if !nestedCode.isEmpty { return nestedCode }
        }

        let code = trimmedString(parsed["code"])
        if !code.isEmpty { return code }
        return nil
    }

    private static func trimmedString(_ value: Any?) -> String {
        (value as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }
}
