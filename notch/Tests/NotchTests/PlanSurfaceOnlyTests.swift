import Foundation
import Testing
@testable import Notch

@MainActor
struct PlanSurfaceOnlyTests {
    @Test func hostedViewModelDefaultsToPlanTab() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        #expect(vm.planSurfaceOnly)
        #expect(vm.activeTab == .plan)
    }

    @Test func planSurfaceOnlyClampsSelectTabAwayFromOtherTabs() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.selectTab(.pulse)
        #expect(vm.activeTab == .plan)
        vm.selectTab(.brief)
        #expect(vm.activeTab == .plan)
        vm.selectTab(.capture)
        #expect(vm.activeTab == .plan)
        #expect(vm.dictationUsesCaptureDraft == false)
    }

    @Test func enablePlanSurfaceOnlyLocksExistingViewModel() {
        let vm = NotchViewModel()
        #expect(vm.activeTab == .pulse)
        vm.enablePlanSurfaceOnly()
        #expect(vm.planSurfaceOnly)
        #expect(vm.activeTab == .plan)
        vm.selectTab(.positions)
        #expect(vm.activeTab == .plan)
    }

    @Test func hostedLauncherStartsOnPlanAndExpandHelpersStayOnPlan() async {
        let launcher = NotchLauncher(isHostedByStation: true)
        #expect(launcher.viewModel.planSurfaceOnly)
        #expect(launcher.viewModel.activeTab == .plan)

        await launcher.expandToPulse()
        #expect(launcher.viewModel.activeTab == .plan)
        #expect(launcher.viewModel.isExpanded)

        await launcher.expandToCapture()
        #expect(launcher.viewModel.activeTab == .plan)

        await launcher.expandToBrief()
        #expect(launcher.viewModel.activeTab == .plan)
    }

    @Test func standaloneLauncherDoesNotInstallPlanSurfaceOnly() {
        let launcher = NotchLauncher(isHostedByStation: false)
        #expect(launcher.viewModel.planSurfaceOnly == false)
        #expect(launcher.viewModel.activeTab == .pulse)
    }

    @Test func hostedLauncherDoesNotInstallToggleHotkeyMonitorsOnStart() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()
        #expect(launcher.hasInstalledToggleHotkeyMonitors == false)
        launcher.dismiss()
    }

    @Test func hostedToggleExpandsThenCollapsesPlanSurface() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()
        #expect(launcher.viewModel.isExpanded == false)

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded == true)
        #expect(launcher.viewModel.activeTab == .plan)

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded == false)

        launcher.dismiss()
    }

    @Test func chromeTapCollapseCollapsesExpandedPlanSurface() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        #expect(vm.isExpanded == false)

        vm.expandFromCollapsedChromeTap()
        #expect(vm.isExpanded == true)

        vm.collapseExpandedFromChromeTap()
        #expect(vm.isExpanded == false)

        // Idempotent when already collapsed
        vm.collapseExpandedFromChromeTap()
        #expect(vm.isExpanded == false)
    }

    @Test func outsideClickCollapseCollapsesExpandedPlanSurface() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.expandFromCollapsedChromeTap()
        #expect(vm.isExpanded == true)

        vm.collapseExpandedFromOutsideClick()
        #expect(vm.isExpanded == false)
    }

    @Test func hostedExpandCollapseDismissSmokeWithOutsideCollapsePath() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded == true)

        launcher.viewModel.collapseExpandedFromOutsideClick()
        #expect(launcher.viewModel.isExpanded == false)

        launcher.dismiss()
    }

    @Test func requestHidePillCollapsesAndInvokesHideCallback() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        var hideCalls = 0
        vm.onRequestHidePill = { hideCalls += 1 }

        vm.expandFromCollapsedChromeTap()
        #expect(vm.isExpanded == true)

        vm.requestHidePill()
        #expect(vm.isExpanded == false)
        #expect(hideCalls == 1)
    }

    @Test func ingestPasteRequestsLiveScreenWhenNotDebrief() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.barSurfacePhase = .livePlan
        vm.ingestImageData(Self.onePxPng, hintedType: "image/png")
        #expect(vm.requestLiveCaptureScreen)
        #expect(vm.consumeLiveCaptureScreenRequest())
        #expect(vm.requestLiveCaptureScreen == false)
    }

    @Test func ingestPasteDoesNotLeaveLiveDuringDebrief() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.barSurfacePhase = .debrief
        vm.ingestImageData(Self.onePxPng, hintedType: "image/png")
        #expect(vm.requestLiveCaptureScreen)
        #expect(vm.consumeLiveCaptureScreenRequest() == false)
        #expect(vm.requestLiveCaptureScreen == false)
    }

    @Test func hostedExpandToCaptureStillClampsToPlanTab() async {
        let launcher = NotchLauncher(isHostedByStation: true)
        await launcher.expandToCapture()
        #expect(launcher.viewModel.activeTab == .plan)
        #expect(launcher.viewModel.planSurfaceOnly)
    }

    @Test func requestLinkUnpostedConfirmsReplaceWhenTradeAlreadyHasChart() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.unpostedCaptures = [
            UnpostedCaptureRecord(
                id: UUID(),
                createdAt: Date(),
                updatedAt: Date(),
                filename: "x.png",
                contentType: "image/png",
                byteSize: 1,
                caption: "",
                capturePhase: "during",
                lastError: nil,
                consolePendingId: nil,
                idempotencyKey: nil
            )
        ]
        vm.tradeIdsWithChart.insert("trade-has-chart")
        vm.requestLinkUnposted(to: "trade-has-chart")
        #expect(vm.replaceConfirmTradeId == "trade-has-chart")
        vm.cancelReplaceChart()
        #expect(vm.replaceConfirmTradeId == nil)
    }

    @Test func requestLinkUnpostedNoopsWithoutUnpostedItems() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.unpostedCaptures = []
        vm.tradeIdsWithChart.insert("trade-has-chart")
        vm.requestLinkUnposted(to: "trade-has-chart")
        #expect(vm.replaceConfirmTradeId == nil)
    }

    private static var onePxPng: Data {
        Data(
            base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
        )!
    }
}
