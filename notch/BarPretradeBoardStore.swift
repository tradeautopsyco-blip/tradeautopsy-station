import Combine
import Foundation

/// Per-family Pre-trade board: seed ids always rehydrate from code.
/// Custom boards persist extras only. Analog of prototype `ta.notch.pretrade.boards`.
enum BarPretradeBoardFamily: String, CaseIterable, Equatable, Hashable {
    case nfo
    case cash
    case lastOnly

    static func family(for asset: BarDeclareAssetClass) -> BarPretradeBoardFamily {
        switch asset {
        case .usdm, .coinm: return .lastOnly
        case .spot, .equity, .options: return .cash
        }
    }
}

struct BarPretradeCustomBoard: Equatable, Codable {
    var id: String
    var name: String
    var planDock: String
    var kinds: [String]
    /// Parallel to `kinds` — drop the tile if counts disagree.
    var x: [Int]
    var y: [Int]
    var w: [Int]
    var h: [Int]
}

@MainActor
final class BarPretradeBoardStore: ObservableObject {
    static let defaultStorageKey = "tradeautopsy.pretrade.boards"
    static let shared = BarPretradeBoardStore()

    @Published private(set) var currentId: [BarPretradeBoardFamily: String]
    @Published private(set) var custom: [BarPretradeBoardFamily: [BarPretradeCustomBoard]]
    @Published var editing = false

    private let defaults: UserDefaults
    private let storageKey: String

    convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKey: Self.defaultStorageKey)
    }

    init(defaults: UserDefaults, storageKey: String) {
        self.defaults = defaults
        self.storageKey = storageKey
        var current: [BarPretradeBoardFamily: String] = [:]
        var custom: [BarPretradeBoardFamily: [BarPretradeCustomBoard]] = [:]
        for family in BarPretradeBoardFamily.allCases {
            current[family] = BarCockpitBoardId.cockpit.rawValue
            custom[family] = []
        }
        if let data = defaults.data(forKey: storageKey),
           let parsed = try? JSONDecoder().decode(Persisted.self, from: data)
        {
            for family in BarPretradeBoardFamily.allCases {
                if let id = parsed.current[family.rawValue], !id.isEmpty {
                    current[family] = id
                }
                custom[family] = parsed.custom[family.rawValue] ?? []
            }
        }
        self.currentId = current
        self.custom = custom
        // Seed ids always rehydrate — a stale cockpit mutation is dropped.
        for family in BarPretradeBoardFamily.allCases {
            if let id = current[family], BarCockpitBoardId(rawValue: id) != nil {
                continue
            }
            if let id = current[family], (custom[family] ?? []).contains(where: { $0.id == id }) {
                continue
            }
            self.currentId[family] = BarCockpitBoardId.cockpit.rawValue
        }
    }

    func currentBoardId(for family: BarPretradeBoardFamily) -> String {
        currentId[family] ?? BarCockpitBoardId.cockpit.rawValue
    }

    func seedId(for family: BarPretradeBoardFamily) -> BarCockpitBoardId? {
        BarCockpitBoardId(rawValue: currentBoardId(for: family))
    }

    func select(_ id: String, family: BarPretradeBoardFamily) {
        currentId[family] = id
        persist()
    }

    func customBoard(id: String, family: BarPretradeBoardFamily) -> BarPretradeCustomBoard? {
        custom[family]?.first(where: { $0.id == id })
    }

    func saveCustom(_ board: BarPretradeCustomBoard, family: BarPretradeBoardFamily) {
        var list = custom[family] ?? []
        if let idx = list.firstIndex(where: { $0.id == board.id }) {
            list[idx] = board
        } else {
            list.append(board)
        }
        custom[family] = list
        currentId[family] = board.id
        persist()
    }

    func deleteCustom(id: String, family: BarPretradeBoardFamily) {
        guard BarCockpitBoardId(rawValue: id) == nil else { return }
        custom[family] = (custom[family] ?? []).filter { $0.id != id }
        if currentId[family] == id {
            currentId[family] = BarCockpitBoardId.cockpit.rawValue
        }
        persist()
    }

    func resetToCockpit(family: BarPretradeBoardFamily) {
        currentId[family] = BarCockpitBoardId.cockpit.rawValue
        persist()
    }

    private func persist() {
        var current: [String: String] = [:]
        var customMap: [String: [BarPretradeCustomBoard]] = [:]
        for family in BarPretradeBoardFamily.allCases {
            current[family.rawValue] = currentId[family] ?? BarCockpitBoardId.cockpit.rawValue
            customMap[family.rawValue] = custom[family] ?? []
        }
        let blob = Persisted(v: 1, current: current, custom: customMap)
        if let data = try? JSONEncoder().encode(blob) {
            defaults.set(data, forKey: storageKey)
        }
    }

    private struct Persisted: Codable {
        var v: Int
        var current: [String: String]
        var custom: [String: [BarPretradeCustomBoard]]
    }
}
