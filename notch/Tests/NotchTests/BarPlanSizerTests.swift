import Foundation
import Testing
@testable import Notch

struct BarPlanSizerTests {
    @Test func sizeDashedWhenFundsDark() {
        let p = BarPlanSizer.present(
            BarPlanSizer.Inputs(
                dailyFloor: 12_000,
                entry: 100,
                invalidationPrice: 95,
                stopLoss: nil,
                fundsLit: false
            ),
            quoteCurrency: "INR"
        )
        #expect(p.sizeText == "—")
        #expect(p.sizeEmphasisDashed)
        #expect(p.plannedRiskText == "INR 12000")
        #expect(p.footnote.contains("reference doc"))
    }

    @Test func sizeNotDashedWhenFundsAndInvalidationLitButFormulaBlocked() {
        let p = BarPlanSizer.present(
            BarPlanSizer.Inputs(
                dailyFloor: nil,
                entry: 100,
                invalidationPrice: nil,
                stopLoss: 95,
                fundsLit: true
            ),
            quoteCurrency: "USDT"
        )
        #expect(p.sizeText == "—")
        #expect(!p.sizeEmphasisDashed)
        #expect(p.plannedRiskText == "—")
    }

    @Test func invalidationDarkWhenEntryMissing() {
        let p = BarPlanSizer.present(
            BarPlanSizer.Inputs(
                dailyFloor: 5_000,
                entry: nil,
                invalidationPrice: 95,
                stopLoss: nil,
                fundsLit: true
            ),
            quoteCurrency: "INR"
        )
        #expect(p.sizeEmphasisDashed)
    }
}
