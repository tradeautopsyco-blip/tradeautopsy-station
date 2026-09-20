import Foundation
import Testing
@testable import Notch

struct BarWave2PlanFieldTests {
    private func wave2ReadyInput(
        setup: String = "Breakout",
        isOptions: Bool = false,
        isUsdm: Bool = false,
        requiresCash: Bool = false,
        product: String = "MIS",
        invKind: String = "price",
        invLine: String = "Last through 1390.",
        invPrice: String = "1390",
        optionLegs: Int = 0
    ) -> BarIntradayDeclarationSubmitInput {
        BarIntradayDeclarationSubmitInput(
            blocksDeclarationSubmit: false,
            protectiveSlConsent: !isUsdm,
            calm: 2,
            confidence: 3,
            stopLossText: "1400",
            symbolRaw: isOptions ? "BANKNIFTY" : "RELIANCE",
            quantityText: "10",
            setupType: setup,
            invalidationTypeRaw: invKind,
            invalidationCondition: invLine,
            declarationKindWire: "intraday",
            scalperSessionId: "",
            lotsText: optionLegs > 0 ? "1" : "",
            isOptions: isOptions,
            isUsdm: isUsdm,
            optionLegCount: optionLegs,
            maxPlannedLossText: isOptions ? "15000" : "",
            frustration: 1,
            excitement: 2,
            stanceRaw: "planned",
            intent: "Range break, volume confirmed.",
            targetPriceText: "1500",
            invalidationPriceText: invPrice,
            requiresCashProduct: requiresCash,
            cashProduct: product,
            gate: BarPlanGateStripState(
                capitalAck: true,
                onePercentAck: true,
                maxLossAck: true,
                hedgeAck: true,
                reviewAck: true
            )
        )
    }

    @Test func cashConfirmRequiresFullSnapshot() {
        var input = wave2ReadyInput(requiresCash: true)
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready)

        input.intent = ""
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)

        input = wave2ReadyInput(requiresCash: true)
        input.targetPriceText = ""
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)

        input = wave2ReadyInput(requiresCash: true, product: "")
        #expect(BarIntradayDeclareValidator.submitReadiness(input).hint?.contains("CNC") == true)

        input = wave2ReadyInput(requiresCash: true)
        input.gate.capitalAck = false
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)
    }

    @Test func priceInvalidationDoesNotRequirePremortemLine() {
        var input = wave2ReadyInput(invLine: "", invPrice: "1390")
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready)
        input.invalidationPriceText = ""
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)
    }

    @Test func calmFourDoesNotChangeQuantityOnWire() {
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: "RELIANCE",
            sideBuy: true,
            quantity: 10,
            stopLoss: 1400,
            declarationKind: "intraday",
            moodStress: 4,
            moodImpulse: 3,
            invalidationNote: nil,
            protectiveSlConsent: true,
            entryPrice: 1420,
            targetPrice: 1500,
            scalperSessionId: nil,
            setupTypeLabel: "Breakout",
            invalidationTypeWire: "price",
            bookId: "kotak-nse-bse-cash",
            moodFrustration: 2,
            moodExcitement: 1,
            stance: "planned",
            intent: "Range break.",
            invalidationPrice: 1390,
            cashProduct: "MIS"
        )
        #expect(obj["quantity"] as? Double == 10)
        let payload = obj["declaration_payload"] as? [String: Any]
        let s1 = payload?["s1"] as? [String: Any]
        #expect(s1?["intent"] as? String == "Range break.")
        #expect(s1?["stance"] as? String == "planned")
        #expect(s1?["product"] as? String == "MIS")
        #expect(s1?["invalidation_price"] as? Double == 1390)
        #expect(s1?["mood_frustration"] as? Double == 2)
    }

    @Test func optionsStillSkipSetupWhenWave2Filled() {
        var input = wave2ReadyInput(setup: "", isOptions: true, invKind: "", invLine: "IV crushed.", invPrice: "", optionLegs: 1)
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready)
        input.optionLegCount = 0
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)
    }

    @Test func usdmConfirmDoesNotRequireAutoPlaceStopWhenWave2Filled() {
        let input = wave2ReadyInput(
            setup: "breakout",
            isUsdm: true,
            invKind: "behaviour",
            invLine: "Last through invalidation.",
            invPrice: ""
        )
        let ready = BarIntradayDeclareValidator.submitReadiness(input)
        #expect(ready.ready)
        #expect(ready.hint == nil)
    }
}
