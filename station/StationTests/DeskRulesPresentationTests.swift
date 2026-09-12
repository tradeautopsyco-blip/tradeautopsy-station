import Foundation
import Testing
@testable import Station

struct DeskRulesPresentationTests {
    @Test func usedTodayFromClosedLossIsPositiveUsed() {
        #expect(DeskRulesPresentation.usedTodayAmount(closedPnL: -5_800, healthy: true) == 5_800)
    }

    @Test func usedTodayFromClosedProfitIsZero() {
        #expect(DeskRulesPresentation.usedTodayAmount(closedPnL: 2_000, healthy: true) == 0)
    }

    @Test func usedTodayMissingWhenUnhealthyOrNoClosedPnL() {
        #expect(DeskRulesPresentation.usedTodayAmount(closedPnL: -5_800, healthy: false) == nil)
        #expect(DeskRulesPresentation.usedTodayAmount(closedPnL: nil, healthy: true) == nil)
    }

    @Test func usedTodayCaptionPaintsQuoteOfActiveBook() {
        let caption = DeskRulesPresentation.usedTodayCaption(
            closedPnL: -5_800,
            quoteCurrency: "INR",
            healthy: true
        )
        #expect(caption.hasPrefix("Used today"))
        #expect(caption.contains("5,800") || caption.contains("5800"))
        #expect(caption.contains("−") || caption.contains("-") || caption.contains("₹"))
    }

    @Test func usedTodayCaptionIsDashWhenUnhealthy() {
        #expect(
            DeskRulesPresentation.usedTodayCaption(
                closedPnL: -5_800,
                quoteCurrency: "INR",
                healthy: false
            ) == "—"
        )
    }

    @Test func dualNoBlendDashesWhenQuoteIsMissing() {
        #expect(
            DeskRulesPresentation.usedTodayCaption(
                closedPnL: -5_800,
                quoteCurrency: nil,
                healthy: true
            ) == "—"
        )
    }

    @Test func floorUsedWholePercentIsFortyEightForFiveThousandEightHundredOfTwelveThousand() {
        #expect(DeskRulesPresentation.floorUsedWholePercent(floor: 12_000, used: 5_800) == 48)
        #expect(DeskRulesPresentation.floorUsedWholePercent(floor: nil, used: 5_800) == nil)
        #expect(DeskRulesPresentation.floorUsedWholePercent(floor: 12_000, used: nil) == nil)
    }

    @Test func remainingPreviewIsFloorMinusUsedWhenBothExist() {
        #expect(DeskRulesPresentation.remainingPreview(floor: 12_000, used: 5_800) == 6_200)
        #expect(DeskRulesPresentation.remainingPreview(floor: 12_000, used: nil) == nil)
        #expect(DeskRulesPresentation.remainingPreview(floor: nil, used: 5_800) == nil)
    }

    @Test func neverEncodesALossLimitsPOST() {
        #expect(
            DeskRulesPresentation.lossLimitsRequest(
                dailyFloor: 12_000,
                meanLoss: 5_400,
                maxRoundTrips: 12
            ) == nil
        )
    }

    @Test func apiKeysDestinationIsBrokers() {
        #expect(DeskRulesPresentation.navigationDestination(for: .apiKeys) == .brokers)
        #expect(DeskRulesPresentation.navigationDestination(for: .openNotch) == nil)
    }
}
