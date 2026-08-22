import Foundation

enum NotchChipSystemSlot: String, CaseIterable, Equatable {
    case limitLeft = "limit_left"
    case trades
    case clock
    case stop
    case sync

    var title: String {
        switch self {
        case .limitLeft: return "Limit left"
        case .trades: return "Session trades"
        case .clock: return "Session clock"
        case .stop: return "Stop"
        case .sync: return "Broker sync"
        }
    }

    var meta: String {
        switch self {
        case .limitLeft: return "Daily loss limit remaining"
        case .trades: return "Count / max"
        case .clock: return "Time left in window"
        case .stop: return "Missing / on"
        case .sync: return "Book age"
        }
    }

    static func contains(_ id: String) -> Bool {
        Self(rawValue: id) != nil
    }
}

struct NotchChipCatalogState: Equatable, Codable {
    static let maxExtras = 4
    static let impactId = "impact"

    /// Extra row ids that are on. Impact is always on and never stored here.
    var extras: [String]
    /// Symbols seen on the book — catalog names, not a free-text box.
    var seenSymbols: [String]

    init(extras: [String] = [], seenSymbols: [String] = []) {
        self.extras = extras
        self.seenSymbols = seenSymbols
    }

    /// Name extras only. Empty → `AccountImpact` uses all open positions.
    var pinnedNameSymbols: [String] {
        extras.filter { !NotchChipSystemSlot.contains($0) }
    }

    var canAddExtra: Bool { extras.count < Self.maxExtras }

    func isOn(_ id: String) -> Bool {
        if id == Self.impactId { return true }
        return extras.contains(id)
    }
}

protocol NotchChipCatalogStoring: AnyObject {
    func load() -> NotchChipCatalogState
    func save(_ state: NotchChipCatalogState)
}

final class UserDefaultsNotchChipCatalogStore: NotchChipCatalogStoring {
    static let defaultStorageKey = "tradeautopsy.notch.chip.catalog"

    private let defaults: UserDefaults
    private let storageKey: String

    convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKey: Self.defaultStorageKey)
    }

    init(defaults: UserDefaults, storageKey: String) {
        self.defaults = defaults
        self.storageKey = storageKey
    }

    func load() -> NotchChipCatalogState {
        guard let data = defaults.data(forKey: storageKey) else {
            return NotchChipCatalogState()
        }
        return (try? JSONDecoder().decode(NotchChipCatalogState.self, from: data)) ?? NotchChipCatalogState()
    }

    func save(_ state: NotchChipCatalogState) {
        guard let data = try? JSONEncoder().encode(state) else { return }
        defaults.set(data, forKey: storageKey)
    }
}

enum NotchChipCatalogMutations {
    static func toggleExtra(_ id: String, in state: NotchChipCatalogState) -> NotchChipCatalogState {
        let trimmed = id.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed != NotchChipCatalogState.impactId else { return state }
        var next = state
        if let idx = next.extras.firstIndex(of: trimmed) {
            next.extras.remove(at: idx)
            return next
        }
        guard next.canAddExtra else { return state }
        next.extras.append(trimmed)
        return next
    }

    static func rememberSymbols(_ symbols: [String], in state: NotchChipCatalogState) -> NotchChipCatalogState {
        var next = state
        var seen = Set(next.seenSymbols.map { $0.uppercased() })
        for raw in symbols {
            let sym = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !sym.isEmpty, !NotchChipSystemSlot.contains(sym) else { continue }
            let key = sym.uppercased()
            if seen.contains(key) { continue }
            seen.insert(key)
            next.seenSymbols.append(sym)
        }
        return next
    }
}
