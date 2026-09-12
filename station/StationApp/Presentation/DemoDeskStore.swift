import Combine
import Foundation

/// Opt-in Station Demo desk. Sibling of `DeskRulesStore` so money keys stay uncontaminated.
/// Default off. Never POSTs loss-limits, never fires Kill, never touches Console.
@MainActor
public final class DemoDeskStore: ObservableObject {
    public static let defaultStorageKey = "tradeautopsy.demoDesk.enabled"
    public static let shared = DemoDeskStore()

    @Published public private(set) var demoEnabled: Bool

    private let defaults: UserDefaults
    private let storageKey: String

    public convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKey: Self.defaultStorageKey)
    }

    public init(defaults: UserDefaults, storageKey: String) {
        self.defaults = defaults
        self.storageKey = storageKey
        demoEnabled = defaults.bool(forKey: storageKey)
    }

    public func setEnabled(_ next: Bool) {
        guard next != demoEnabled else { return }
        demoEnabled = next
        defaults.set(next, forKey: storageKey)
    }
}
