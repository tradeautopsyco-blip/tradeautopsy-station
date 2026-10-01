import Foundation
import Testing
@testable import Notch

struct DeskHotkeyDispatchTests {
    @Test func killHotkeyOpensWarningAndDoesNotLatch() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.daemonConnectionState = .connected
        vm.performDeskHotkey("kill")
        #expect(vm.planKillPhase == .warning)
        #expect(vm.killSwitchActive == false)
    }

    @Test func killHotkeyDoesNothingWhenAgentIsDown() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.daemonConnectionState = .disconnected
        vm.performDeskHotkey("kill")
        #expect(vm.planKillPhase == .idle)
    }

    @Test func confirmHotkeyOnlyRaisesTheSubmitRequest() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.performDeskHotkey("confirm_declare")
        #expect(vm.hotkeyConfirmRequest == 1)
        #expect(vm.barDeclarationLastError == nil)
    }

    @Test func focusHotkeysPublishScreenRawValues() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.performDeskHotkey("focus_open")
        #expect(vm.hotkeyScreenRequest == "Open")
        vm.performDeskHotkey("focus_plan")
        #expect(vm.hotkeyScreenRequest == "Plan")
        vm.performDeskHotkey("focus_working")
        #expect(vm.hotkeyScreenRequest == "Working")
        vm.performDeskHotkey("focus_debrief")
        #expect(vm.hotkeyScreenRequest == "Debrief")
    }

    @Test func cancelWithoutSelectionDoesNotOpenTheReasonStep() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.performDeskHotkey("cancel_selected_declaration")
        #expect(vm.cancelDeclStep == 0)
        #expect(vm.hotkeyScreenRequest == nil)
    }

    @Test func manualFillHotkeyShowsTheLaneRequestOnDebrief() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.performDeskHotkey("open_manual_fill")
        #expect(vm.requestManualFillLane)
        #expect(vm.hotkeyScreenRequest == "Debrief")
        #expect(vm.consumeManualFillRequest())
        #expect(vm.requestManualFillLane == false)
    }

    @Test func calmHotkeyDoesNotDismissBeforeTheCountdownEnds() async {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.killSwitchActive = true
        vm.killSwitchCountdownSecs = 12
        vm.performDeskHotkey("dismiss_kill_overlay")
        try? await Task.sleep(nanoseconds: 50_000_000)
        #expect(vm.killSwitchActive)
    }

    @Test func unknownHotkeyIdIsNoOp() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.performDeskHotkey("not_in_hotkey_actions_md")
        #expect(vm.hotkeyConfirmRequest == 0)
        #expect(vm.hotkeyScreenRequest == nil)
    }

    @Test func prefsListKeepsStableIdsAndPostsOnSave() {
        let ids = DeskHotkeyPreferences.configurableActions.map(\.id)
        #expect(ids.contains("kill"))
        #expect(ids.contains("confirm_declare"))
        #expect(ids.contains("toggle_notch"))
        #expect(ids.contains("dismiss_kill_overlay"))
        let key = DeskHotkeyPreferences.storageKey
        let previous = UserDefaults.standard.data(forKey: key)
        defer { UserDefaults.standard.set(previous, forKey: key) }
        let flag = HotkeySaveFlag()
        let token = NotificationCenter.default.addObserver(
            forName: DeskHotkeyPreferences.didSaveNotification,
            object: nil,
            queue: nil
        ) { _ in
            flag.posted = true
        }
        defer { NotificationCenter.default.removeObserver(token) }
        DeskHotkeyPreferences.save([])
        #expect(flag.posted)
        #expect(DeskHotkeyPreferences.load().isEmpty)
    }
}

private final class HotkeySaveFlag: @unchecked Sendable {
    var posted = false
}
