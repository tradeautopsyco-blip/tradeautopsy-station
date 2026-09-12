import Foundation
import Notch
import Testing
@testable import Station

struct TodayDetectJoinTests {
    @Test func uncoveredOpenSymbolIsNoFormLikeJournalImpulsive() {
        let position = DeskPosition(symbol: "BANKNIFTY", qty: 15, unrealizedPnL: nil, direction: "LONG")
        let declarations = [card(status: "pending", symbol: "RELIANCE", stop: 1100)]
        let input = TodayDetectJoin.input(
            position: position,
            declarations: declarations,
            quoteCurrency: "INR"
        )
        #expect(input != nil)
        #expect(input?.planStop == nil)
        #expect(input?.entry == nil)
        #expect(input?.liveStop == nil)
        #expect(DetectCard.evaluate(input!).kind == .noForm)
    }

    @Test func pendingDeclarationSuppliesPlanStopAndDoesNotInventEntry() {
        let position = DeskPosition(symbol: "RELIANCE", qty: 50, unrealizedPnL: nil, direction: "LONG")
        let declarations = [card(status: "pending", symbol: "reliance", stop: 1100)]
        let input = TodayDetectJoin.input(
            position: position,
            declarations: declarations,
            quoteCurrency: "INR"
        )
        #expect(input?.planStop == 1100)
        #expect(input?.entry == nil)
        #expect(input?.liveStop == nil)
        #expect(input?.qty == 50)
        #expect(input?.tradeCurrency == "INR")
    }

    @Test func cancelledOrExpiredDeclarationDoesNotCoverTheSymbol() {
        let position = DeskPosition(symbol: "HDFCBANK", qty: 20, unrealizedPnL: nil, direction: "LONG")
        let declarations = [
            card(status: "cancelled", symbol: "HDFCBANK", stop: 1640),
            card(status: "expired", symbol: "HDFCBANK", stop: 1640),
        ]
        let input = TodayDetectJoin.input(
            position: position,
            declarations: declarations,
            quoteCurrency: "INR"
        )
        #expect(input?.planStop == nil)
        #expect(DetectCard.evaluate(input!).kind == .noForm)
    }

    @Test func missingQuoteDoesNotDefaultToINR() {
        let position = DeskPosition(symbol: "RELIANCE", qty: 50, unrealizedPnL: nil, direction: "LONG")
        let input = TodayDetectJoin.input(
            position: position,
            declarations: [],
            quoteCurrency: nil
        )
        #expect(input?.tradeCurrency != "INR")
        #expect(input?.accountCurrency != "INR")
        #expect(input?.planStop == nil)
    }

    @Test func liveStopIsUsedOnlyWhenProvidedNeverInvented() {
        let position = DeskPosition(symbol: "RELIANCE", qty: 50, unrealizedPnL: nil, direction: "LONG")
        let withLive = TodayDetectJoin.input(
            position: position,
            declarations: [card(status: "matched", symbol: "RELIANCE", stop: 1100)],
            quoteCurrency: "INR",
            liveStop: 1150
        )
        let withoutLive = TodayDetectJoin.input(
            position: position,
            declarations: [card(status: "matched", symbol: "RELIANCE", stop: 1100)],
            quoteCurrency: "INR"
        )
        #expect(withLive?.liveStop == 1150)
        #expect(withLive?.planStop == 1100)
        #expect(withoutLive?.liveStop == nil)
        #expect(withoutLive?.entry == nil)
    }

    private func card(status: String, symbol: String, stop: Double?) -> JournalDeclarationCard {
        JournalDeclarationCard(
            id: "d-\(symbol)-\(status)",
            status: status,
            declarationKind: "intraday",
            symbol: symbol,
            side: "BUY",
            quantity: 50,
            quantityFilled: status == "matched" ? 50 : nil,
            localDate: "2026-09-12",
            protectiveSlConsent: true,
            snapshot: JournalSnapshot(
                setupLabel: "Pullback",
                invalidationLine: "VWAP loss",
                invalidationKind: "behaviour",
                calmScale: 2,
                confidenceScale: 4,
                stopLoss: stop,
                target: 1320
            ),
            notes: JournalNotes(pre: "Declared.", live: "", post: ""),
            fidelity: JournalFidelity(score: nil, dimensions: nil),
            attachments: JournalAttachments(shots: 0, voice: false),
            citedNet: nil,
            citedCurrency: nil
        )
    }
}
