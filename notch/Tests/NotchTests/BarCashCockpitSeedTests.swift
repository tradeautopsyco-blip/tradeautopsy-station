import Foundation
import Testing
@testable import Notch

/// Prototype oracle: `?asset=equity&board=cockpit&phase=plan` (`CASH_SEEDS`)
/// and `?asset=usdm&board=cockpit` (`LAST_SEEDS`). Layout spec — not SwiftUI.
struct BarCashCockpitSeedTests {
    @Test func cashCockpitSeedMatchesPrototypeBoard() {
        #expect(BarCashCockpitSeed.planDock == .rail)
        #expect(BarCashCockpitSeed.tiles(for: .equity) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarCashCockpitSeed.Tile(kind: .depth, x: 0, y: 3, w: 4, h: 2),
        ])
        #expect(BarCashCockpitSeed.tiles(for: .spot) == BarCashCockpitSeed.tiles(for: .equity))
        #expect(BarCashCockpitSeed.showsDepth(for: .equity))
        #expect(BarCashCockpitSeed.showsDepth(for: .spot))
    }

    @Test func lastOnlyCockpitHasSessionAndDepth() {
        #expect(BarCashCockpitSeed.tiles(for: .usdm) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarCashCockpitSeed.Tile(kind: .depth, x: 0, y: 3, w: 4, h: 2),
        ])
        #expect(BarCashCockpitSeed.tiles(for: .coinm) == BarCashCockpitSeed.tiles(for: .usdm))
        #expect(BarCashCockpitSeed.showsDepth(for: .usdm))
        #expect(BarCashCockpitSeed.showsDepth(for: .coinm))
        #expect(BarCashCockpitSeed.tiles(for: .usdm) == BarCashCockpitSeed.tiles(for: .equity))
    }

    @Test func cashGlanceStripIsLastHistoryDepth() {
        #expect(BarCashCockpitSeed.stripKinds(for: .equity) == [.last, .history, .depth])
        #expect(BarCashCockpitSeed.stripKinds(for: .spot) == [.last, .history, .depth])
        #expect(BarCashCockpitSeed.stripKinds(for: .usdm) == [.last, .history, .depth])
        #expect(BarCashCockpitSeed.stripKinds(for: .coinm) == [.last, .history, .depth])
    }

    @Test func leftoverOptionsOnStandardFormStillUsesCashCockpitTiles() {
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTCUSDT"
        ) == .standardForm)
        #expect(BarCashCockpitSeed.tiles(for: .options) == BarCashCockpitSeed.tiles(for: .spot))
        #expect(BarCashCockpitSeed.stripKinds(for: .options) == [.last, .history, .depth])
    }

    @Test func cashHeroAndFocusMatchPrototype() {
        #expect(BarCashCockpitSeed.planDock(for: .hero) == .rail)
        #expect(BarCashCockpitSeed.tiles(for: .equity, board: .hero) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarCashCockpitSeed.Tile(kind: .depth, x: 0, y: 3, w: 2, h: 1),
        ])
        #expect(BarCashCockpitSeed.planDock(for: .focus) == .floor)
        #expect(BarCashCockpitSeed.tiles(for: .spot, board: .focus) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarCashCockpitSeed.Tile(kind: .depth, x: 0, y: 3, w: 4, h: 1),
        ])
    }

    @Test func lastOnlyHeroAndFocusMatchCashSeeds() {
        #expect(BarCashCockpitSeed.tiles(for: .usdm, board: .hero) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarCashCockpitSeed.Tile(kind: .depth, x: 0, y: 3, w: 2, h: 1),
        ])
        #expect(BarCashCockpitSeed.tiles(for: .coinm, board: .focus) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
            BarCashCockpitSeed.Tile(kind: .depth, x: 0, y: 3, w: 4, h: 1),
        ])
        #expect(BarCashCockpitSeed.catalog(for: .usdm) == [.session, .depth])
        #expect(BarCashCockpitSeed.catalog(for: .coinm) == [.session, .depth])
        #expect(BarCashCockpitSeed.catalog(for: .usdm).contains(.depth))
        #expect(!BarCashCockpitSeed.catalog(for: .usdm).contains(.ticket))
    }

    @Test func usdmTicketOverlayIsTwoRowsSoContractsSitBelowTif() {
        let usdm = BarCashCockpitSeed.ticketOverlaySize(for: .usdm)
        #expect(usdm.w == 2)
        #expect(usdm.h == 2)
        #expect(BarCashCockpitSeed.ticketOverlaySize(for: .coinm).h == 2)
        #expect(BarCashCockpitSeed.ticketOverlaySize(for: .spot).h == 1)
        #expect(BarCashCockpitSeed.ticketOverlaySize(for: .equity).h == 1)
        #expect(
            BarCockpitTicketOverlay.firstHole(
                BarCashCockpitSeed.occupancies(for: .usdm),
                w: usdm.w,
                h: usdm.h
            ) == BarCockpitTicketOverlay.Occupancy(x: 0, y: 5, w: 2, h: 2)
        )
    }

    @Test func cashCatalogMayAddDepth() {
        #expect(BarCashCockpitSeed.catalog(for: .equity) == [.session, .depth])
        #expect(BarCashCockpitSeed.catalog(for: .spot) == [.session, .depth])
    }
}
