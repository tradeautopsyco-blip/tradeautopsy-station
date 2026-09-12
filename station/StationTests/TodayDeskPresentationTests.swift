import Foundation
import Testing
@testable import Station

struct TodayDeskPresentationTests {
    @Test func remainingPreviewIsFloorMinusClosedUsedAndLabeledT8() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 7,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000
        )
        #expect(desk.remainingAmount == 6_200)
        #expect(desk.remainingText != TodayScreenPresentation.emDash)
        #expect(desk.remainingText.contains("6,200") || desk.remainingText.contains("6200"))
        #expect(desk.remainingBadge == "T8 preview · floor not live")
        #expect(desk.floorLine == 12_000)
    }

    @Test func kotakInrFloorUsedPercentIsFortyEightAndUsedCaptionContainsFiveThousandEightHundred() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000
        )
        #expect(desk.floorUsedPercentText.contains("48%"))
        #expect(desk.floorUsedPercentText.contains("12,000") || desk.floorUsedPercentText.contains("12000"))
        #expect(desk.usedCaption.contains("5,800") || desk.usedCaption.contains("5800"))
    }

    @Test func missingFloorStaysDashAndDoesNotInventTwelveThousand() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: nil
        )
        #expect(desk.remainingAmount == nil)
        #expect(desk.remainingText == TodayScreenPresentation.emDash)
        #expect(!desk.remainingText.contains("12,000"))
        #expect(!desk.remainingText.contains("12000"))
        #expect(desk.usedCaption == TodayScreenPresentation.emDash)
        #expect(desk.floorUsedPercentText == TodayScreenPresentation.emDash)
        #expect(!desk.usedCaption.contains("12,000"))
        #expect(!desk.usedCaption.contains("12000"))
        #expect(!desk.floorUsedPercentText.contains("12,000"))
        #expect(!desk.floorUsedPercentText.contains("12000"))
        #expect(desk.floorLine == nil)
        #expect(desk.remainingBadge == "T8 preview · floor not live")
    }

    @Test func dualNoBlendDashesRemainingWhenQuoteIsMissing() {
        let payload = TodayAgentPayload(
            localDate: "2026-09-12",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: -5_800, tradesToday: 1, winRate: 0),
            topSignals: [],
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ],
            openPositionCount: 0,
            brokerSlug: nil,
            quoteCurrency: nil
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000
        )
        #expect(desk.remainingAmount == nil)
        #expect(desk.remainingText == TodayScreenPresentation.emDash)
        #expect(!desk.remainingText.contains("INR"))
        #expect(desk.quoteCurrency == nil)
        #expect(desk.usedCaption == TodayScreenPresentation.emDash)
        #expect(desk.floorUsedPercentText == TodayScreenPresentation.emDash)
        #expect(desk.brokerSlug == nil)
    }

    @Test func kotakClockTextIsFourteenOhSevenAndDateHeadlineIsSaturdayTwelveSeptember() {
        var ist = Calendar(identifier: .gregorian)
        ist.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        let now = ist.date(from: DateComponents(year: 2026, month: 9, day: 12, hour: 14, minute: 7))!
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000,
            now: now
        )
        #expect(desk.clockText == "14:07")
        #expect(desk.dateHeadline.contains("Saturday"))
        #expect(desk.dateHeadline.contains("12"))
        #expect(desk.dateHeadline.contains("September"))
        #expect(desk.brokerSlug == "kotak_neo")
        #expect(desk.daySpineSubtitle == "One local day · session open · 14:07")
    }

    @Test func splitClocksSubtitleIsTwoClocksHappenedAndNow() {
        #expect(TodayDeskPresentation.splitClocksSubtitle == "Two clocks · happened and now")
    }

    @Test func chartReadoutStartsWithClockContainsClosedRemainingAndRefusesOpenMtm() {
        var ist = Calendar(identifier: .gregorian)
        ist.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        let now = ist.date(from: DateComponents(year: 2026, month: 9, day: 12, hour: 14, minute: 7))!
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let positions = [
            DeskPosition(symbol: "TCS", qty: 10, unrealizedPnL: 900, direction: "LONG"),
        ]
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions,
            dailyFloor: 12_000,
            now: now
        )
        #expect(desk.chartReadout.hasPrefix(desk.clockText))
        #expect(desk.chartReadout.contains("closed"))
        #expect(desk.chartReadout.contains("6,200") || desk.chartReadout.contains("6200") || desk.chartReadout.contains(desk.remainingText))
        #expect(desk.chartReadout.contains("open MTM not on this line"))
        #expect(!desk.chartReadout.contains("900"))
        #expect(!desk.chartReadout.contains("T8 preview"))
    }

    @Test func kotakInrUsedCaptionUsesRupeeOrInrNotUsd() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000
        )
        #expect(desk.usedCaption.contains("₹") || desk.usedCaption.contains("INR"))
        #expect(!desk.usedCaption.contains("USD"))
        #expect(!desk.usedCaption.contains("$"))
    }

    @Test func dualNoBlendDashesRemainingWhenTwoDesksHaveNoActiveQuote() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000,
            showActiveMoney: false
        )
        #expect(desk.remainingAmount == nil)
        #expect(desk.remainingText == TodayScreenPresentation.emDash)
        #expect(desk.chartPoints.isEmpty)
        #expect(desk.screen.heroTiles.allSatisfy { $0.value == TodayScreenPresentation.emDash })
        #expect(desk.usedCaption == TodayScreenPresentation.emDash)
        #expect(desk.floorUsedPercentText == TodayScreenPresentation.emDash)
    }

    @Test func chartIsThisDayClosedCumulativeAndExcludesOpenMTM() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 2,
            trades: [
                trade(closedAt: "2026-09-12T05:45:00.000Z", symbol: "HDFCBANK", net: -4_800),
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -1_000),
            ]
        )
        let positions = [
            DeskPosition(symbol: "TCS", qty: 10, unrealizedPnL: 900, direction: "LONG"),
        ]
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions,
            dailyFloor: 12_000
        )
        #expect(desk.chartPoints.map(\.cumulativeClosedPnL) == [-1_000, -5_800])
        #expect(desk.chartPoints.map(\.closedAt) == [
            "2026-09-12T04:11:00.000Z",
            "2026-09-12T05:45:00.000Z",
        ])
        #expect(desk.floorLine == 12_000)
        #expect(desk.nowOpenCount == 1)
    }

    @Test func heroIsClosedOnlyAndDoesNotFoldOpenMTM() {
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(closedAt: "2026-09-12T04:11:00.000Z", symbol: "RELIANCE", net: -5_800),
            ]
        )
        let positions = [
            DeskPosition(symbol: "TCS", qty: 10, unrealizedPnL: 900, direction: "LONG"),
        ]
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions,
            dailyFloor: 12_000
        )
        let hero = desk.screen.heroTiles.first { $0.id == "pnl" }
        #expect(hero?.value.contains("5,800") == true || hero?.value.contains("5800") == true)
        #expect(hero?.value.contains("4,900") != true)
        #expect(hero?.caption.contains("closed") == true || hero?.caption.contains("fees") == true)
        #expect(desk.remainingAmount == 6_200)
        #expect(desk.screen.openRows.first?.mtmText.contains("900") == true)
    }

    @Test func overnightTagRequiresFirstFilledAtOnAPriorLocalDate() {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        let firstFill = calendar.date(from: DateComponents(year: 2026, month: 9, day: 11, hour: 15, minute: 20))!
        let payload = kotakPayload(
            pnl: nil,
            tradesToday: nil,
            trades: []
        )
        let positions = [
            DeskPosition(
                symbol: "RELIANCE",
                qty: 50,
                unrealizedPnL: nil,
                direction: "LONG",
                firstFilledAt: firstFill
            ),
        ]
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions,
            dailyFloor: nil,
            calendar: calendar
        )
        #expect(desk.screen.openRows.first?.behaviorText == "Overnight")
    }

    @Test func missingFillDateKeepsStillOpenAndDoesNotClaimOvernight() {
        let payload = kotakPayload(
            pnl: nil,
            tradesToday: nil,
            trades: []
        )
        let positions = [
            DeskPosition(symbol: "RELIANCE", qty: 50, unrealizedPnL: nil, direction: "LONG"),
        ]
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions
        )
        #expect(desk.screen.openRows.first?.behaviorText == "Still open")
        #expect(desk.screen.openRows.first?.behaviorText != "Overnight")
    }

    @Test func kotakCashQtyTwoDoesNotRecomputeOptionsLotPnlOntoRemaining() {
        // Cash book lock refuses FO; presentation does not recompute OPTIONS lot PnL.
        let payload = kotakPayload(
            pnl: -5_800,
            tradesToday: 1,
            trades: [
                trade(
                    closedAt: "2026-09-12T04:11:00.000Z",
                    symbol: "RELIANCE",
                    net: -5_800,
                    avgEntry: 10,
                    avgExit: 11,
                    qty: 2
                ),
            ]
        )
        let desk = TodayDeskPresentation.build(
            payload: payload,
            agentHealthy: true,
            dailyFloor: 12_000
        )
        #expect(desk.remainingAmount == 6_200)
        #expect(desk.remainingAmount != 100)
        #expect(desk.remainingAmount != 2)
        #expect(desk.chartPoints.last?.cumulativeClosedPnL == -5_800)
        let hero = desk.screen.heroTiles.first { $0.id == "pnl" }
        #expect(hero?.value.contains("5,800") == true || hero?.value.contains("5800") == true)
        #expect(hero?.value.contains("100") != true)
    }

    private func kotakPayload(
        pnl: Double?,
        tradesToday: Int?,
        trades: [TodayTradeRowPayload],
        quote: String? = "INR"
    ) -> TodayAgentPayload {
        TodayAgentPayload(
            localDate: "2026-09-12",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: pnl, tradesToday: tradesToday, winRate: 0.4),
            topSignals: [],
            trades: trades,
            openPositionCount: 0,
            brokerSlug: "kotak_neo",
            quoteCurrency: quote
        )
    }

    private func trade(
        closedAt: String,
        symbol: String,
        net: Double?,
        avgEntry: Double = 100,
        avgExit: Double = 90,
        qty: Double = 1
    ) -> TodayTradeRowPayload {
        TodayTradeRowPayload(
            closedAt: closedAt,
            symbol: symbol,
            avgEntry: avgEntry,
            avgExit: avgExit,
            qty: qty,
            netPnlUsd: net,
            primaryFlag: "Clean",
            flagSeverity: "clean",
            dataQualityFlags: []
        )
    }
}
