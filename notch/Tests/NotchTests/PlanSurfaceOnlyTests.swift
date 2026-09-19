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

    // ⌥Space on the Station-hosted Notch is a PLAN expand/collapse, never a visibility toggle:
    // the pill must stay on screen across the whole summon.
    @Test func hostedToggleKeepsPillVisibleInsteadOfHidingTheHud() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()
        #expect(launcher.isPanelVisible)

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded)
        #expect(launcher.isPanelVisible)

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded == false)
        #expect(launcher.isPanelVisible)

        launcher.dismiss()
    }

    // Second ⌥Space mid-flight re-targets instead of leaving the window stuck at the expanded
    // frame — the pending frame snap is cancelled and re-armed, never brick-walled.
    @Test func rapidDoubleToggleSettlesCollapsedWithPanelBackAtPillFrame() async throws {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()

        launcher.toggle()
        launcher.toggle()
        launcher.toggle()
        #expect(launcher.viewModel.isExpanded)
        // Frame is already expanded on the same turn as the keypress — no window size animation.
        #expect(launcher.viewModel.summonPanelAtExpandedFrame)

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded == false)

        try await Task.sleep(nanoseconds: 500_000_000)
        #expect(launcher.viewModel.summonPanelAtExpandedFrame == false)
        #expect(launcher.isPanelVisible)

        launcher.dismiss()
    }

    // Pre-warm: the expanded surface is laid out at final size while the window is still the
    // pill, so the first summon composites an already-built tree. The pre-warm size must never
    // leak into the collapsed window — that squashed the pill into a visibleFrame-sized layout.
    @Test func startPrewarmsExpandedSurfaceSizeWithoutGrowingTheCollapsedPill() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()

        let prewarmed = launcher.viewModel.expandedSurfaceSize
        #expect(launcher.viewModel.isExpanded == false)
        #expect(prewarmed.width > 0)
        #expect(prewarmed.height > 0)

        let collapsedFrame = launcher.panelFrame
        #expect(collapsedFrame.width < prewarmed.width)
        #expect(collapsedFrame.height < prewarmed.height)

        launcher.dismiss()
    }

    // The expanded window and the pre-warmed surface must be the same size: the surface is an
    // overlay laid out at `expandedSurfaceSize`, so any divergence misaligns it in the window.
    @Test func expandedPanelFrameMatchesPrewarmedSurfaceSize() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded)
        #expect(launcher.panelFrame.size == launcher.viewModel.expandedSurfaceSize)

        launcher.dismiss()
    }

    @Test func restoreChromeAfterCaptureKeepsExpandedFrameMatchedToSurface() {
        let launcher = NotchLauncher(isHostedByStation: true)
        launcher.configure(secret: "test-secret", port: 9137, webBase: "http://127.0.0.1:9137")
        launcher.start()

        launcher.toggle()
        #expect(launcher.viewModel.isExpanded)

        launcher.hideChromeForInteractiveCapture()
        launcher.restoreChromeAfterInteractiveCapture()

        #expect(launcher.viewModel.isExpanded)
        #expect(launcher.panelFrame.size == launcher.viewModel.expandedSurfaceSize)
        #expect(launcher.panelFrame.width > 0)
        #expect(launcher.panelFrame.height > 0)

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
