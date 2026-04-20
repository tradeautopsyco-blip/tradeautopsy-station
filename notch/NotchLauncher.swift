import AppKit
import Foundation
import SwiftUI

@MainActor
protocol NotchLauncherHost: AnyObject {
    func expandToPulse() async
    func expandToBrief() async
    func expandToTAI() async
    func expandToPositions() async
}

/// Owns `NSPanel` + view model; driven from Rust via C ABI.
@MainActor
final class NotchLauncher: NSObject, NotchLauncherHost {
    private var panelController: NotchPanelController?
    let viewModel = NotchViewModel()

    func configure(secret: String, port: UInt16, webBase: String) {
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

    func start() {
        viewModel.stopPolling()
        if panelController == nil {
            panelController = NotchPanelController(viewModel: viewModel)
        }
        viewModel.startPolling()
        panelController?.show()
    }

    func dismiss() {
        viewModel.stopPolling()
        panelController?.hide()
        panelController = nil
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
        viewModel.prepareProgrammaticExpansion()
        viewModel.isExpanded = true
        Task {
            try? await Task.sleep(nanoseconds: 30_000_000_000)
            viewModel.isExpanded = false
        }
    }

    func notifyBrokerSession(active: Bool) {
        viewModel.brokerSessionActive = active
    }

    func expandToPulse() async {
        viewModel.prepareProgrammaticExpansion()
        viewModel.activeTab = .pulse
        viewModel.isExpanded = true
    }

    func expandToBrief() async {
        viewModel.prepareProgrammaticExpansion()
        viewModel.activeTab = .brief
        viewModel.isExpanded = true
    }

    func expandToTAI() async {
        viewModel.prepareProgrammaticExpansion()
        viewModel.activeTab = .tai
        viewModel.isExpanded = true
    }

    func expandToPositions() async {
        viewModel.prepareProgrammaticExpansion()
        viewModel.activeTab = .positions
        viewModel.isExpanded = true
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
        let l = NotchLauncher()
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
