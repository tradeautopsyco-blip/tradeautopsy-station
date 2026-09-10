import Foundation
import Testing
@testable import Notch

struct BarCryptoSettlementTests {
    @Test func longCallOtmIsMinusPremiumAndNeverBelow() {
        let k = 100.0
        let premium = 2.0
        let otm = BarCryptoSettlement.pnlPerContract(
            s: 90, strike: k, right: .call, side: .buy, premium: premium
        )
        #expect(otm == -2)
        let itm = BarCryptoSettlement.pnlPerContract(
            s: 110, strike: k, right: .call, side: .buy, premium: premium
        )
        #expect(itm == 8)
        for s in [0.0, 50, 99.9, 100, 1000] {
            let pnl = BarCryptoSettlement.pnlPerContract(
                s: s, strike: k, right: .call, side: .buy, premium: premium
            )
            #expect(pnl >= -premium)
        }
    }

    @Test func longPutFlipsWithRight() {
        let k = 100.0
        let premium = 3.0
        #expect(BarCryptoSettlement.pnlPerContract(
            s: 110, strike: k, right: .put, side: .buy, premium: premium
        ) == -3)
        #expect(BarCryptoSettlement.pnlPerContract(
            s: 90, strike: k, right: .put, side: .buy, premium: premium
        ) == 7)
    }

    @Test func shortCallHasNoFiniteMaxLossAndOpenWing() {
        let k = 100.0
        let premium = 2.0
        let otm = BarCryptoSettlement.pnlPerContract(
            s: 90, strike: k, right: .call, side: .sell, premium: premium
        )
        #expect(otm == 2)
        let deep = BarCryptoSettlement.pnlPerContract(
            s: 1000, strike: k, right: .call, side: .sell, premium: premium
        )
        #expect(deep == -898)
        let shape = BarCryptoSettlement.polyline(
            strike: k, right: .call, side: .sell, premium: premium, spotS: 90
        )
        #expect(shape.openWing)
        let far = shape.points.map(\.s).max() ?? 0
        #expect(far > k)
        let input = BarCryptoSettlement.Inputs(
            datedContractBound: true,
            strikeText: "100",
            right: .call,
            side: .sell,
            premiumText: "2",
            contractCount: 1,
            indexPriceText: "90",
            expiryDateMs: 1_789_430_400_000,
            nowMs: 1_788_998_400_000
        )
        guard case let .lit(lit) = BarCryptoSettlement.evaluate(input) else {
            Issue.record("short call should light")
            return
        }
        #expect(lit.longMaxLoss == nil)
        #expect(lit.openWing)
        #expect(!lit.caption.localizedCaseInsensitiveContains("unlimited"))
        #expect(!lit.caption.contains("Unlimited"))
    }

    @Test func indexUnavailableKeepsHoleNoSpotStitch() {
        var input = BarCryptoSettlement.Inputs(
            datedContractBound: true,
            strikeText: "100",
            right: .call,
            side: .buy,
            premiumText: "2",
            contractCount: 1,
            indexPriceText: nil,
            expiryDateMs: 1_789_430_400_000,
            nowMs: 1_788_998_400_000
        )
        guard case let .hole(reason) = BarCryptoSettlement.evaluate(input) else {
            Issue.record("missing index must stay a hole")
            return
        }
        #expect(reason == BarCryptoSettlement.holeIndexMissing)
        #expect(!reason.contains("OPTIONS-PRICING"))
        #expect(!reason.localizedCaseInsensitiveContains("lastPrice"))
        input.indexPriceText = ""
        guard case .hole = BarCryptoSettlement.evaluate(input) else {
            Issue.record("empty index string must stay a hole")
            return
        }
    }

    @Test func subCentPremiumIsNotRoundedToZero() {
        let pnl = BarCryptoSettlement.pnlPerContract(
            s: 1.0, strike: 2.0, right: .call, side: .buy, premium: 0.0012
        )
        #expect(pnl == -0.0012)
        let formatted = BarCryptoSettlement.formatUsdt(0.0012)
        #expect(formatted.contains("0.0012"))
        #expect(!formatted.hasPrefix("0.00 USDT"))
        #expect(BarCryptoSettlement.parsePremium("0.0012") == 0.0012)
    }

    @Test func captionIsPerContractUnitNotAppliedNotModelComputed() {
        let input = BarCryptoSettlement.Inputs(
            datedContractBound: true,
            strikeText: "100",
            right: .call,
            side: .buy,
            premiumText: "2",
            contractCount: 3,
            indexPriceText: "90",
            expiryDateMs: 1_789_430_400_000,
            nowMs: 1_788_998_400_000
        )
        guard case let .lit(lit) = BarCryptoSettlement.evaluate(input) else {
            Issue.record("long call should light")
            return
        }
        #expect(lit.caption.contains("per contract"))
        #expect(lit.caption.contains("unit is not applied"))
        #expect(lit.caption.contains("GET /eapi/v1/index"))
        #expect(lit.caption.contains("not ModelComputed"))
        #expect(!lit.caption.contains("OPTIONS-PRICING"))
        #expect(!lit.caption.contains("Black-76"))
        #expect(!lit.caption.contains(" 1/50"))
        #expect(!lit.caption.contains("unit=1"))
        #expect(lit.longMaxLoss == 2)
        // Y is per contract even when count is 3.
        let otm = BarCryptoSettlement.pnlPerContract(
            s: 90, strike: 100, right: .call, side: .buy, premium: 2
        )
        #expect(otm == -2)
        #expect(otm != -6)
    }

    @Test func dteFromKnownExpiryDateMsFixture() {
        // 2026-09-10 00:00 UTC vs 2026-09-15 00:00 UTC → 5 calendar days.
        let nowMs: Int64 = 1_788_998_400_000
        let expiryMs: Int64 = 1_789_430_400_000
        #expect(BarCryptoSettlement.calendarDTE(expiryDateMs: expiryMs, nowMs: nowMs) == 5)
        // After expiry floors at 0.
        #expect(BarCryptoSettlement.calendarDTE(expiryDateMs: 1_596_067_200_000, nowMs: nowMs) == 0)
        // Dated-contract parse: official CLI example 200730.
        let fromId = BarCryptoSettlement.expiryDateMs(fromDatedContract: "BTC-200730-9000-C")
        #expect(fromId != nil)
        #expect(BarCryptoSettlement.calendarDTE(expiryDateMs: fromId!, nowMs: nowMs) == 0)
    }

    @Test func expiryHorizonDoesNotInventDTEWhenExpiryMissing() {
        #expect(BarPlanHorizon.days(for: .expiry, dte: nil) == nil)
        #expect(BarPlanHorizon.days(for: .today, dte: nil) == 1)
        let input = BarCryptoSettlement.Inputs(
            datedContractBound: true,
            strikeText: "100",
            right: .call,
            side: .buy,
            premiumText: "2",
            contractCount: 1,
            indexPriceText: "90",
            expiryDateMs: nil,
            nowMs: 1_788_998_400_000
        )
        guard case let .lit(lit) = BarCryptoSettlement.evaluate(input) else {
            Issue.record("shape can light without DTE")
            return
        }
        #expect(lit.dteKnown == false)
        #expect(BarPlanHorizon.days(for: .expiry, dte: lit.dteKnown ? lit.dte : nil) == nil)
    }

    @Test func nfoThreeZonePayoffCopyUnchanged() {
        #expect(BarNfoPayoffCopy.holeTitle == "chain unavailable")
        #expect(BarNfoPayoffCopy.holeBody == "derived/payoff inherits market/option_chain")
        #expect(!BarNfoPayoffCopy.holeBody.contains("GET /eapi/v1/index"))
        #expect(!BarNfoPayoffCopy.holeBody.contains("settlement identity"))
    }
}
