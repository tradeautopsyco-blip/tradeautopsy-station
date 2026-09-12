import Foundation
import Testing
@testable import Station

struct TodayClosedFloorChartLayoutTests {
    @Test func kotakINREmptyPointsShowsNseSessionTicksWithoutInventingSeries() {
        let layout = layout(
            points: [],
            quoteCurrency: "INR",
            brokerSlug: "kotak_neo"
        )
        #expect(layout.showsNseSessionTicks == true)
        #expect(layout.tickLabels == ["09:15", "10:15", "11:15", "12:15", "13:15", "14:15", "15:30"])
        #expect(layout.plotPoints.isEmpty)
        #expect(layout.lastPillText == nil)
    }

    @Test func binanceUsdPointDoesNotUseNseSessionTicks() throws {
        let layout = layout(
            points: [
                TodayClosedChartPoint(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    cumulativeClosedPnL: -100
                ),
            ],
            quoteCurrency: "USD",
            brokerSlug: "binance_com"
        )
        #expect(layout.showsNseSessionTicks == false)
        #expect(layout.tickLabels.isEmpty)
        #expect(layout.tickXs.isEmpty)
        try #require(layout.plotPoints.count == 1)
        #expect(layout.plotPoints[0].x == 0.5)
        #expect(abs(layout.plotPoints[0].x - (26.0 / 375.0)) > 0.01)
    }

    @Test func missingQuoteDoesNotShowNseTicks() {
        let layout = layout(
            points: [
                TodayClosedChartPoint(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    cumulativeClosedPnL: -5_800
                ),
            ],
            quoteCurrency: nil,
            brokerSlug: "kotak_neo"
        )
        #expect(layout.showsNseSessionTicks == false)
        #expect(layout.tickLabels.isEmpty)
        #expect(layout.tickXs.isEmpty)
    }

    @Test func kotakClosedAtMapsXFromIstSessionNotEqualIndex() throws {
        let layout = kotakTwoPointLayout()
        try #require(layout.plotPoints.count == 2)
        #expect(abs(layout.plotPoints[0].x - (26.0 / 375.0)) < 0.0001)
        #expect(layout.plotPoints[0].x != 0.5)
        #expect(layout.plotPoints[1].x != 0.5)
    }

    @Test func kotakTwoClosedAtsFollowSessionClockNotEqualSpacing() throws {
        let layout = kotakTwoPointLayout()
        try #require(layout.plotPoints.count == 2)
        #expect(abs(layout.plotPoints[0].x - (26.0 / 375.0)) < 0.0001)
        #expect(abs(layout.plotPoints[1].x - (120.0 / 375.0)) < 0.0001)
        #expect(layout.plotPoints[0].x != 0)
        #expect(layout.plotPoints[1].x != 1)
    }

    @Test func lastCumulativeLossFormatsCompactPillAndLossFill() throws {
        let layout = kotakTwoPointLayout()
        try #require(layout.lastPillText != nil)
        let pill = layout.lastPillText!
        #expect(pill.contains("5.8"))
        #expect(pill.lowercased().contains("k"))
        #expect(pill.contains("₹") || pill.uppercased().contains("INR"))
        #expect(layout.fillIsLoss == true)
    }

    @Test func lastCumulativeGainIsNotLossFill() {
        let layout = layout(
            points: [
                TodayClosedChartPoint(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    cumulativeClosedPnL: 2_400
                ),
            ],
            quoteCurrency: "INR",
            brokerSlug: "kotak_neo"
        )
        #expect(layout.fillIsLoss == false)
    }

    @Test func emptyDoesNotInventFloorLineWhenFloorIsNil() {
        let layout = layout(
            points: [],
            floor: nil,
            quoteCurrency: "INR",
            brokerSlug: "kotak_neo"
        )
        #expect(layout.plotPoints.isEmpty)
        #expect(layout.floorY == nil)
        #expect(layout.lastPillText == nil)
        #expect(layout.fillIsLoss == nil)
    }

    private func kotakTwoPointLayout() -> TodayClosedFloorChartLayout {
        layout(
            points: [
                TodayClosedChartPoint(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    cumulativeClosedPnL: -1_000
                ),
                TodayClosedChartPoint(
                    closedAt: "2026-09-12T05:45:00.000Z",
                    cumulativeClosedPnL: -5_800
                ),
            ],
            quoteCurrency: "INR",
            brokerSlug: "kotak_neo"
        )
    }

    private func layout(
        points: [TodayClosedChartPoint],
        floor: Double? = nil,
        quoteCurrency: String?,
        brokerSlug: String?
    ) -> TodayClosedFloorChartLayout {
        TodayClosedFloorChartLayout.build(
            points: points,
            floor: floor,
            quoteCurrency: quoteCurrency,
            brokerSlug: brokerSlug,
            now: Date(),
            calendar: Calendar(identifier: .gregorian)
        )
    }
}
