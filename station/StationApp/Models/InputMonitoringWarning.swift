import Foundation

public struct InputMonitoringWarning: Equatable {
    public static let systemSettingsURL = URL(
        string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
    )!

    public static let restartReminderMessage =
        "Permission granted — please quit and reopen TradeAutopsy Station for it to take effect."

    public var message: String
    public var systemSettingsURL: URL

    public init(
        message: String = "Global ⌥Space requires Input Monitoring permission",
        systemSettingsURL: URL = InputMonitoringWarning.systemSettingsURL
    ) {
        self.message = message
        self.systemSettingsURL = systemSettingsURL
    }
}
