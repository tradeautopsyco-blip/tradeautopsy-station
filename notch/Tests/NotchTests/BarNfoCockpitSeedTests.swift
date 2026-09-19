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
}
