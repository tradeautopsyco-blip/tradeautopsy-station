import Foundation
import Testing
@testable import Notch

@MainActor
struct BarPretradeBoardStoreTests {
    @Test func seedBoardsAlwaysRehydrateAndCustomPersists() {
        let suite = "ta.pretrade.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = BarPretradeBoardStore(defaults: defaults, storageKey: "test.boards")
        #expect(store.currentBoardId(for: .nfo) == BarCockpitBoardId.cockpit.rawValue)

        store.select(BarCockpitBoardId.hero.rawValue, family: .nfo)
        #expect(store.seedId(for: .nfo) == .hero)

        let custom = BarPretradeCustomBoard(
            id: "custom-ab12",
            name: "Mine",
            planDock: BarCockpitDock.floor.rawValue,
            kinds: ["session", "greeks", "strike"],
            x: [0, 2, 0],
            y: [0, 0, 2],
            w: [2, 2, 4],
            h: [2, 1, 1],
        )
        store.saveCustom(custom, family: .nfo)
        #expect(store.currentBoardId(for: .nfo) == "custom-ab12")

        let tiles = BarNfoOptionsCockpitView.nfoTiles(from: custom)
        #expect(tiles.map(\.kind) == [.session, .greeks])
        #expect(!tiles.contains(where: { $0.kind.rawValue == "strike" }))

        let reloaded = BarPretradeBoardStore(defaults: defaults, storageKey: "test.boards")
        #expect(reloaded.currentBoardId(for: .nfo) == "custom-ab12")
        #expect(reloaded.customBoard(id: "custom-ab12", family: .nfo)?.name == "Mine")

        reloaded.deleteCustom(id: "custom-ab12", family: .nfo)
        #expect(reloaded.currentBoardId(for: .nfo) == BarCockpitBoardId.cockpit.rawValue)

        defaults.removePersistentDomain(forName: suite)
    }

    @Test func lastOnlyCustomKeepsNamedDepthAndDropsChain() {
        let board = BarPretradeCustomBoard(
            id: "custom-d",
            name: "Depth sneak",
            planDock: "rail",
            kinds: ["session", "depth", "chain"],
            x: [0, 0, 0],
            y: [0, 3, 4],
            w: [4, 4, 4],
            h: [3, 2, 1],
        )
        let tiles = BarCashCockpitView.cashTiles(from: board, asset: .usdm)
        #expect(tiles.map(\.kind) == [.session, .depth])
        #expect(!tiles.contains(where: { $0.kind.rawValue == "chain" }))
    }

    @Test func familiesDoNotShareCurrentBoard() {
        let suite = "ta.pretrade.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        let store = BarPretradeBoardStore(defaults: defaults, storageKey: "test.boards.fam")
        store.select(BarCockpitBoardId.focus.rawValue, family: .nfo)
        store.select(BarCockpitBoardId.hero.rawValue, family: .cash)
        #expect(store.seedId(for: .nfo) == .focus)
        #expect(store.seedId(for: .cash) == .hero)
        #expect(store.seedId(for: .lastOnly) == .cockpit)
        defaults.removePersistentDomain(forName: suite)
    }
}
