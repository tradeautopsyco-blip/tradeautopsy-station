import Foundation
import Testing
@testable import Notch

struct AccountImpactTests {
    @Test func missingLimitIsUnknown() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", -10_400)],
            pinnedSymbols: [],
            dailyLossLimit: nil,
        )
        #expect(impact == .unknown)
        #expect(impact.chipLabel == "—")
        #expect(impact.trackFill == 0)
    }

    @Test func nonPositiveLimitIsUnknown() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", -10_400)],
            pinnedSymbols: [],
            dailyLossLimit: 0,
        )
        #expect(impact == .unknown)
    }

    @Test func emptyPinsUseAllOpenPositionsDown() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", -6_800), ("NIFTY", -3_600)],
            pinnedSymbols: [],
            dailyLossLimit: 32_500,
        )
        #expect(impact == .known(percent: 32, sign: .down))
        #expect(impact.chipLabel == "32↓")
        #expect(impact.trackFill == 0.32)
    }

    @Test func emptyPinsUseAllOpenPositionsUp() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", 6_800), ("NIFTY", 3_600)],
            pinnedSymbols: [],
            dailyLossLimit: 32_500,
        )
        #expect(impact == .known(percent: 32, sign: .up))
        #expect(impact.chipLabel == "32↑")
    }

    @Test func zeroPnLIsZeroChip() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", 0)],
            pinnedSymbols: [],
            dailyLossLimit: 32_500,
        )
        #expect(impact == .known(percent: 0, sign: .zero))
        #expect(impact.chipLabel == "0")
        #expect(impact.trackFill == 0)
    }

    @Test func pinnedSubsetIgnoresOtherNames() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", -6_800), ("NIFTY", -3_600)],
            pinnedSymbols: ["RELIANCE"],
            dailyLossLimit: 32_500,
        )
        #expect(impact == .known(percent: 21, sign: .down))
    }

    @Test func missingPinnedNameContributesZero() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", -6_800)],
            pinnedSymbols: ["NIFTY"],
            dailyLossLimit: 32_500,
        )
        #expect(impact == .known(percent: 0, sign: .zero))
    }

    @Test func percentCapsAt999() {
        let impact = AccountImpact.compute(
            positions: [("RELIANCE", -1_000_000)],
            pinnedSymbols: [],
            dailyLossLimit: 100,
        )
        #expect(impact == .known(percent: 999, sign: .down))
    }
}
