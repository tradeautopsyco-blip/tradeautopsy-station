import Foundation

/// User-defined hotkey bindings. Empty storage keeps Carbon ⌥Space / ⌥⇧Space only.
public struct DeskHotkeyBinding: Equatable, Codable {
    public var actionId: String
    public var keyCode: UInt32
    public var carbonModifiers: UInt32

    public init(actionId: String, keyCode: UInt32, carbonModifiers: UInt32) {
        self.actionId = actionId
        self.keyCode = keyCode
        self.carbonModifiers = carbonModifiers
    }
}

public enum DeskHotkeyPreferences {
    public static let storageKey = "tradeautopsy.station.hotkey_bindings"
    public static let didSaveNotification = Notification.Name("tradeautopsy.station.hotkey_bindings.didSave")

    /// Ids are stable. Labels can change. See `docs/design/hotkey-actions.md`.
    public static let configurableActions: [(id: String, label: String)] = [
        ("toggle_notch", "Toggle Notch"),
        ("open_station", "Open / front Station"),
        ("kill", "Kill warning"),
        ("confirm_declare", "Confirm declare"),
        ("plan_another", "Plan another"),
        ("focus_open", "Open"),
        ("focus_plan", "Plan"),
        ("focus_working", "Working"),
        ("focus_debrief", "Debrief"),
        ("cancel_selected_declaration", "Cancel selected declaration"),
        ("capture_working_condition", "Capture working condition"),
        ("protective_sl_chrome", "Protective SL"),
        ("dismiss_kill_overlay", "I'm Calm"),
        ("open_manual_fill", "Manual fill"),
    ]

    public static func load() -> [DeskHotkeyBinding] {
        guard let data = UserDefaults.standard.data(forKey: storageKey),
              let decoded = try? JSONDecoder().decode([DeskHotkeyBinding].self, from: data)
        else { return [] }
        return decoded
    }

    public static func save(_ bindings: [DeskHotkeyBinding]) {
        guard let data = try? JSONEncoder().encode(bindings) else { return }
        UserDefaults.standard.set(data, forKey: storageKey)
        NotificationCenter.default.post(name: didSaveNotification, object: nil)
    }
}
