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

    @Test func lastLineOmittedWhenLastIsNil() {
        let scale = SessionChartScale(minPrice: 100, maxPrice: 200, height: 100)
        #expect(SessionChartOverlay.lastLineY(last: nil, scale: scale) == nil)
    }

    @Test func lastLineYMatchesScaleAndStaysInPricePane() {
        let scale = SessionChartScale(minPrice: 100, maxPrice: 200, height: 100)
        let y = SessionChartOverlay.lastLineY(last: 150, scale: scale)
        #expect(y == 50)
        #expect(y! >= 0)
        #expect(y! <= scale.height)
    }

    @Test func slTpPathStartsThirtyPercentFromTheRight() {
        #expect(SessionChartOverlay.dashedStartFraction == 0.7)
        #expect(SessionChartOverlay.dashedLineStartX(plotWidth: 100) == 70)
    }

    @Test func lastTagReservesAxisTicks() {
        let scale = SessionChartScale(minPrice: 100, maxPrice: 200, height: 100)
        let lastY = scale.y(forPrice: 150)
        let ticks = SessionChartOverlay.visibleTicks(
            prices: [100, 125, 150, 175, 200],
            lastY: lastY,
            scale: scale,
            reserve: 12
        )
        #expect(!ticks.contains(150))
        #expect(ticks.contains(100))
        #expect(ticks.contains(200))
    }

    @Test func riskRewardNilUnlessEntrySlTpValidForSide() {
        #expect(
            BarIntradayDeclareValidator.riskRewardRatio(
                entry: 100,
                stop: 90,
                target: 120,
                sideBuy: true
            ) == 2
        )
        #expect(
            BarIntradayDeclareValidator.riskRewardRatio(
                entry: 100,
                stop: 90,
                target: 120,
                sideBuy: false
            ) == nil
        )
        #expect(
            BarIntradayDeclareValidator.riskRewardRatio(
                entry: 100,
                stop: 110,
                target: 80,
                sideBuy: false
            ) == 2
        )
    }

    @Test func magnetSnapsWithinSlopAndLeavesPriceOutside() {
        let scale = SessionChartScale(minPrice: 100, maxPrice: 200, height: 100)
        // close at 150 is y=50. A free price whose y is within 6px of 50 snaps.
        let near = scale.price(atY: 54)
        let snapped = SessionChartMagnet.snap(
            price: near,
            open: 140,
            high: 160,
            low: 130,
            close: 150,
            scale: scale
        )
        #expect(snapped == 150)
        let far = scale.price(atY: 20)
        let free = SessionChartMagnet.snap(
            price: far,
            open: 140,
            high: 160,
            low: 130,
            close: 150,
            scale: scale
        )
        #expect(free == far)
    }

    @Test func magnetCandleIndexUsesPlotWidth() {
        #expect(SessionChartMagnet.candleIndex(x: 5, plotWidth: 100, count: 10) == 0)
        #expect(SessionChartMagnet.candleIndex(x: 50, plotWidth: 100, count: 10) == 5)
        #expect(SessionChartMagnet.candleIndex(x: 99, plotWidth: 100, count: 10) == 9)
        #expect(SessionChartMagnet.candleIndex(x: 100, plotWidth: 100, count: 10) == nil)
        #expect(SessionChartMagnet.candleIndex(x: -1, plotWidth: 100, count: 10) == nil)
    }

    @Test func countdownRemainingFromOneMinuteAndFifteenMin() {
        let open: Int64 = 1_000_000
        let oneMin = SessionChartBarClose.remainingLabel(
            openTimeMs: open,
            interval: "1m",
            nowMs: open + 15_000
        )
        #expect(oneMin == "45s")
        let fifteen = SessionChartBarClose.remainingLabel(
            openTimeMs: open,
            interval: "15min",
            nowMs: open + 60_000
        )
        #expect(fifteen == "14:00")
        #expect(SessionChartBarClose.intervalMs("1m") == 60_000)
        #expect(SessionChartBarClose.intervalMs("15min") == 15 * 60_000)
    }

    @Test func countdownNilWhenIntervalMissing() {
        #expect(
            SessionChartBarClose.remainingLabel(
                openTimeMs: 1,
                interval: nil,
                nowMs: 2
            ) == nil
        )
        #expect(SessionChartBarClose.intervalMs(nil) == nil)
        #expect(SessionChartBarClose.intervalMs("  ") == nil)
    }

    @Test func legendChangeUsesPriorBarCloseNotQuoteLast() {
        let model = SessionChartLegend.model(
            symbol: "BTCUSDT",
            interval: "1m",
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1_200_000,
            prevClose: 100
        )
        #expect(model.headline == "BTCUSDT · 1m")
        #expect(model.ohlcv?.contains("V 1.20M") == true)
        #expect(model.change == "+5.00 (+5.00%)")
        #expect(model.changeUp)
        let vsLast = SessionChartLegend.change(close: 105, prevClose: 81182)
        #expect(vsLast?.text.contains("81182") == false)
    }

    @Test func quoteLastUnknownWithNumberStillBinds() {
        #expect(SessionChartQuoteLast.isBound(status: "unknown"))
        #expect(SessionChartQuoteLast.isBound(status: "fresh"))
        #expect(!SessionChartQuoteLast.isBound(status: "unavailable"))
        #expect(!SessionChartQuoteLast.isBound(status: "quotes_http"))
        #expect(SessionChartQuoteLast.value(81182.10, status: "unknown") == 81182.10)
        #expect(SessionChartQuoteLast.value(81182.10, status: "unavailable") == nil)
        #expect(SessionChartQuoteLast.value(nil, status: "fresh") == nil)
    }

    @Test func fittedScaleIncludesLastSoEntrySitsOnCandle() {
        let scale = SessionChartScale.fitted(prices: [100, 110, 105], height: 80)
        #expect(scale.height == 80)
        #expect(scale.minPrice < 100)
        #expect(scale.maxPrice > 110)
    }
}
