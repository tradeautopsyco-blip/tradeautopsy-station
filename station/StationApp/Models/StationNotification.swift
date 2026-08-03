import Foundation

/// The single onboarding-related notification to show, if any. Priority-ordered:
/// input monitoring warning first, then its restart reminder, then the login-item
/// prompt — these are inherently sequential, so showing at most one never hides
/// something the user still needs to see. Agent health has its own permanent home
/// in the toolbar and is not part of this type.
public enum StationNotification: Equatable {
    case inputMonitoringWarning(InputMonitoringWarning)
    case inputMonitoringRestartReminder(String)
    case loginItemPrompt
}

public extension StationNotification {
    static func current(
        inputMonitoringWarning: InputMonitoringWarning?,
        inputMonitoringRestartReminder: String?,
        showLoginItemPrompt: Bool
    ) -> StationNotification? {
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
