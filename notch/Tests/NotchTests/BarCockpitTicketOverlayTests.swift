import Foundation
import Testing
@testable import Notch

/// Prototype `mosaicHTML` ticket=C overlay: `{ kind: ticket, w:2, h:1 }` via `firstHole`.
/// Layout spec — not SwiftUI.
struct BarCockpitTicketOverlayTests {
    @Test func nfoOccupiedSeedPlacesTwoByOneTicketAtFirstHole() {
        #expect(
            BarCockpitTicketOverlay.firstHole(BarNfoCockpitSeed.occupancies(), w: 2, h: 1)
                == BarCockpitTicketOverlay.Occupancy(x: 0, y: 5, w: 2, h: 1)
        )
        #expect(!BarNfoCockpitSeed.tiles.map(\.kind).contains(.ticket))
    }

    @Test func cashEquityOccupiedPlacesTicketBelowDepth() {
        #expect(
            BarCockpitTicketOverlay.firstHole(BarCashCockpitSeed.occupancies(for: .equity), w: 2, h: 1)
                == BarCockpitTicketOverlay.Occupancy(x: 0, y: 5, w: 2, h: 1)
        )
    }

    @Test func usdmSessionOnlyPlacesTicketBelowSession() {
        #expect(
            BarCockpitTicketOverlay.firstHole(BarCashCockpitSeed.occupancies(for: .usdm), w: 2, h: 1)
                == BarCockpitTicketOverlay.Occupancy(x: 0, y: 3, w: 2, h: 1)
        )
    }

    @Test func cryptoOptionsTicketOverlayMatchesNfoHole() {
        let overlaid = BarNfoCockpitSeed.tilesWithTicketC()
        #expect(overlaid.last == BarNfoCockpitSeed.Tile(kind: .ticket, x: 0, y: 5, w: 2, h: 1))
        #expect(Array(overlaid.dropLast()) == BarNfoCockpitSeed.tiles)
        #expect(!BarNfoCockpitSeed.tiles.map(\.kind).contains(.ticket))
    }
}
