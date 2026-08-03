import Foundation

/// Maps `/api/daemon/bar/live-state` HTTP failures to Notch-readable copy.
///
/// The agent wraps *every* upstream failure (network unreachable, Console down, missing
/// Station Caller tokens) in the same `502 { error_class: "SERVER_DOWN" }` envelope — so
/// without inspecting `message`, Notch can't tell "device login was never completed" apart
/// from "Console is unreachable". Those need different copy: the former is a one-time,
/// user-actionable setup step in Station; the latter is a transient outage. Conflating them
/// (or conflating either with the *broker* sync pill, a completely separate subsystem) is
/// what produced the confusing "everything is offline" impression.
enum BarLiveStateErrorPresentation {
    private static let deviceLoginMarker = "complete device login"

    static func message(httpStatus: Int, body: Data) -> String {
        if requiresDeviceLogin(body: body) {
            return "Live Plan needs Station sign-in — complete device login in Station Settings."
        }
        return "Live state unreachable (\(httpStatus)) — this is unrelated to your broker connection."
    }

    /// True when the failure is specifically "no Station Caller tokens in Keychain",
    /// i.e. device login was never completed on this machine (not a broker or network issue).
    static func requiresDeviceLogin(body: Data) -> Bool {
        guard let parsed = try? JSONDecoder().decode(ErrorBody.self, from: body),
              let message = parsed.message
        else { return false }
        return message.localizedCaseInsensitiveContains(deviceLoginMarker)
    }

    private struct ErrorBody: Decodable {
        let message: String?
    }
}
