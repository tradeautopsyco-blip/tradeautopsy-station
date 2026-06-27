import Foundation

// MARK: - #121 invalidation hint fade (UserDefaults, per kind)

/// Tracks how many times the user completed the setup step with each invalidation kind selected.
/// After `maxLongFormExposures`, only the short placeholder is shown (unified reference §Pre-trade).
final class BarInvalidationHintUsageStore {
    private let defaults: UserDefaults
    private let keyPrefix = "bar.invalidationHintExposures.v1."

    var maxLongFormExposures: Int = 3

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func exposureCount(for kind: BarInvalidationKind) -> Int {
        let k = keyPrefix + kind.rawValue
        return defaults.integer(forKey: k)
    }

    /// Call when leaving the setup & invalidation step (`Next`) with this kind selected.
    func recordCompletedPass(for kind: BarInvalidationKind) {
        let k = keyPrefix + kind.rawValue
        let n = defaults.integer(forKey: k)
        defaults.set(n + 1, forKey: k)
    }

    func shouldShowLongFormHint(for kind: BarInvalidationKind?) -> Bool {
        guard let kind else { return true }
        return exposureCount(for: kind) < maxLongFormExposures
    }

    /// Test seam — reset one kind.
    func resetExposures(for kind: BarInvalidationKind) {
        defaults.removeObject(forKey: keyPrefix + kind.rawValue)
    }
}
