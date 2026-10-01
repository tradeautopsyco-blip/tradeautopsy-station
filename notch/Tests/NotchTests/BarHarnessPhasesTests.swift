import Foundation
import Testing
@testable import Notch

struct BarHarnessPhasesTests {
    @Test func openStartRequiresRuleOnly() {
        #expect(!BarOpenStartGate.unlocked(rule: "  "))
        #expect(BarOpenStartGate.unlocked(rule: "No chase"))
        #expect(!BarOpenStartGate.showsPreMarketIndices(morningBrief: nil))
    }

    @Test func preMarketIndicesNeedBriefPrints() {
        let brief = MorningBrief(
            summary: "",
            niftyFutures: 24_000,
            bankniftyFutures: 0,
            niftyChangePct: 0,
            bankniftyChangePct: 0,
            vix: 0,
            edgeSymbols: [],
            cautionSymbols: [],
            recommendation: "",
            fetchedAt: Date(),
            tradeCount: 0,
            isNewUser: true,
            patterns: [],
            ownMetrics: nil,
            behavioralDateLine: nil,
            behavioralHeadline: nil,
            sessionPnLKpi: nil,
            planAdherencePct: nil,
            winRateKpi: nil,
            leftOnTableInr: nil,
            nonNegotiableRule: nil
        )
        #expect(BarOpenStartGate.showsPreMarketIndices(morningBrief: brief))
    }

    @Test func workingHarnessFlatVsDeclared() {
        let pending = BarPendingDeclaration(
            id: "a",
            status: "PENDING",
            createdAt: nil,
            symbol: "X",
            side: "BUY",
            quantity: 1,
            declarationKind: "intraday",
            protectiveSlConsent: true,
            stopLoss: 1,
            target: 2,
            planSnapshot: nil,
            filledQty: nil,
            avgFill: nil,
            fillSymbol: nil,
            fillSide: nil
        )
        #expect(
            BarWorkingHarnessDetailResolver.resolve(
                pendingRows: [],
                selectedPending: nil,
                hasOpenPositions: false,
                hasUndeclaredInventory: false
            ) == .flat
        )
        #expect(
            BarWorkingHarnessDetailResolver.resolve(
                pendingRows: [pending],
                selectedPending: pending,
                hasOpenPositions: false,
                hasUndeclaredInventory: false
            ) == .declaredNotFilled
        )
    }

    @Test func debriefCitedNetUniqueOrDash() {
        let trips = [("RELIANCE", 100.0), ("RELIANCE", 200.0)]
        #expect(
            BarDebriefCitedNet.displayUniqueOrAmbiguous(
                optionsOrNfo: false,
                symbol: "RELIANCE",
                trips: trips,
                currency: "INR"
            ).contains("multiple")
        )
        #expect(
            BarDebriefCitedNet.netUniqueSymbol(symbol: "RELIANCE", trips: [("RELIANCE", 50)]) == 50
        )
    }

    @Test func slSuggestorBlockedWithoutReference() {
        let p = BarPlanSlSuggestor.present(
            BarPlanSlSuggestor.Input(
                riskPercentOfMargin: 2,
                marginLit: true,
                marginDisplay: "INR 10000",
                entry: 100,
                sideBuy: true,
                bookId: "kotak-nse-bse-cash"
            )
        )
        #expect(p.suggestedStopText == "—")
        #expect(p.footnote.contains("POSITION-SIZING"))
        #expect(p.suggestedSizeText == "—")
    }

    @Test func slSuggestorShowsAuthoredSizeFromPreview() {
        let p = BarPlanSlSuggestor.present(
            BarPlanSlSuggestor.Input(
                riskPercentOfMargin: 1,
                marginLit: true,
                marginDisplay: "USDT 5000",
                entry: 100,
                sideBuy: true,
                bookId: "binance-com-spot",
                authoredSizeText: "10"
            )
        )
        #expect(p.suggestedSizeText == "10")
        #expect(p.suggestedStopText == "—")
    }

    @Test func freeAmountReadsGlanceText() {
        #expect(BarPlanRiskPreview.freeAmount(from: "INR 19.41") == 19.41)
        #expect(BarPlanRiskPreview.freeAmount(from: "—") == nil)
    }

    @Test func dataVendorHonestyNeverInventsChain() {
        let body = BarDataVendorHonesty.body(status: "unavailable", capability: "option_chain")
        #expect(body.contains("vendor"))
        #expect(!body.contains("0.00"))
    }

    @Test func riskPreviewLocalTypedQtyRisk() {
        let p = BarPlanRiskPreview.localPresentation(
            BarPlanRiskPreview.Request(
                bookId: "binance-com-spot",
                sideBuy: true,
                budgetMode: .fixedMoney,
                budgetValue: 100,
                entry: 100,
                stop: 90,
                target: 120,
                overrideQty: 2,
                fundsLit: true,
                fundsBalance: nil,
                fundsDisplayText: "USDT 5000",
                priceIncrement: nil,
                multiplier: nil,
                unitBatchSize: nil,
                instrumentRole: "spot",
                leverage: nil,
                symbol: "BTCUSDT"
            ),
            quoteCurrency: "USDT"
        )
        #expect(p.proposedSizeDashed)
        #expect(p.riskText != "—")
    }
}
