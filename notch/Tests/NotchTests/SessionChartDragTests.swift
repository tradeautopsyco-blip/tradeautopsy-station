import Foundation
import Testing
@testable import Notch

struct SessionChartDragTests {
    @Test func cashStylePriceFormatsToTwoDecimalPlaces() {
        #expect(SessionChartPriceFormat.string(from: 24245.5) == "24245.50")
    }

    @Test func usdtPremiumKeepsFourDecimalPlaces() {
        #expect(SessionChartPriceFormat.string(from: 0.0012) == "0.0012")
    }

    @Test func usdtPremiumDoesNotRoundThousandthToZero() {
        #expect(SessionChartPriceFormat.string(from: 0.001) == "0.001")
    }

    @Test func hitTestMatchesPrototypeSlTpEntryOrder() {
        let hit = SessionChartHit(entryY: 40, slY: 80, tpY: 12, slop: 8)
        #expect(hit.line(atY: 40) == .entry)
        #expect(hit.line(atY: 80) == .sl)
        #expect(hit.line(atY: 12) == .tp)
        #expect(hit.line(atY: 50) == nil)
    }

    @Test func overlappingHitPrefersSlThenTpThenEntry() {
        let hit = SessionChartHit(entryY: 40, slY: 40, tpY: 40, slop: 8)
        #expect(hit.line(atY: 40) == .sl)
    }

    @Test func priceMappingMidplotIsOneFifty() {
        let scale = SessionChartScale(minPrice: 100, maxPrice: 200, height: 100)
        #expect(scale.price(atY: 50) == 150)
        #expect(scale.y(forPrice: 150) == 50)
    }

    @Test func priceMappingClampsToRange() {
        let scale = SessionChartScale(minPrice: 100, maxPrice: 200, height: 100)
        #expect(scale.price(atY: -10) == 200)
        #expect(scale.price(atY: 200) == 100)
    }
}
