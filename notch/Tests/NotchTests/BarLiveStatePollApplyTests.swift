import Combine
import Foundation
import Testing
@testable import Notch

@MainActor
struct BarLiveStatePollApplyTests {
    @Test func failureDoesNotNilLastGoodLiveState() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        let lastGood = Self.sampleNotch(slug: "kotak_neo")
        vm.barLiveState = lastGood
        vm.applyLiveStatePollFailure(message: "Live state unreachable (502)", isDeviceLogin: false)
        #expect(vm.barLiveState == lastGood)
        #expect(vm.barStateError == nil)
        vm.applyLiveStatePollFailure(message: "Live state unreachable (502)", isDeviceLogin: false)
        #expect(vm.barLiveState == lastGood)
        #expect(vm.barStateError == "Live state unreachable (502)")
    }

    @Test func deviceLoginFailurePaintsStripWithoutClearingLastGood() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.barLiveState = Self.sampleNotch(slug: "kotak_neo")
        vm.applyLiveStatePollFailure(
            message: "Live Plan needs Station sign-in — complete device login in Station Settings.",
            isDeviceLogin: true
        )
        #expect(vm.barLiveState != nil)
        #expect(vm.barStateRequiresDeviceLogin == true)
        #expect(vm.barStateError?.localizedCaseInsensitiveContains("device login") == true)
    }

    @Test func equalProtectiveSlugAndPhaseDoNotFireObjectWillChange() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        let state = Self.sampleNotch(slug: "kotak_neo")
        vm.barLiveState = state
        vm.barFeaturesActiveFromApi = true
        vm.barProtectiveBrokerSlug = "kotak_neo"
        vm.recomputeBarSurfacePhase()
        var fires = 0
        let sub = vm.objectWillChange.sink { _ in fires += 1 }
        vm.applyLiveStatePollSuccess(
            BarLiveStateAPIResponse(barFeaturesActive: true, notch: state)
        )
        #expect(fires == 0)
        _ = sub
    }

    private static func sampleNotch(slug: String) -> BarLiveStateResponse {
        BarLiveStateResponse(
            planState: "GREEN",
            primarySentence: nil,
            triggerType: nil,
            isRedTerminal: false,
            composite: nil,
            activeInterventions: [],
            syncState: "GREEN",
            lastSyncAt: nil,
            declarationSubmitBlocked: nil,
            pendingDeclaration: nil,
            protectiveBrokerSlug: slug
        )
    }
}
