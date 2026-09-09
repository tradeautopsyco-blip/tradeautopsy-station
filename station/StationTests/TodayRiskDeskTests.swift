import Foundation
import Notch
import Testing
@testable import Station

struct TodayRiskDeskTests {
    @Test func openNowDashesUnknownMarkToMarketStaysTheLock() {
        let payload = TodayAgentPayload(
            localDate: "2026-08-21",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: true,
            hero: TodayHeroPayload(pnlTodayUsd: nil, tradesToday: nil, winRate: nil),
            topSignals: [],
            trades: [],
            openPositionCount: 1
        )
        let positions = [
            DeskPosition(symbol: "RELIANCE", qty: 10, unrealizedPnL: nil, direction: "LONG"),
        ]
        let presentation = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: true,
            positions: positions
        )
        #expect(presentation.openRows.first?.mtmText == TodayScreenPresentation.emDash)
        #expect(presentation.openRows.first?.behaviorText == "Still open")
    }

    @Test func inspectorDoesNotInventMTMOrJournalImages() {
        let detect = DetectCard.evaluate(
            DetectCardInput(
                qty: 10,
                entry: nil,
                planStop: nil,
                liveStop: nil,
                sideBuy: true,
                accountEquity: nil,
                tradeCurrency: "INR",
                accountCurrency: "INR"
            )
        )
        let model = ThisTradeInspectorModel.fromOpenRow(
            symbol: "RELIANCE",
            sideText: "LONG",
            qtyText: "10",
            mtmText: "—",
            detect: detect
        )
        #expect(model.pulseMTMText == "—")
        #expect(model.journalStub.contains("T4 N2"))
        #expect(detect.kind == .noForm)
    }

    @Test func dualNoBlendStillBlocksUsdInrHero() {
        let resolution = DeskHonesty.resolve(activeSlugs: ["binance_com", "kotak_neo"])
        guard case .dualNoBlend = resolution else {
            Issue.record("expected dualNoBlend")
            return
        }
        #expect(DeskHonesty.heroQuoteCurrency(activeSlugs: ["binance_com", "kotak_neo"]) == nil)
    }
}
