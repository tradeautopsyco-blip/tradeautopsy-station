import Foundation
import Testing
@testable import Notch

/// Prototype oracle: `?asset=options&desk=nfo&kind=opt&board=cockpit&phase=plan`
/// Seed tiles are a public layout spec — not SwiftUI.
struct BarNfoCockpitSeedTests {
    @Test func cockpitSeedMatchesPrototypeBoard() {
        #expect(BarNfoCockpitSeed.planDock == .rail)
        #expect(BarNfoCockpitSeed.tiles == [
            BarNfoCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 2, h: 2),
            BarNfoCockpitSeed.Tile(kind: .oi, x: 2, y: 0, w: 2, h: 2),
            BarNfoCockpitSeed.Tile(kind: .payoff, x: 0, y: 2, w: 2, h: 2),
            BarNfoCockpitSeed.Tile(kind: .depth, x: 2, y: 2, w: 2, h: 2),
            BarNfoCockpitSeed.Tile(kind: .chain, x: 0, y: 4, w: 4, h: 1),
        ])
    }

    @Test func cockpitSeedDoesNotIncludeEditBoardTiles() {
        let kinds = Set(BarNfoCockpitSeed.tiles.map(\.kind))
        #expect(!kinds.contains(.greeks))
        #expect(!kinds.contains(.legs))
        #expect(!kinds.contains(.ladder))
        #expect(!kinds.contains(.ticket))
    }

    @Test func heroSeedMatchesPrototypeBoard() {
        #expect(BarNfoCockpitSeed.planDock(for: .hero) == .rail)
        #expect(BarNfoCockpitSeed.tiles(for: .hero) == [
            BarNfoCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarNfoCockpitSeed.Tile(kind: .oi, x: 0, y: 3, w: 2, h: 1),
            BarNfoCockpitSeed.Tile(kind: .payoff, x: 2, y: 3, w: 2, h: 1),
            BarNfoCockpitSeed.Tile(kind: .chain, x: 0, y: 4, w: 4, h: 1),
        ])
    }

    @Test func focusSeedMatchesPrototypeBoard() {
        #expect(BarNfoCockpitSeed.planDock(for: .focus) == .floor)
        #expect(BarNfoCockpitSeed.tiles(for: .focus) == [
            BarNfoCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarNfoCockpitSeed.Tile(kind: .oi, x: 0, y: 3, w: 1, h: 1),
            BarNfoCockpitSeed.Tile(kind: .payoff, x: 1, y: 3, w: 1, h: 1),
            BarNfoCockpitSeed.Tile(kind: .depth, x: 2, y: 3, w: 1, h: 1),
            BarNfoCockpitSeed.Tile(kind: .chain, x: 3, y: 3, w: 1, h: 1),
        ])
    }

    @Test func nfoCatalogHasNoStrikeGrid() {
        #expect(!BarNfoCockpitSeed.catalog.contains(.ticket))
        let titles = BarNfoCockpitSeed.catalog.map(BarNfoCockpitSeed.title)
        #expect(!titles.contains(where: { $0.lowercased().contains("strike grid") }))
    }
}
