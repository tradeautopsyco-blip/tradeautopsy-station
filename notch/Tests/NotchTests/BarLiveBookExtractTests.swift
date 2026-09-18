import Foundation
import Testing
@testable import Notch

@MainActor
struct BarLiveBookExtractTests {
    @Test func pendingDeclarationArmsFromBookNotTimeout() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        let pending = BarPendingDeclaration(
            id: "00000000-0000-4000-8000-000000000099",
            status: "PENDING",
            createdAt: nil,
            symbol: "RELIANCE",
            side: "BUY",
            quantity: 10,
            declarationKind: "intraday",
            protectiveSlConsent: true,
            stopLoss: 1400,
            target: nil,
            planSnapshot: nil
        )
        vm.barLiveState = BarLiveStateResponse(
            planState: "",
            primarySentence: nil,
            triggerType: nil,
            isRedTerminal: false,
            composite: nil,
            activeInterventions: [],
            syncState: "GREEN",
            lastSyncAt: nil,
            declarationSubmitBlocked: nil,
            pendingDeclaration: pending
        )
        vm.recomputeBarSurfacePhase()
        #expect(vm.barSurfacePhase == .armed)
        #expect(vm.barOptimisticArmedDisplay == nil)
    }

    @Test func optimisticAgeDoesNotUnarmPolicy() {
        let snap = BarOptimisticArmedSnapshot(
            declarationId: "x",
            submittedAt: Date().timeIntervalSince1970 - 120,
            symbol: "RELIANCE",
            side: "BUY",
            quantityLabel: "1"
        )
        #expect(
            BarOptimisticArmedReconcilePolicy.shouldClearOptimistic(
                snapshot: snap,
                pollFailures: 9
            ) == false
        )
    }
}
