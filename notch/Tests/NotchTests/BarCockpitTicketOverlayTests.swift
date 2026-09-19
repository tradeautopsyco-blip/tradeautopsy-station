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

    @Test func usdmAndCoinmPlaceTicketBelowDepthLikeCash() {
        #expect(
            BarCockpitTicketOverlay.firstHole(BarCashCockpitSeed.occupancies(for: .usdm), w: 2, h: 1)
                == BarCockpitTicketOverlay.Occupancy(x: 0, y: 5, w: 2, h: 1)
        )
        #expect(
            BarCockpitTicketOverlay.firstHole(BarCashCockpitSeed.occupancies(for: .coinm), w: 2, h: 1)
                == BarCockpitTicketOverlay.firstHole(BarCashCockpitSeed.occupancies(for: .equity), w: 2, h: 1)
        )
    }

    @Test func cryptoOptionsTicketOverlayMatchesNfoHole() {
        let overlaid = BarNfoCockpitSeed.tilesWithTicketC()
        #expect(overlaid.last == BarNfoCockpitSeed.Tile(kind: .ticket, x: 0, y: 5, w: 2, h: 1))
        #expect(Array(overlaid.dropLast()) == BarNfoCockpitSeed.tiles)
        #expect(!BarNfoCockpitSeed.tiles.map(\.kind).contains(.ticket))
    }
}

struct BarCockpitMosaicMetricsTests {
    @Test func fittedSizeHonorsProposedHeightSoTicketStaysOnScreen() {
        let fitted = BarCockpitMosaicMetrics.fittedSize(
            proposed: CGSize(width: 800, height: 400),
            rows: 6,
            spacing: 8,
            minRowHeight: 96
        )
        #expect(fitted.width == 800)
        #expect(fitted.height == 400)
        let row = BarCockpitMosaicMetrics.rowHeight(
            in: 400,
            rows: 6,
            spacing: 8
        )
        #expect(abs(row - 60) < 0.001)
    }

    @Test func unconstrainedBoardKeepsMinimumRowHeight() {
        let fitted = BarCockpitMosaicMetrics.fittedSize(
            proposed: CGSize(width: 800, height: -1),
            rows: 6,
            spacing: 8,
            minRowHeight: 96
        )
        #expect(fitted.height == 616)
    }
}
