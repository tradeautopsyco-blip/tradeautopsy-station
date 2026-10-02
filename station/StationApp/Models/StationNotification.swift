import Foundation

/// The single onboarding-related notification to show, if any. Priority-ordered:
/// device login first — a burned Station session disables the product outright —
/// then input monitoring warning, its restart reminder, then the login-item
/// prompt — those are inherently sequential, so showing at most one never hides
/// something the user still needs to see. Agent health has its own permanent home
/// in the toolbar and is not part of this type.
public enum StationNotification: Equatable {
    case deviceLoginRequired
    case inputMonitoringWarning(InputMonitoringWarning)
    case inputMonitoringRestartReminder(String)
    case loginItemPrompt
}

public extension StationNotification {
    static func current(
        stationLoginRequired: Bool,
        inputMonitoringWarning: InputMonitoringWarning?,
        inputMonitoringRestartReminder: String?,
        showLoginItemPrompt: Bool
    ) -> StationNotification? {
        if stationLoginRequired {
            return .deviceLoginRequired
        }
        if let inputMonitoringWarning {
            return .inputMonitoringWarning(inputMonitoringWarning)
        }
        if let inputMonitoringRestartReminder {
            return .inputMonitoringRestartReminder(inputMonitoringRestartReminder)
        }
        if showLoginItemPrompt {
            return .loginItemPrompt
        }
        return nil
    }
}
