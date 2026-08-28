import Foundation
import Testing
@testable import Notch

struct BarIntradayDeclarationPayloadTests {
    @Test func oneLegOptionsConfirmContainsLegFields() {
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: "NIFTY",
            sideBuy: true,
            quantity: 99,
            stopLoss: 80,
            declarationKind: "intraday",
            moodStress: 2,
            moodImpulse: 4,
            invalidationNote: "If 24700 breaks",
            protectiveSlConsent: true,
            entryPrice: 100,
            targetPrice: nil,
            scalperSessionId: nil,
            optionLeg: BarIntradayDeclarationPayload.OptionLeg(
                underlying: "NIFTY",
                expiry: "2026-09-24",
                strike: "25000",
                right: "CE",
                sideBuy: true,
                lots: 1,
            ),
            horizonDays: 1,
            maxPlannedLossINR: 20,
        )

        #expect(obj["symbol"] as? String == "NIFTY")
        #expect(obj["side"] as? String == "BUY")
        #expect(obj["quantity"] as? Double == 1)
        #expect(obj["stop_loss"] as? Double == 80)
        #expect(obj["horizon_days"] as? Int == 1)
        #expect(obj["max_planned_loss_inr"] as? Double == 20)

        let legs = obj["legs"] as? [[String: Any]]
        #expect(legs?.count == 1)
        let leg = legs?[0]
        #expect(leg?["underlying"] as? String == "NIFTY")
        #expect(leg?["expiry"] as? String == "2026-09-24")
        #expect(leg?["strike"] as? String == "25000")
        #expect(leg?["right"] as? String == "CE")
        #expect(leg?["side"] as? String == "BUY")
        #expect(leg?["lots"] as? Int == 1)
    }

    @Test func spotDoesNotSendLegs() {
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: "RELIANCE",
            sideBuy: false,
            quantity: 10,
            stopLoss: 1400,
            declarationKind: "swing",
            moodStress: 1,
            moodImpulse: 5,
            invalidationNote: nil,
            protectiveSlConsent: true,
            entryPrice: 1420,
            targetPrice: 1500,
            scalperSessionId: nil,
            horizonDays: 5,
            maxPlannedLossINR: 200,
        )
        #expect(obj["legs"] == nil)
        #expect(obj["quantity"] as? Double == 10)
        #expect(obj["horizon_days"] as? Int == 5)
        #expect(obj["side"] as? String == "SELL")
    }
}
