import Combine
import Foundation

/// One active desk shell. Math and DualNoBlend stay identical in every mode.
public enum RiskDeskMode: String, CaseIterable, Sendable, Equatable {
    case blotter = "a"
    case notchFlip = "b"
    case sessionTape = "c"

    public static let `default`: RiskDeskMode = .blotter

    public var title: String {
        switch self {
        case .blotter: return "A · Blotter"
        case .notchFlip: return "B · Notch flip"
        case .sessionTape: return "C · Session tape"
        }
    }

    public var whoIsTheDesk: String {
        switch self {
        case .blotter: return "Today is the desk"
        case .notchFlip: return "Notch is the desk"
        case .sessionTape: return "Time is the desk"
        }
    }
}

/// UserDefaults persistence — not secrets, not Keychain.
@MainActor
public final class RiskDeskModeStore: ObservableObject {
    public static let defaultStorageKey = "tradeautopsy.riskDesk.mode"
    public static let shared = RiskDeskModeStore()

    @Published public private(set) var mode: RiskDeskMode

    private let defaults: UserDefaults
    private let storageKey: String

    public convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKey: Self.defaultStorageKey)
    }

    public init(defaults: UserDefaults, storageKey: String) {
        self.defaults = defaults
        self.storageKey = storageKey
        if let raw = defaults.string(forKey: storageKey),
           let parsed = RiskDeskMode(rawValue: raw)
        {
            mode = parsed
        } else {
            mode = .default
        }
    }

    public func setMode(_ next: RiskDeskMode) {
        guard next != mode else { return }
        mode = next
        defaults.set(next.rawValue, forKey: storageKey)
    }
}
