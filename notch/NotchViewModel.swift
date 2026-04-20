import AppKit
import Combine
import Foundation
import SwiftUI

// MARK: - Edge / caution rows (brief right zone)

struct EdgeSymbolRow: Identifiable {
    var id: String { symbol }
    var symbol: String
    /// Signed percent change for display (0 if unknown).
    var changePct: Double
}

struct CautionSymbolRow: Identifiable, Equatable {
    var id: String { symbol + reason }
    var symbol: String
    var reason: String
}

struct SignalBreakdown {
    var lossChasingScore: Double = 0
    var revengeScore: Double = 0
    var overtradingScore: Double = 0
    var sizingErrorScore: Double = 0
    var symbolDriftScore: Double = 0
}

struct SignalRow: Identifiable {
    var id: String { name }
    var name: String
    var value: Double
}

struct NotchPosition: Identifiable {
    var id: String { symbol + "\(qty)" }
    var symbol: String
    var qty: Int
    var unrealizedPnL: Double
    var direction: String
}

struct MorningBrief: Equatable {
    var summary: String
    var niftyFutures: Double
    var bankniftyFutures: Double
    var niftyChangePct: Double
    var bankniftyChangePct: Double
    var vix: Double
    var edgeSymbols: [String]
    var cautionSymbols: [CautionSymbolRow]
    var recommendation: String
    var fetchedAt: Date
}

struct WorkflowStatus: Identifiable {
    var id: String
    var name: String
    var nextRunMinutes: Int
    var isRunning: Bool
}

struct QuickWorkflowItem: Identifiable {
    var id: String { workflowId }
    var workflowId: String
    var name: String
    var isRunning: Bool
}

struct WorkflowRun: Identifiable {
    var id: String { name + "\(minutesAgo)" + result }
    var name: String
    var minutesAgo: Int
    var result: String
    var success: Bool
}

struct TAIMessage: Identifiable {
    var id = UUID()
    var role: String
    var content: String
    var timestamp: Date
}

enum NotchTab: String, CaseIterable, Hashable {
    case pulse = "PULSE"
    case brief = "BRIEF"
    case workflows = "WORKFLOWS"
    case tai = "TAI"
    case positions = "POSITIONS"
}

enum TaiMode: String, CaseIterable, Hashable {
    case ambient = "AMBIENT"
    case coaching = "COACHING"
    case focus = "FOCUS"
    case silent = "SILENT"
}

enum PulseAttention {
    case none
    case teal
    case amber
    case red
}

@MainActor
final class NotchViewModel: ObservableObject {
    @Published var compositeScore: Double = 0
    @Published var behavioralState: String = "CALM"
    @Published var sessionPnL: Double = 0
    @Published var winRate: Double = 0
    @Published var tradesToday: Int = 0
    @Published var signals = SignalBreakdown()
    @Published var positions: [NotchPosition] = []
    @Published var openOrders: Int = 0
    @Published var killSwitchActive: Bool = false
    @Published var morningBrief: MorningBrief?
    @Published var activeWorkflows: [WorkflowStatus] = []
    @Published var recentWorkflowRuns: [WorkflowRun] = []
    @Published var taiMessages: [TAIMessage] = []
    @Published var taiInput: String = ""
    @Published var taiMode: TaiMode = .ambient
    @Published var isLoading: Bool = false
    @Published var activeTab: NotchTab = .pulse
    @Published var lastAlert: String?
    @Published var pulseAttention: PulseAttention = .none
    @Published var brokerSessionActive: Bool = false
    /// Hover-driven + programmatic expansion (BoringNotch-style).
    @Published var isExpanded: Bool = false
    /// After FFI/programmatic expand, ignore hover-leave collapse until the cursor enters the panel again.
    var hoverCollapseEnabled: Bool = true
    /// One-shot ring scale pulse after smart trigger (0.3s).
    @Published var scoreRingPulseScale: CGFloat = 1.0
    /// Full-panel tint pulse (smart triggers).
    @Published var backgroundPulseColor: Color = .clear
    @Published var backgroundPulseOpacity: Double = 0

    /// Called from `NotchPanelController` to front the panel on hover-expand.
    var onRequestOrderFront: (() -> Void)?

    /// Mirrors `NSScreen.safeAreaInsets.top` for layout (notch camera strip).
    @Published var notchTopInset: CGFloat = 0

    /// `true` when the built-in display reports a top safe-area inset (physical notch / housing).
    var hasPhysicalNotch: Bool { notchTopInset > 0 }

    /// Collapsed score dot — breathe when elevated risk.
    var shouldPulse: Bool { compositeScore > 0.25 }

    var formattedSessionPnL: String {
        let raw = formatINR(sessionPnL)
        if sessionPnL >= 0 { return "+\(raw)" }
        return raw
    }

    var winRateFormatted: String {
        String(format: "%.0f%%", winRate * 100)
    }

    var signalRows: [SignalRow] {
        [
            SignalRow(name: "Loss chase", value: signals.lossChasingScore),
            SignalRow(name: "Revenge", value: signals.revengeScore),
            SignalRow(name: "Overtrade", value: signals.overtradingScore),
            SignalRow(name: "Sizing", value: signals.sizingErrorScore),
            SignalRow(name: "Drift", value: signals.symbolDriftScore),
        ]
    }

    var niftyValue: Double { morningBrief?.niftyFutures ?? 0 }
    var bnfValue: Double { morningBrief?.bankniftyFutures ?? 0 }
    var vixValue: Double { morningBrief?.vix ?? 0 }
    var niftyChange: Double? { morningBrief.map(\.niftyChangePct) }
    var bnfChange: Double? { morningBrief.map(\.bankniftyChangePct) }

    var edgeSymbols: [String] {
        morningBrief?.edgeSymbols ?? []
    }

    var totalUnrealizedPnL: Double { unrealizedTotal }

    var totalExposure: Double { totalExposureApprox }

    var quickWorkflows: [QuickWorkflowItem] { quickWorkflowItems }

    var daemonSecret: String = ""
    var daemonPort: UInt16 = 9137
    var webBaseURL: String = "https://localhost:3000"

    private var pollFast: Timer?
    private var pollWorkflows: Timer?
    private var pollBrief: Timer?
    private var lastBriefFetchDay: String?
    private var expandHoverTimer: Timer?
    private var collapseHoverTimer: Timer?

    weak var notchHost: NotchLauncherHost?

    func startPolling() {
        stopPolling()
        pollFast = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                if self.isIstMarketSession() {
                    await self.fetchPulseData()
                    await self.fetchPositions()
                }
            }
        }

        pollWorkflows = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { [weak self] _ in
            Task { @MainActor in
                await self?.fetchWorkflowStatus()
            }
        }

        pollBrief = Timer.scheduledTimer(withTimeInterval: 300, repeats: true) { [weak self] _ in
            Task { @MainActor in
                await self?.fetchMorningBriefIfNeeded()
            }
        }

        Task {
            await fetchWorkflowStatus()
            await fetchMorningBriefIfNeeded()
            if isIstMarketSession() {
                await fetchPulseData()
                await fetchPositions()
            }
        }
    }

    func stopPolling() {
        pollFast?.invalidate()
        pollFast = nil
        pollWorkflows?.invalidate()
        pollWorkflows = nil
        pollBrief?.invalidate()
        pollBrief = nil
        stopHoverTimers()
    }

    private func stopHoverTimers() {
        expandHoverTimer?.invalidate()
        expandHoverTimer = nil
        collapseHoverTimer?.invalidate()
        collapseHoverTimer = nil
    }

    /// Next cursor enter re-enables hover-leave collapse after programmatic open.
    func prepareProgrammaticExpansion() {
        hoverCollapseEnabled = false
    }

    func handleHover(_ hovering: Bool) {
        if hovering {
            collapseHoverTimer?.invalidate()
            collapseHoverTimer = nil
            hoverCollapseEnabled = true
            expandHoverTimer?.invalidate()
            let t = Timer(timeInterval: 0.3, repeats: false) { [weak self] _ in
                guard let self else { return }
                Task { @MainActor in
                    withAnimation(NotchTheme.springExpand) {
                        self.isExpanded = true
                    }
                    NSApp.activate(ignoringOtherApps: true)
                    self.onRequestOrderFront?()
                }
            }
            RunLoop.main.add(t, forMode: .common)
            expandHoverTimer = t
        } else {
            expandHoverTimer?.invalidate()
            expandHoverTimer = nil
            guard hoverCollapseEnabled else { return }
            collapseHoverTimer?.invalidate()
            let t = Timer(timeInterval: 0.2, repeats: false) { [weak self] _ in
                guard let self else { return }
                Task { @MainActor in
                    withAnimation(NotchTheme.springExpand) {
                        self.isExpanded = false
                    }
                }
            }
            RunLoop.main.add(t, forMode: .common)
            collapseHoverTimer = t
        }
    }

    func pulseBackground(color: Color, duration: TimeInterval) {
        backgroundPulseColor = color
        withAnimation(.easeIn(duration: 0.1)) {
            backgroundPulseOpacity = 0.15
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + duration) { [weak self] in
            guard let self else { return }
            withAnimation(.easeOut(duration: 0.3)) {
                self.backgroundPulseOpacity = 0
            }
        }
    }

    private func istCalendar() -> Calendar {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = TimeZone(identifier: "Asia/Kolkata") ?? .current
        return c
    }

    /// NSE cash-style window 09:15–15:30 IST (inclusive end minute).
    func isIstMarketSession() -> Bool {
        let c = istCalendar()
        let now = Date()
        let h = c.component(.hour, from: now)
        let m = c.component(.minute, from: now)
        let wd = c.component(.weekday, from: now)
        if wd == 1 || wd == 7 { return false }
        let mins = h * 60 + m
        return mins >= (9 * 60 + 15) && mins <= (15 * 60 + 30)
    }

    private func todayKeyIST() -> String {
        let f = DateFormatter()
        f.calendar = istCalendar()
        f.timeZone = istCalendar().timeZone
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: Date())
    }

    private func baseURL() -> String {
        "http://127.0.0.1:\(daemonPort)"
    }

    private func authorizedRequest(url: URL, method: String = "GET", body: Data? = nil) -> URLRequest {
        var r = URLRequest(url: url)
        r.httpMethod = method
        r.setValue(daemonSecret, forHTTPHeaderField: "x-daemon-secret")
        r.setValue("notch", forHTTPHeaderField: "x-daemon-source")
        if body != nil {
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
            r.httpBody = body
        }
        return r
    }

    func fetchPulseData() async {
        guard let url = URL(string: baseURL() + "/api/daemon/pulse") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            let old = compositeScore
            if let c = j["composite_score"] as? Double { compositeScore = c }
            if let s = j["behavioral_state"] as? String { behavioralState = s }
            if let p = j["session_pnl"] as? Double { sessionPnL = p }
            if let w = j["win_rate"] as? Double { winRate = w }
            if let t = j["trades_today"] as? Int {
                let prev = tradesToday
                if t > prev {
                    fireTradeFillRingPulse()
                }
                tradesToday = t
            }
            if let sig = j["signals"] as? [String: Any] {
                signals = SignalBreakdown(
                    lossChasingScore: sig["loss_chasing"] as? Double ?? 0,
                    revengeScore: sig["revenge"] as? Double ?? 0,
                    overtradingScore: sig["overtrading"] as? Double ?? 0,
                    sizingErrorScore: sig["sizing_error"] as? Double ?? 0,
                    symbolDriftScore: sig["symbol_drift"] as? Double ?? 0
                )
            }
            checkSmartTriggers(oldScore: old, newScore: compositeScore)
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    func fetchPositions() async {
        guard let url = URL(string: baseURL() + "/api/daemon/positions") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            let oldKs = killSwitchActive
            if let k = j["kill_switch_active"] as? Bool { killSwitchActive = k }
            if let o = j["open_orders"] as? Int { openOrders = o }
            var out: [NotchPosition] = []
            if let arr = j["positions"] as? [[String: Any]] {
                for p in arr {
                    let sym = (p["symbol"] as? String) ?? (p["tradingSymbol"] as? String) ?? "—"
                    let qty = (p["quantity"] as? Int) ?? (p["qty"] as? Int) ?? 0
                    let pnl = (p["unrealizedPnl"] as? Double) ?? (p["unrealized_pnl"] as? Double) ?? 0
                    let dir = (p["direction"] as? String) ?? (p["side"] as? String) ?? ""
                    out.append(NotchPosition(symbol: sym, qty: qty, unrealizedPnL: pnl, direction: dir))
                }
            }
            positions = out
            if killSwitchActive != oldKs {
                checkKillSwitchTransition(from: oldKs, to: killSwitchActive)
            }
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    func fetchWorkflowStatus() async {
        guard let url = URL(string: baseURL() + "/api/daemon/workflows/status") else { return }
        let req = authorizedRequest(url: url, method: "POST", body: Data("{}".utf8))
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            if let active = j["activeWorkflows"] as? [[String: Any]] {
                activeWorkflows = active.compactMap { row in
                    guard let id = row["id"] as? String, let name = row["name"] as? String else { return nil }
                    let next = row["nextRunMinutes"] as? Int ?? 0
                    let run = row["isRunning"] as? Bool ?? false
                    return WorkflowStatus(id: id, name: name, nextRunMinutes: next, isRunning: run)
                }
            }
            if let recent = j["recentWorkflowRuns"] as? [[String: Any]] {
                recentWorkflowRuns = recent.compactMap { row in
                    guard let name = row["name"] as? String else { return nil }
                    let ago = row["minutesAgo"] as? Int ?? 0
                    let res = row["result"] as? String ?? ""
                    let ok = row["success"] as? Bool ?? !res.lowercased().contains("fail")
                    return WorkflowRun(name: name, minutesAgo: ago, result: res, success: ok)
                }
            }
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    func fetchMorningBriefIfNeeded() async {
        let day = todayKeyIST()
        if lastBriefFetchDay == day, morningBrief != nil { return }
        await fetchMorningBrief()
    }

    func fetchMorningBrief() async {
        guard let url = URL(string: baseURL() + "/api/daemon/morning-brief") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            let summary = (j["summary"] as? String) ?? (j["briefing"] as? String) ?? ""
            let nifty = (j["niftyFutures"] as? Double) ?? (j["nifty_futures"] as? Double) ?? 0
            let bn = (j["bankniftyFutures"] as? Double) ?? (j["banknifty_futures"] as? Double) ?? 0
            let niftyCh = (j["niftyChangePct"] as? Double) ?? (j["nifty_change_pct"] as? Double) ?? 0
            let bnCh = (j["bankniftyChangePct"] as? Double) ?? (j["banknifty_change_pct"] as? Double) ?? 0
            let vix = (j["vix"] as? Double) ?? 0
            let edges = (j["edgeSymbols"] as? [String]) ?? (j["edge_symbols"] as? [String]) ?? []
            var cautions: [CautionSymbolRow] = []
            if let arr = j["cautionSymbols"] as? [[String: Any]] {
                cautions = arr.compactMap { row in
                    guard let sym = row["symbol"] as? String else { return nil }
                    let reason = row["reason"] as? String ?? row["note"] as? String ?? ""
                    return CautionSymbolRow(symbol: sym, reason: reason)
                }
            }
            let rec = (j["recommendation"] as? String) ?? ""
            morningBrief = MorningBrief(
                summary: summary,
                niftyFutures: nifty,
                bankniftyFutures: bn,
                niftyChangePct: niftyCh,
                bankniftyChangePct: bnCh,
                vix: vix,
                edgeSymbols: edges,
                cautionSymbols: cautions,
                recommendation: rec,
                fetchedAt: Date()
            )
            lastBriefFetchDay = todayKeyIST()
            let hour = istCalendar().component(.hour, from: Date())
            let minute = istCalendar().component(.minute, from: Date())
            if hour == 8 && minute == 30 {
                await expandBriefTeal()
            }
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    func sendTAIMessage(_ text: String) async {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        taiMessages.append(TAIMessage(role: "user", content: trimmed, timestamp: Date()))
        taiInput = ""
        isLoading = true
        defer { isLoading = false }
        guard let url = URL(string: baseURL() + "/api/daemon/tai-chat") else { return }
        let ctx: [String: Any] = [
            "composite_score": compositeScore,
            "behavioral_state": behavioralState,
            "session_pnl": sessionPnL,
            "tai_mode": taiMode.rawValue,
        ]
        let body: [String: Any] = ["message": trimmed, "context": ctx]
        let bodyData = try? JSONSerialization.data(withJSONObject: body)
        let req = authorizedRequest(url: url, method: "POST", body: bodyData)
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            let content = (j["content"] as? String) ?? (j["message"] as? String) ?? ""
            let msg = TAIMessage(role: "assistant", content: content, timestamp: Date())
            taiMessages.append(msg)
            await expandTAIWithHaptic()
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    func triggerWorkflow(workflowId: String) async {
        guard let url = URL(string: baseURL() + "/api/daemon/workflows/trigger") else { return }
        let body = try? JSONSerialization.data(withJSONObject: ["workflow_id": workflowId])
        let req = authorizedRequest(url: url, method: "POST", body: body)
        _ = try? await URLSession.shared.data(for: req)
    }

    func activateKillSwitch() async {
        guard let url = URL(string: baseURL() + "/api/daemon/kill-switch") else { return }
        let body = try? JSONSerialization.data(withJSONObject: ["level": 3, "reason": "notch_manual"])
        let req = authorizedRequest(url: url, method: "POST", body: body)
        _ = try? await URLSession.shared.data(for: req)
    }

    func exitAllPositions() async {
        guard let url = URL(string: baseURL() + "/api/daemon/cancel-all") else { return }
        let req = authorizedRequest(url: url, method: "POST", body: Data("{}".utf8))
        _ = try? await URLSession.shared.data(for: req)
    }

    /// Same daemon route until a dedicated orders-only endpoint exists.
    func cancelAllOrders() async {
        await exitAllPositions()
    }

    func checkSmartTriggers(oldScore: Double, newScore: Double) {
        if newScore > 0.30, oldScore <= 0.30 {
            pulseAttention = .red
            NotchHaptics.play(.heavy)
            pulseBackground(color: Color.taDanger, duration: 0.4)
            fireSmartTriggerPulse()
            Task { await notchHost?.expandToPulse() }
        } else if newScore > 0.25, oldScore <= 0.25 {
            pulseAttention = .amber
            NotchHaptics.play(.medium)
            pulseBackground(color: Color.taWarning, duration: 0.3)
            fireSmartTriggerPulse()
            Task { await notchHost?.expandToPulse() }
        }
    }

    func checkKillSwitchTransition(from old: Bool, to new: Bool) {
        if new, !old {
            pulseAttention = .red
            NotchHaptics.play(.heavy)
            pulseBackground(color: Color.taDanger, duration: 0.6)
            fireSmartTriggerPulse()
            Task { await notchHost?.expandToPositions() }
        }
    }

    func expandBriefTeal() async {
        pulseAttention = .teal
        NotchHaptics.play(.light)
        pulseBackground(color: Color.taAccent, duration: 0.25)
        await notchHost?.expandToBrief()
    }

    func expandTAIWithHaptic() async {
        pulseAttention = .teal
        NotchHaptics.play(.light)
        pulseBackground(color: Color.taAccent, duration: 0.2)
        await notchHost?.expandToTAI()
    }

    func scoreColor() -> Color {
        NotchTheme.scoreColor(compositeScore)
    }

    func openDeepLink(_ urlString: String) {
        if let u = URL(string: urlString), NSWorkspace.shared.open(u) {
            return
        }
        if urlString.hasPrefix("tradeautopsy://"),
           let fallback = URL(string: webBaseURL + "/dashboard") {
            NSWorkspace.shared.open(fallback)
        }
    }

    func sessionSummaryForClose() -> String {
        let wr = String(format: "%.0f%%", winRate * 100)
        return "Session · P&L \(formatINR(sessionPnL)) · \(tradesToday) trades · WR \(wr)"
    }

    /// Collapsed / center strip label: FLOW · CALM · TILTED · DANGER
    var displayBehavioralState: String {
        let raw = behavioralState.uppercased()
        if raw.contains("TILT") || raw.contains("REVENGE") { return "TILTED" }
        if compositeScore > 0.30 { return "DANGER" }
        if compositeScore < 0.20 { return "FLOW" }
        return "CALM"
    }

    /// Rough exposure placeholder when LTP not available: sum |qty| × ₹1k (tunable when API adds notional).
    var totalExposureApprox: Double {
        positions.reduce(0) { $0 + Double(abs($1.qty)) * 1000 }
    }

    var unrealizedTotal: Double {
        positions.map(\.unrealizedPnL).reduce(0, +)
    }

    var edgeSymbolRows: [EdgeSymbolRow] {
        guard let b = morningBrief else { return [] }
        return b.edgeSymbols.map { EdgeSymbolRow(symbol: $0, changePct: 0) }
    }

    var cautionRows: [CautionSymbolRow] {
        morningBrief?.cautionSymbols ?? []
    }

    /// Session P&L delta vs previous fetch not tracked server-side — show neutral until wired.
    var sessionPnLChangeHint: Double { 0 }

    func briefTimeString() -> String {
        guard let b = morningBrief else { return "—" }
        let f = DateFormatter()
        f.timeStyle = .short
        f.dateStyle = .none
        return f.string(from: b.fetchedAt)
    }

    func vixIndicatorColor() -> Color {
        guard let v = morningBrief?.vix else { return NotchTheme.accentTeal }
        if v < 15 { return NotchTheme.accentTeal }
        if v <= 20 { return NotchTheme.accentWarning }
        return NotchTheme.accentDanger
    }

    /// Quick-run grid: prefer daemon workflows; pad with built-ins.
    var quickWorkflowItems: [QuickWorkflowItem] {
        var rows: [QuickWorkflowItem] = activeWorkflows.map {
            QuickWorkflowItem(workflowId: $0.id, name: $0.name, isRunning: $0.isRunning)
        }
        let builtins: [(String, String)] = [
            ("operator_intelligence_feed", "Operator Feed"),
            ("morning_brief", "Morning Brief"),
            ("position_scan", "Position Scan"),
            ("risk_check", "Risk Check"),
        ]
        for b in builtins where !rows.contains(where: { $0.workflowId == b.0 }) {
            rows.append(QuickWorkflowItem(workflowId: b.0, name: b.1, isRunning: false))
        }
        return Array(rows.prefix(6))
    }

    func fireSmartTriggerPulse() {
        scoreRingPulseScale = 1.05
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 150_000_000)
            scoreRingPulseScale = 1.0
        }
    }

    /// Subtle ring pulse on every new fill (Sprint 1 — closes feedback loop even when score is flat).
    func fireTradeFillRingPulse() {
        NotchHaptics.play(.light)
        scoreRingPulseScale = 1.02
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 500_000_000)
            scoreRingPulseScale = 1.0
        }
    }
}

extension NotchViewModel {
    func formatINR(_ v: Double) -> String {
        let f = NumberFormatter()
        f.numberStyle = .currency
        f.currencyCode = "INR"
        f.maximumFractionDigits = 0
        return f.string(from: NSNumber(value: v)) ?? "₹\(Int(v))"
    }
}
