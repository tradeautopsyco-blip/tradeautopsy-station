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

    @Test func cashEquityOccupiedPlacesTicketBesideDepth() {
        let size = BarCashCockpitSeed.ticketOverlaySize(for: .equity)
        #expect(
            BarCockpitTicketOverlay.firstHole(
                BarCashCockpitSeed.occupancies(for: .equity),
                w: size.w,
                h: size.h,
            )
                == BarCockpitTicketOverlay.Occupancy(x: 2, y: 3, w: 2, h: 2)
        )
    }

    @Test func usdmAndCoinmPlaceTicketBesideDepthLikeCash() {
        let size = BarCashCockpitSeed.ticketOverlaySize(for: .usdm)
        #expect(
            BarCockpitTicketOverlay.firstHole(
                BarCashCockpitSeed.occupancies(for: .usdm),
                w: size.w,
                h: size.h,
            )
                == BarCockpitTicketOverlay.Occupancy(x: 2, y: 3, w: 2, h: 2)
        )
        #expect(
            BarCockpitTicketOverlay.firstHole(
                BarCashCockpitSeed.occupancies(for: .coinm),
                w: size.w,
                h: size.h,
            )
                == BarCockpitTicketOverlay.firstHole(
                    BarCashCockpitSeed.occupancies(for: .equity),
                    w: size.w,
                    h: size.h,
                )
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

    @Test func cashCockpitWithTicketUsesFiveMosaicRows() {
        let rows = max(
            1,
            BarCashCockpitSeed.tiles(for: .spot).map { $0.y + $0.h }.max() ?? 1,
        )
        let withTicket = BarCockpitTicketOverlay.firstHole(
            BarCashCockpitSeed.occupancies(for: .spot),
            w: 2,
            h: 2,
        )
        let totalRows = max(rows, withTicket.y + withTicket.h)
        #expect(totalRows == 5)
        let row = BarCockpitMosaicMetrics.rowHeight(in: 400, rows: totalRows, spacing: 8)
        #expect(row > 72)
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
