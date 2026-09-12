import Combine
import Foundation

/// Today hierarchy only. A Day spine · C Split clocks. B is not a ship target.
/// Does not replace `RiskDeskMode` (blotter / Notch-flip / tape).
public enum TodayLayoutMode: String, CaseIterable, Sendable, Equatable {
    case daySpine = "a"
    case splitClocks = "c"

    public static let `default`: TodayLayoutMode = .daySpine

    public var title: String {
        switch self {
        case .daySpine: return "A · Day spine"
        case .splitClocks: return "C · Split clocks"
        }
    }
}

/// UserDefaults persistence — not secrets, not Keychain, not RiskDeskMode.
@MainActor
public final class TodayLayoutModeStore: ObservableObject {
    public static let defaultStorageKey = "tradeautopsy.today.layoutMode"
    public static let shared = TodayLayoutModeStore()

    @Published public private(set) var mode: TodayLayoutMode

    private let defaults: UserDefaults
    private let storageKey: String

    public convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKey: Self.defaultStorageKey)
    }

    public init(defaults: UserDefaults, storageKey: String) {
        self.defaults = defaults
        self.storageKey = storageKey
        if let raw = defaults.string(forKey: storageKey),
           let parsed = TodayLayoutMode(rawValue: raw)
        {
            mode = parsed
        } else {
            mode = .default
        }
    }

    public func setMode(_ next: TodayLayoutMode) {
        guard next != mode else { return }
        mode = next
        defaults.set(next.rawValue, forKey: storageKey)
    }
}
