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

    @Test func lastOnlyCockpitHasNoDepthTile() {
        #expect(BarCashCockpitSeed.tiles(for: .usdm) == [
            BarCashCockpitSeed.Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
        ])
        #expect(BarCashCockpitSeed.tiles(for: .coinm) == BarCashCockpitSeed.tiles(for: .usdm))
        #expect(!BarCashCockpitSeed.showsDepth(for: .usdm))
        #expect(!BarCashCockpitSeed.showsDepth(for: .coinm))
    }

    @Test func cashGlanceStripIsLastHistoryDepth() {
        #expect(BarCashCockpitSeed.stripKinds(for: .equity) == [.last, .history, .depth])
        #expect(BarCashCockpitSeed.stripKinds(for: .spot) == [.last, .history, .depth])
    }

    @Test func lastOnlyGlanceStripShowsHistoryHoleAndMargin() {
        #expect(BarCashCockpitSeed.stripKinds(for: .usdm) == [.last, .history, .margin])
        #expect(BarCashCockpitSeed.stripKinds(for: .coinm) == [.last, .history, .margin])
    }

    @Test func leftoverOptionsOnStandardFormStillUsesCashCockpitTiles() {
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTCUSDT"
        ) == .standardForm)
        #expect(BarCashCockpitSeed.tiles(for: .options) == BarCashCockpitSeed.tiles(for: .spot))
        #expect(BarCashCockpitSeed.stripKinds(for: .options) == [.last, .history, .depth])
    }
}
