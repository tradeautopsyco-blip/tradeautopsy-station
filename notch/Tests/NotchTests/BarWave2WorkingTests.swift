import Foundation
import Testing
@testable import Notch

struct BarWave2WorkingTests {
    @Test func workingPlanLevelsFromPendingSnapshot() {
        let pending = BarPendingDeclaration(
            id: "d1",
            status: "PENDING",
            createdAt: nil,
            symbol: "BTCUSDT",
            side: "BUY",
            quantity: 0.01,
            declarationKind: "intraday",
            protectiveSlConsent: true,
            stopLoss: 90_000,
            target: 100_000,
            planSnapshot: BarPlanSnapshotSummary(
                setupLabel: "harness",
                invalidationLine: nil,
                calmScale: nil,
                confidenceScale: nil,
                frustrationScale: nil,
                excitementScale: nil,
                stance: "planned",
                intent: nil,
                invalidationKind: nil,
                invalidationPrice: nil,
                product: nil,
                targetPrice: nil,
                symbol: "BTCUSDT",
                side: "BUY",
                quantity: 0.01,
                stopLoss: nil,
                bookId: "binance-com-spot",
                entryPrice: 95_000,
                emotionIn: nil,
                invalidation: nil,
            ),
            filledQty: nil,
            avgFill: nil,
            fillSymbol: nil,
            fillSide: nil,
        )
        let levels = BarWorkingLivePresentation.planLevels(pending: pending)
        #expect(levels?.entry == 95_000)
        #expect(levels?.stop == 90_000)
        #expect(levels?.target == 100_000)
        #expect(levels?.sideBuy == true)
    }

    @Test func marginHonestWhenBrokerNotConnected() {
        let m = BarWorkingLivePresentation.marginDisplay(
            brokerSyncClass: "not_connected",
            barSyncState: "NOT_CONNECTED",
            fundsStatus: "success",
            freeText: "USDT 999",
        )
        #expect(m.value == "—")
        #expect(m.subtitle == "Broker not synced")
    }

    @Test func unrealizedSubtitlePendingNoFill() {
        let sub = BarWorkingLivePresentation.unrealizedSubtitle(
            pending: BarPendingDeclaration(
                id: "x",
                status: "PENDING",
                createdAt: nil,
                symbol: "X",
                side: "BUY",
                quantity: 1,
                declarationKind: "intraday",
                protectiveSlConsent: false,
                stopLoss: nil,
                target: nil,
                planSnapshot: nil,
                filledQty: nil,
                avgFill: nil,
                fillSymbol: nil,
                fillSide: nil,
            ),
            hasOpenFill: false,
        )
        #expect(sub == "Pending — no fill yet")
    }

    @Test func unboundLastIsDarkNotBreached() {
        #expect(
            BarWorkingCompare.vsInvalidation(
                sideBuy: true,
                last: 99,
                status: "unavailable",
                kind: "price",
                price: 100
            ) == .dark
        )
        #expect(
            !BarWorkingCompare.isPriceInvalidated(
                sideBuy: true,
                last: 99,
                status: "unavailable",
                kind: "price",
                price: 100
            )
        )
        #expect(BarWorkingCompare.labeledLast(last: 99, status: "unavailable") == "— · unavailable")
    }

    @Test func buyLastAtOrBelowInvalidationIsBreached() {
        #expect(
            BarWorkingCompare.isPriceInvalidated(
                sideBuy: true,
                last: 100,
                status: "fresh",
                kind: "price",
                price: 100
            )
        )
        #expect(
            BarWorkingCompare.vsInvalidation(
                sideBuy: true,
                last: 101,
                status: "fresh",
                kind: "price",
                price: 100
            ) == .intact
        )
    }

    @Test func sellLastAtOrAboveInvalidationIsBreached() {
        #expect(
            BarWorkingCompare.isPriceInvalidated(
                sideBuy: false,
                last: 100,
                status: "fresh",
                kind: "price",
                price: 100
            )
        )
        #expect(
            !BarWorkingCompare.isPriceInvalidated(
                sideBuy: false,
                last: 99,
                status: "fresh",
                kind: "price",
                price: 100
            )
        )
    }

    @Test func timeBehaviourContextStayWaiting() {
        for kind in ["time", "behaviour", "context"] {
            #expect(
                BarWorkingCompare.vsInvalidation(
                    sideBuy: true,
                    last: 50,
                    status: "fresh",
                    kind: kind,
                    price: 100
                ) == .waiting
            )
        }
    }

    @Test func pendingActualUsesFillAndLabeledLast() {
        let pending = BarPendingDeclaration(
            id: "d1",
            status: "PENDING",
            createdAt: nil,
            symbol: "RELIANCE",
            side: "BUY",
            quantity: 2,
            declarationKind: "intraday",
            protectiveSlConsent: false,
            stopLoss: 1400,
            target: 1500,
            planSnapshot: BarPlanSnapshotSummary(
                setupLabel: "ORB",
                invalidationLine: nil,
                calmScale: 4,
                confidenceScale: 4,
                frustrationScale: 2,
                excitementScale: 2,
                stance: "patient",
                intent: "hold the open",
                invalidationKind: "price",
                invalidationPrice: 1410,
                product: "MIS",
                targetPrice: 1500,
                symbol: "RELIANCE",
                side: "BUY",
                quantity: 2,
                stopLoss: 1400,
                bookId: nil,
                entryPrice: nil,
                emotionIn: nil,
                invalidation: nil
            ),
            filledQty: 2,
            avgFill: 1420,
            fillSymbol: "RELIANCE",
            fillSide: "BUY"
        )
        let rows = BarEscrowMatchPresentation.matchRows(
            from: nil,
            pending: pending,
            last: 1405,
            lastStatus: "fresh"
        )
        #expect(rows.contains(where: { $0.label == "Avg fill" && $0.actual == "1420" }))
        #expect(rows.contains(where: { $0.label == "Quantity" && $0.actual == "2" }))
        #expect(rows.contains(where: { $0.label == "Invalidation" && $0.actual.contains("fresh") }))
        #expect(rows.contains(where: { $0.label == "Invalidation" && $0.tone == .red }))
        #expect(
            BarWorkingCompare.isPriceInvalidated(
                sideBuy: true,
                last: 1405,
                status: "fresh",
                kind: "price",
                price: 1410
            )
        )
    }
}
