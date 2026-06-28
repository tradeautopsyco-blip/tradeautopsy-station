import AppKit
import Foundation
import SwiftUI

@MainActor
protocol NotchLauncherHost: AnyObject {
    func expandToPulse() async
    func expandToBrief() async
    func expandToTAI() async
    func expandToPositions() async
    func expandToPlan() async
    func expandToCapture() async
}

/// Owns `NSPanel` + view model; driven from Rust via C ABI.
@MainActor
public final class NotchLauncher: NSObject, NotchLauncherHost {
    private var panelController: NotchPanelController?
    private var killSwitchOverlayController: KillSwitchOverlayController?
    private var globalHotkeyMonitor: Any?
    private var localHotkeyMonitor: Any?
    private var fnGlobalMonitor: Any?
    private var fnLocalMonitor: Any?
    public let isHostedByStation: Bool
    public let viewModel: NotchViewModel

    public init(isHostedByStation: Bool = false, injectedViewModel: NotchViewModel? = nil) {
        self.isHostedByStation = isHostedByStation
        self.viewModel = injectedViewModel ?? NotchViewModel()
        super.init()
    }

    var hasInstalledToggleHotkeyMonitors: Bool {
        globalHotkeyMonitor != nil || localHotkeyMonitor != nil
    }

    public func toggle() {
        panelController?.toggle()
    }

    public func configure(secret: String, port: UInt16, webBase: String) {
        viewModel.daemonSecret = secret
        viewModel.daemonPort = port
        let base = webBase.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        if base.hasPrefix("http://") || base.hasPrefix("https://") {
            viewModel.webBaseURL = base
        } else {
            viewModel.webBaseURL = "https://\(base)"
        }
        viewModel.notchHost = self
    }

    public func start() {
        viewModel.stopPolling()
        if panelController == nil {
            panelController = NotchPanelController(viewModel: viewModel)
        }
        if killSwitchOverlayController == nil {
            killSwitchOverlayController = KillSwitchOverlayController(viewModel: viewModel)
        }
        if !isHostedByStation {
            installHotkeyMonitorsIfNeeded()
        }
        installFnKeyMonitorsIfNeeded()
        viewModel.ensureDictationWired()
        viewModel.startPolling()
        panelController?.show()
    }

    public func dismiss() {
        viewModel.stopPolling()
        if !isHostedByStation {
            uninstallHotkeyMonitors()
        }
        uninstallFnKeyMonitors()
        panelController?.hide()
        panelController = nil
        killSwitchOverlayController = nil
    }

    private func installHotkeyMonitorsIfNeeded() {
        guard globalHotkeyMonitor == nil, localHotkeyMonitor == nil else { return }
        globalHotkeyMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard Self.isToggleShortcut(event) else { return }
            DispatchQueue.main.async {
                self?.panelController?.toggle()
            }
        }
        localHotkeyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard Self.isToggleShortcut(event) else { return event }
            self?.panelController?.toggle()
            return nil
        }
    }

    private func uninstallHotkeyMonitors() {
        if let monitor = globalHotkeyMonitor {
            NSEvent.removeMonitor(monitor)
            globalHotkeyMonitor = nil
        }
        if let monitor = localHotkeyMonitor {
            NSEvent.removeMonitor(monitor)
            localHotkeyMonitor = nil
        }
    }

    private func installFnKeyMonitorsIfNeeded() {
        guard fnGlobalMonitor == nil, fnLocalMonitor == nil else { return }
        fnGlobalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .flagsChanged) { [weak self] e in
            Self.handleFnEvent(e, launcher: self)
        }
        fnLocalMonitor = NSEvent.addLocalMonitorForEvents(matching: .flagsChanged) { [weak self] e in
            Self.handleFnEvent(e, launcher: self)
            return e
        }
    }

    private func uninstallFnKeyMonitors() {
        if let monitor = fnGlobalMonitor {
            NSEvent.removeMonitor(monitor)
            fnGlobalMonitor = nil
        }
        if let monitor = fnLocalMonitor {
            NSEvent.removeMonitor(monitor)
            fnLocalMonitor = nil
        }
    }

    private static func handleFnEvent(_ e: NSEvent, launcher: NotchLauncher?) {
        let mask = e.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let fnDown = mask.contains(.function)
        DispatchQueue.main.async {
            launcher?.viewModel.dictationPushToTalk(
                fnDown: fnDown,
                reduceMotion: NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
            )
        }
    }

    static func isToggleShortcut(_ event: NSEvent) -> Bool {
        let isSpace = event.keyCode == 49
        let optionOnly = event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .option
        return isSpace && optionOnly
    }

    func updateScore(score: Double, state: String) {
        let old = viewModel.compositeScore
        viewModel.compositeScore = score
        viewModel.behavioralState = state
        viewModel.checkSmartTriggers(oldScore: old, newScore: score)
    }

    func notifyKillSwitch(_ active: Bool) {
        viewModel.killSwitchActive = active
        if active {
            viewModel.pulseAttention = .red
            NotchHaptics.play(.heavy)
            Task { await expandToPositions() }
        }
    }

    func notifyMarketOpen() {
        Task {
            await viewModel.fetchMorningBrief()
            await expandToBrief()
            viewModel.pulseAttention = .teal
        }
    }

    func notifyMarketClose() {
        let msg = viewModel.sessionSummaryForClose()
        viewModel.lastAlert = msg
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
        Task {
            try? await Task.sleep(nanoseconds: 30_000_000_000)
            viewModel.isExpanded = false
        }
    }

    func notifyBrokerSession(active: Bool) {
        viewModel.brokerSessionActive = active
    }

    func expandToPulse() async {
        viewModel.selectTab(.pulse)
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
    }

    func expandToBrief() async {
        viewModel.selectTab(.brief)
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
    }

    func expandToTAI() async {
        viewModel.selectTab(.tai)
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
    }

    func expandToPositions() async {
        viewModel.selectTab(.positions)
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
    }

    func expandToPlan() async {
        viewModel.selectTab(.plan)
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
    }

    func expandToCapture() async {
        viewModel.selectTab(.capture)
        viewModel.isExpanded = true
        viewModel.onRequestOrderFront?()
    }
}

// MARK: - Global handle for FFI (must stay alive)

private final class NotchGlobal {
    static let shared = NotchGlobal()
    var launcher: NotchLauncher?
}

extension NotchLauncher {
    static func sharedLauncher() -> NotchLauncher {
        if let l = NotchGlobal.shared.launcher { return l }
        let l = NotchLauncher(isHostedByStation: false)
        NotchGlobal.shared.launcher = l
        return l
    }
}

// MARK: - C ABI (Rust `extern "C"`)

@_cdecl("tradeautopsy_notch_launch")
public func tradeautopsy_notch_launch(
    _ secret: UnsafePointer<CChar>?,
    _ port: UInt16,
    _ apiBase: UnsafePointer<CChar>?
) {
    let s = secret.map { String(cString: $0) } ?? ""
    let base = apiBase.map { String(cString: $0) } ?? "https://localhost:3000"
    DispatchQueue.main.async {
        let l = NotchLauncher.sharedLauncher()
        l.configure(secret: s, port: port, webBase: base)
        l.start()
    }
}

@_cdecl("tradeautopsy_notch_dismiss")
public func tradeautopsy_notch_dismiss() {
    DispatchQueue.main.async {
        NotchGlobal.shared.launcher?.dismiss()
    }
}

@_cdecl("tradeautopsy_notch_update_score")
public func tradeautopsy_notch_update_score(_ score: Double, _ state: UnsafePointer<CChar>?) {
    let st = state.map { String(cString: $0) } ?? "CALM"
    DispatchQueue.main.async {
        NotchGlobal.shared.launcher?.updateScore(score: score, state: st)
    }
}

@_cdecl("tradeautopsy_notch_kill_switch")
public func tradeautopsy_notch_kill_switch(_ active: Bool) {
    DispatchQueue.main.async {
        NotchGlobal.shared.launcher?.notifyKillSwitch(active)
    }
}

@_cdecl("tradeautopsy_notch_market_open")
public func tradeautopsy_notch_market_open() {
    DispatchQueue.main.async {
        NotchGlobal.shared.launcher?.notifyMarketOpen()
    }
}

@_cdecl("tradeautopsy_notch_market_close")
public func tradeautopsy_notch_market_close() {
    DispatchQueue.main.async {
        NotchGlobal.shared.launcher?.notifyMarketClose()
    }
}

@_cdecl("tradeautopsy_notch_broker_session")
public func tradeautopsy_notch_broker_session(_ active: Bool) {
    DispatchQueue.main.async {
        NotchGlobal.shared.launcher?.notifyBrokerSession(active: active)
    }
}
