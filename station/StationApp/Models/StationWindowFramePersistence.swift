import AppKit
import Foundation

public struct StationWindowFramePersistence {
    public static let userDefaultsKey = "station.window.frame"

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func persist(frame: CGRect) {
        let data = try? NSKeyedArchiver.archivedData(
            withRootObject: NSValue(rect: frame),
            requiringSecureCoding: true
        )
        if let data {
            defaults.set(data, forKey: Self.userDefaultsKey)
        }
    }

    public func restoreFrame() -> CGRect? {
        guard let data = defaults.data(forKey: Self.userDefaultsKey),
              let value = try? NSKeyedUnarchiver.unarchivedObject(ofClass: NSValue.self, from: data)
        else {
            return nil
        }
        return value.rectValue
    }
}

@MainActor
public final class UserDefaultsLaunchStore: StationLaunchStoring {
    private enum Keys {
        static let firstLaunchCompleted = "station.firstLaunch.completed"
        static let wasWindowVisible = "station.window.wasVisible"
    }

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public var isFirstLaunchCompleted: Bool {
        defaults.bool(forKey: Keys.firstLaunchCompleted)
    }

    public func setFirstLaunchCompleted() {
        defaults.set(true, forKey: Keys.firstLaunchCompleted)
    }

    public var wasWindowVisibleBeforeQuit: Bool {
        defaults.bool(forKey: Keys.wasWindowVisible)
    }

    public func setWasWindowVisibleBeforeQuit(_ visible: Bool) {
        defaults.set(visible, forKey: Keys.wasWindowVisible)
    }

    public var loginAtBoot: Bool {
        ProcessInfo.processInfo.environment["XPC_SERVICE_NAME"]?.contains("loginwindow") == true
            || ProcessInfo.processInfo.arguments.contains("--login-at-boot")
    }
}
