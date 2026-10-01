import Foundation

/// User-defined hotkey bindings — storage only; Carbon defaults remain until product maps actions.
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

    public static let configurableActions: [(id: String, label: String)] = [
        ("toggle_notch", "Toggle Notch"),
        ("open_station", "Open / front Station"),
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
    }
}
