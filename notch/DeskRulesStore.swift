import Combine
import Foundation

/// Station desk rules on this Mac — floor / mean loss / max round trips / hide Notch.
/// UserDefaults persistence — not secrets, not Keychain, not Console `daily_loss_limit`.
/// Empty keys stay `nil` / false. Do not invent rupees.
@MainActor
public final class DeskRulesStore: ObservableObject {
    public static let defaultStorageKeyPrefix = "tradeautopsy.deskRules"
    public static let shared = DeskRulesStore()

    public static let dailyFloorKeySuffix = "dailyFloor"
    public static let meanLossKeySuffix = "meanLoss"
    public static let maxRoundTripsKeySuffix = "maxRoundTrips"
    public static let hideNotchKeySuffix = "hideNotch"

    @Published public private(set) var dailyFloor: Double?
    @Published public private(set) var meanLoss: Double?
    @Published public private(set) var maxRoundTrips: Int?
    @Published public private(set) var hideNotch: Bool

    private let defaults: UserDefaults
    private let storageKeyPrefix: String

    public convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKeyPrefix: Self.defaultStorageKeyPrefix)
    }

    public init(defaults: UserDefaults, storageKeyPrefix: String) {
        self.defaults = defaults
        self.storageKeyPrefix = storageKeyPrefix
        dailyFloor = Self.readDouble(defaults, key: Self.key(prefix: storageKeyPrefix, suffix: Self.dailyFloorKeySuffix))
        meanLoss = Self.readDouble(defaults, key: Self.key(prefix: storageKeyPrefix, suffix: Self.meanLossKeySuffix))
        maxRoundTrips = Self.readInt(defaults, key: Self.key(prefix: storageKeyPrefix, suffix: Self.maxRoundTripsKeySuffix))
        hideNotch = defaults.bool(forKey: Self.key(prefix: storageKeyPrefix, suffix: Self.hideNotchKeySuffix))
    }

    public func setDailyFloor(_ next: Double?) {
        dailyFloor = next
        writeOptional(next, suffix: Self.dailyFloorKeySuffix)
    }

    public func setMeanLoss(_ next: Double?) {
        meanLoss = next
        writeOptional(next, suffix: Self.meanLossKeySuffix)
    }

    public func setMaxRoundTrips(_ next: Int?) {
        maxRoundTrips = next
        let storageKey = key(Self.maxRoundTripsKeySuffix)
        if let next {
            defaults.set(next, forKey: storageKey)
        } else {
            defaults.removeObject(forKey: storageKey)
        }
    }

    public func setHideNotch(_ next: Bool) {
        hideNotch = next
        defaults.set(next, forKey: key(Self.hideNotchKeySuffix))
    }

    public static func key(prefix: String, suffix: String) -> String {
        "\(prefix).\(suffix)"
    }

    private func key(_ suffix: String) -> String {
        Self.key(prefix: storageKeyPrefix, suffix: suffix)
    }

    private func writeOptional(_ value: Double?, suffix: String) {
        let storageKey = key(suffix)
        if let value {
            defaults.set(value, forKey: storageKey)
        } else {
            defaults.removeObject(forKey: storageKey)
        }
    }

    private static func readDouble(_ defaults: UserDefaults, key: String) -> Double? {
        guard defaults.object(forKey: key) != nil else { return nil }
        return defaults.double(forKey: key)
    }

    private static func readInt(_ defaults: UserDefaults, key: String) -> Int? {
        guard defaults.object(forKey: key) != nil else { return nil }
        return defaults.integer(forKey: key)
    }
}
