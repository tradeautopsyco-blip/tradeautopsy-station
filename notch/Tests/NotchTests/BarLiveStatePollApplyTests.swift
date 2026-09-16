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
        #expect(vm.barStateError == nil)
        #expect(vm.barStateRequiresDeviceLogin == false)
    }

    @Test func deviceLoginFailureSetsFlagWithoutClearingLastGoodOrBodyStrip() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.barLiveState = Self.sampleNotch(slug: "kotak_neo")
        vm.applyLiveStatePollFailure(
            message: "Live Plan needs Station sign-in — complete device login in Station Settings.",
            isDeviceLogin: true
        )
        #expect(vm.barLiveState != nil)
        #expect(vm.barStateRequiresDeviceLogin == true)
        #expect(vm.barStateError == nil)
    }

    @Test func successStampsLastFetchedAndClearsDeviceLogin() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        let state = Self.sampleNotch(slug: "kotak_neo")
        vm.barLiveState = state
        vm.barFeaturesActiveFromApi = true
        vm.barProtectiveBrokerSlug = "kotak_neo"
        vm.barStateRequiresDeviceLogin = true
        vm.recomputeBarSurfacePhase()
        let phase = vm.barSurfacePhase
        vm.applyLiveStatePollSuccess(
            BarLiveStateAPIResponse(barFeaturesActive: true, notch: state)
        )
        #expect(vm.barLastFetched != nil)
        #expect(vm.barStateRequiresDeviceLogin == false)
        #expect(vm.barProtectiveBrokerSlug == "kotak_neo")
        #expect(vm.barSurfacePhase == phase)
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
