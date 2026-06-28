import Foundation

@MainActor
public final class UserDefaultsDeskRouteStore: DeskRouteStoring {
    public static let userDefaultsKey = "station.route.desk"

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public var savedDeskRoute: StationRoute? {
        guard let raw = defaults.string(forKey: Self.userDefaultsKey) else { return nil }
        return StationRoute(rawValue: raw)
    }

    public func saveDeskRoute(_ route: StationRoute) {
        defaults.set(route.rawValue, forKey: Self.userDefaultsKey)
    }
}
