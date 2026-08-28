import Foundation
import Testing
@testable import Notch

struct BarOptionsDeclareTests {
    @Test func optionsTabUsesThreeZoneSpotDoesNot() {
        #expect(BarOptionsDeclareSurface.usesThreeZone(for: .options))
        #expect(!BarOptionsDeclareSurface.usesThreeZone(for: .spot))
        #expect(!BarOptionsDeclareSurface.usesThreeZone(for: .equity))
    }

    @Test func darkChainNeverShowsStrikeGrid() {
        let none = BarOptionsChainPresentation.from(underlying: "", chainStatus: .unavailable)
        #expect(none == .nothingDeclared)
        #expect(none.showsStrikeGrid == false)

        let hole = BarOptionsChainPresentation.from(underlying: "BANKNIFTY", chainStatus: .unavailable)
        #expect(hole == .unavailable)
        #expect(hole.showsStrikeGrid == false)

        let empty = BarOptionsChainPresentation.from(underlying: "BANKNIFTY", chainStatus: .empty)
        #expect(empty == .empty)
        #expect(empty.showsStrikeGrid == false)
    }

    @Test func premortemUsesDeclaredLimitWhenSigmaDark() {
        let q = BarOptionsPremortem.question(hasLegs: true, declaredMaxINR: 15000)
        #expect(q.contains("15000"))
        #expect(!q.contains("2σ"))
        let hint = BarOptionsPremortem.hint(hasLegs: true, declaredMaxINR: 15000)
        #expect(hint.contains("declared limit"))
    }

    @Test func stopRungFallsBackToDeclaredMaxWithoutEntry() {
        let rung = BarOptionsLadderModel.stopRung(
            units: 1,
            entry: nil,
            stop: 899,
            sideBuy: false,
            declaredMaxINR: 15000,
        )
        #expect(rung == -15000)
    }

    @Test func sigmaRungsStayUnavailable() {
        let rungs = BarOptionsLadderModel.rungs(
            hasLegs: true,
            stopText: "899",
            stopRung: -15000,
            declaredMaxINR: 15000,
            horizonDays: 1,
            expiryDTE: nil,
        )
        #expect(rungs.count == 5)
        #expect(rungs[0].amountINR == -15000)
        #expect(rungs[1].honesty == .unavailable)
        #expect(rungs[2].honesty == .unavailable)
        #expect(rungs[3].honesty == .unavailable)
        #expect(rungs[1].amountINR == nil)
    }

    @Test func expiryHorizonDoesNotInventDTE() {
        #expect(BarPlanHorizon.days(for: .expiry, dte: nil) == nil)
        #expect(BarPlanHorizon.days(for: .today, dte: nil) == 1)
        #expect(BarPlanHorizon.days(for: .threeSessions, dte: nil) == 3)
    }

    @Test func optionsConfirmSkipsSetupAndRequiresLeg() {
        var input = BarIntradayDeclarationSubmitInput(
            blocksDeclarationSubmit: false,
            protectiveSlConsent: false,
            calm: 2,
            confidence: 4,
            stopLossText: "899",
            symbolRaw: "BANKNIFTY",
            quantityText: "",
            setupType: "",
            invalidationTypeRaw: "",
            invalidationCondition: "It failed because IV crushed.",
            declarationKindWire: "intraday",
            scalperSessionId: "",
            isOptions: true,
            optionLegCount: 0,
            maxPlannedLossText: "15000",
        )
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)

        input.optionLegCount = 1
        let ready = BarIntradayDeclareValidator.submitReadiness(input)
        #expect(ready.ready == true)
        #expect(ready.hint == nil)
    }

    @Test func twoLegsRoundTripOnPayload() {
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: "BANKNIFTY",
            sideBuy: false,
            quantity: 1,
            stopLoss: 899,
            declarationKind: "intraday",
            moodStress: 2,
            moodImpulse: 4,
            invalidationNote: "limit hit",
            protectiveSlConsent: true,
            entryPrice: nil,
            targetPrice: 320,
            scalperSessionId: nil,
            optionLegs: [
                .init(underlying: "BANKNIFTY", expiry: "2026-09-29", strike: "57500", right: "PE", sideBuy: false, lots: 1),
                .init(underlying: "BANKNIFTY", expiry: "2026-09-29", strike: "57000", right: "PE", sideBuy: true, lots: 1),
            ],
            horizonDays: 1,
            maxPlannedLossINR: 15000,
        )
        let legs = obj["legs"] as? [[String: Any]]
        #expect(legs?.count == 2)
        #expect(legs?[1]["strike"] as? String == "57000")
        #expect(obj["max_planned_loss_inr"] as? Double == 15000)
    }
}
