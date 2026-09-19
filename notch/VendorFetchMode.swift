import Foundation

/// Per-Identity fetch manager. Quote/last is never a vendor mode.
public enum VendorFetchMode: String, Codable, CaseIterable, Sendable, Equatable {
    case off
    case onObtain = "on_obtain"
    case paneAuto = "pane_auto"

    public var label: String {
        switch self {
        case .off: return "Off"
        case .onObtain: return "On obtain"
        case .paneAuto: return "Automated while pane open"
        }
    }
}

public enum VendorFetchModeStore {
    private static func modeKey(_ adapterId: String) -> String {
        "station.vendor.fetchMode.\(adapterId)"
    }

    private static func armedKey(_ adapterId: String) -> String {
        "station.vendor.obtainArmed.\(adapterId)"
    }

    public static func mode(for adapterId: String, defaults: UserDefaults = .standard) -> VendorFetchMode {
        guard let raw = defaults.string(forKey: modeKey(adapterId)),
              let mode = VendorFetchMode(rawValue: raw)
        else {
            return .off
        }
        return mode
    }

    public static func set(_ mode: VendorFetchMode, for adapterId: String, defaults: UserDefaults = .standard) {
        defaults.set(mode.rawValue, forKey: modeKey(adapterId))
    }

    public static func armObtain(_ adapterId: String, defaults: UserDefaults = .standard) {
        defaults.set(true, forKey: armedKey(adapterId))
    }

    public static func consumeArmed(_ adapterId: String, defaults: UserDefaults = .standard) -> Bool {
        let armed = defaults.bool(forKey: armedKey(adapterId))
        if armed {
            defaults.set(false, forKey: armedKey(adapterId))
        }
        return armed
    }

    /// Box Obtain now: Off becomes On-obtain so Notch will fetch the armed query.
    public static func prepareObtainNow(adapterId: String, defaults: UserDefaults = .standard) {
        if mode(for: adapterId, defaults: defaults) == .off {
            set(.onObtain, for: adapterId, defaults: defaults)
        }
        armObtain(adapterId, defaults: defaults)
    }
}

/// Client-driven obtain gate. Agent stays fetch-on-demand.
public enum VendorHistoryObtain {
    public static let kotakHistoryPath = "/api/station/obtain?adapter=kotak_neo&operation=history"

    /// Native cash history. Not gated by `licensed_history` Off.
    public static func kotakNativeHistoryPath(instrument: String) -> String {
        let encoded = InstrumentTickBookId.queryEncode(instrument)
        return "\(kotakHistoryPath)&instrument=\(encoded)"
    }

    public static func kotakHistoryPath(mode: VendorFetchMode, armed: Bool) -> String? {
        switch mode {
        case .off:
            return nil
        case .onObtain:
            return armed ? kotakHistoryPath : nil
        case .paneAuto:
            return kotakHistoryPath
        }
    }
}
