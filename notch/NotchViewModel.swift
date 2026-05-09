import AppKit
import Combine
import CryptoKit
import Darwin
import Foundation
import Security
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

struct RecentTrade: Identifiable {
    var id: String
    var symbol: String
    var side: String
    var qty: Int
    var price: Double
    var filledAt: String
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
    /// Journal toolbar capture — design §6.3 (expanded panel).
    case capture = "CAPTURE"
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
    @Published var killSwitchCountdownSecs: Int?
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
    @Published var daemonConnectionState: DaemonConnectionState = .idle
    @Published var daemonProtocolError: DaemonProtocolErrorClass?
    @Published var brokerSyncClass: BrokerSyncClass = .notConnected
    @Published var brokerLastError: String?
    /// Phase 9 — founder QA snapshot text from `/api/daemon/health` (+ metrics URL hint).
    @Published var agentLocalDiagnostics: String = ""
    @Published var dictationWaveform: [Float] = Array(repeating: 0, count: WaveformNineDotRing.dotCount)
    @Published var isDictating: Bool = false
    @Published var dictationPermissionDenied: Bool = false
    @Published var dictationOnDeviceOnlyUnsupported: Bool = false
    /// Expanded-panel capture body (design §6.3). Dictation targets this when `dictationUsesCaptureDraft` is `true`.
    @Published var journalCaptureDraft: String = ""
    /// When the journal capture `TextEditor` exists, set `true` and move the mic/waveform UI there; dictation then fills `journalCaptureDraft` instead of `taiInput`.
    @Published var dictationUsesCaptureDraft: Bool = false
    /// Optional trade UUID (v4) when finalizing as linked capture. Ignored when `journalCaptureExplicitPending`.
    @Published var journalCaptureTradeIdRaw: String = ""
    @Published var journalCaptureExplicitPending: Bool = false
    @Published var journalCaptureBusy: Bool = false
    @Published var journalCaptureBanner: String?
    @Published var journalCaptureLastError: String?
    @Published var journalCaptureLastSuccess: String?
    /// After a successful finalize, optional screenshot attach uses this pending row (Phase 7).
    @Published var journalCaptureLastPendingCaptureId: String?
    @Published var journalCaptureScreenshotBusy: Bool = false
    @Published var journalCaptureScreenshotError: String?
    @Published var recentTrades: [RecentTrade] = []
    @Published var sessionAuthenticated: Bool = true
    @Published var sessionLabel: String?

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

    var daemonConnectionLabel: String {
        if let daemonProtocolError {
            switch daemonProtocolError {
            case .protoVersion:
                return "Update Required"
            case .sigInvalid:
                return "Security Blocked"
            case .validation:
                return "Request Invalid"
            case .rateLimited:
                return "Rate Limited"
            case .unknown:
                return "Agent Error"
            }
        }
        switch daemonConnectionState {
        case .idle, .connecting:
            return "Connecting"
        case .connected:
            return "Connected"
        case .reconnecting:
            return "Reconnecting"
        case .disconnected:
            return "Disconnected"
        }
    }

    var daemonConnectionColor: Color {
        if let daemonProtocolError {
            switch daemonProtocolError {
            case .protoVersion, .validation, .rateLimited:
                return Color(hex: "#F5A524")
            case .sigInvalid, .unknown:
                return Color(hex: "#FF3B30")
            }
        }
        switch daemonConnectionState {
        case .connected:
            return Color(hex: "#00E5C0")
        case .reconnecting:
            return Color(hex: "#F5A524")
        case .disconnected:
            return Color(hex: "#FF3B30")
        case .idle, .connecting:
            return Color.white.opacity(0.5)
        }
    }

    var totalUnrealizedPnL: Double { unrealizedTotal }

    var totalExposure: Double { totalExposureApprox }

    var quickWorkflows: [QuickWorkflowItem] { quickWorkflowItems }

    var daemonSecret: String = ""
    var daemonPort: UInt16 = 9137
    var daemonUserId: String = "ac35ef44-6366-40d6-89d4-95530e8e3dbf"
    var webBaseURL: String = "https://localhost:3000"

    private var pollFast: Timer?
    private var pollWorkflows: Timer?
    private var pollBrief: Timer?
    private var pollTrades: Timer?
    private var connectionTick: Timer?
    private var lastBriefFetchDay: String?
    private var expandHoverTimer: Timer?
    private var collapseHoverTimer: Timer?
    private var daemonEventsTask: Task<Void, Never>?
    private var toolbarShowCoalesceTask: Task<Void, Never>?
    /// Interactive screenshot temp file — removed after upload or from `stopPolling()` (Phase 7 hygiene).
    private var journalCaptureScreenshotTempURL: URL?
    private var connectionFSM = DaemonConnectionFSM()
    /// Phase 9 (#56/#66): Ed25519 trust — boot id + pinned pubkey from `/health`.
    private var pinnedAgentBootId: String?
    private var pinnedAgentSsePubKeyB64: String?
    private var dictationSession: NotchOnDeviceDictationSession?

    weak var notchHost: NotchLauncherHost?

    private enum JournalCapturePersistence {
        static let draftKey = "tradeautopsy.notch.journalCapture.draft"
        static let tradeKey = "tradeautopsy.notch.journalCapture.tradeId"
        static let pendingKey = "tradeautopsy.notch.journalCapture.explicitPending"
        static let lastPendingForScreenshotKey =
            "tradeautopsy.notch.journalCapture.lastPendingForScreenshot"
    }

    /// §6.6 Policy C (swift slice): while the draft has non-whitespace text, trade UUID + pending toggle are fixed.
    var journalCaptureLinkLocked: Bool {
        !journalCaptureDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func selectTab(_ tab: NotchTab) {
        withAnimation(NotchTheme.springExpand) {
            activeTab = tab
        }
        dictationUsesCaptureDraft = (tab == .capture)
    }

    func persistJournalCaptureDraftLocally() {
        writeJournalCaptureDefaults()
        journalCaptureLastError = nil
        journalCaptureLastSuccess = "Draft saved locally."
    }

    private func writeJournalCaptureDefaults() {
        let d = UserDefaults.standard
        d.set(journalCaptureDraft, forKey: JournalCapturePersistence.draftKey)
        d.set(journalCaptureTradeIdRaw, forKey: JournalCapturePersistence.tradeKey)
        d.set(journalCaptureExplicitPending, forKey: JournalCapturePersistence.pendingKey)
        if let p = journalCaptureLastPendingCaptureId, !p.isEmpty {
            d.set(p, forKey: JournalCapturePersistence.lastPendingForScreenshotKey)
        } else {
            d.removeObject(forKey: JournalCapturePersistence.lastPendingForScreenshotKey)
        }
    }

    private func loadJournalCaptureDraftLocally() {
        let d = UserDefaults.standard
        journalCaptureDraft = d.string(forKey: JournalCapturePersistence.draftKey) ?? ""
        journalCaptureTradeIdRaw = d.string(forKey: JournalCapturePersistence.tradeKey) ?? ""
        if d.object(forKey: JournalCapturePersistence.pendingKey) != nil {
            journalCaptureExplicitPending = d.bool(forKey: JournalCapturePersistence.pendingKey)
        }
        journalCaptureLastPendingCaptureId =
            d.string(forKey: JournalCapturePersistence.lastPendingForScreenshotKey)
    }

    func finalizeJournalCapture() async {
        guard sessionAuthenticated else {
            journalCaptureLastError = "Session expired — tap Sign In before finalizing."
            journalCaptureLastSuccess = nil
            return
        }

        let draft = journalCaptureDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        let tradeT = journalCaptureTradeIdRaw.trimmingCharacters(in: .whitespacesAndNewlines)
        if let msg = JournalCaptureLinkValidator.validationMessage(
            draftTrimmed: draft,
            explicitPending: journalCaptureExplicitPending,
            tradeRawTrimmed: tradeT
        ) {
            journalCaptureLastError = msg
            journalCaptureLastSuccess = nil
            return
        }

        guard let url = URL(string: baseURL() + "/api/daemon/journal/toolbar-capture/accept") else {
            journalCaptureLastError = "Bad daemon URL"
            return
        }

        var body: [String: Any] = [
            "draftText": draft,
            "explicitPending": journalCaptureExplicitPending,
            "idempotencyKey": Self.makeULID(),
        ]
        if journalCaptureExplicitPending {
            body["tradeId"] = NSNull()
        } else {
            body["tradeId"] = tradeT
        }

        guard let payload = try? JSONSerialization.data(withJSONObject: body) else {
            journalCaptureLastError = "Could not build JSON"
            return
        }

        journalCaptureBusy = true
        journalCaptureLastError = nil
        journalCaptureLastSuccess = nil
        defer { journalCaptureBusy = false }

        let req = authorizedRequest(url: url, method: "POST", body: payload)
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                journalCaptureLastError = daemonProtocolError.map { $0.rawValue } ?? "Finalize failed (\(statusCode))"
                return
            }
            daemonProtocolError = nil
            guard
                let j = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                let ok = j["success"] as? Bool, ok,
                let inner = j["data"] as? [String: Any],
                let pendingId = inner["pending_capture_id"] as? String
            else {
                journalCaptureLastError = "Unexpected server response"
                return
            }
            let st = (inner["status"] as? String) ?? "accepted"
            journalCaptureLastPendingCaptureId = pendingId
            journalCaptureDraft = ""
            journalCaptureTradeIdRaw = ""
            journalCaptureExplicitPending = false
            writeJournalCaptureDefaults()
            journalCaptureLastSuccess = "Saved (\(st)) · \(pendingId.prefix(8))… — you can attach a screenshot."
            journalCaptureLastError = nil
        } catch {
            journalCaptureLastError = error.localizedDescription
        }
    }

    /// Phase 7: `screencapture -i` → agent `/api/daemon/screenshot/presign` → R2 PUT → agent PATCH pending (no hosted shortcuts).
    func attachJournalCaptureScreenshotToPending() async {
        let pid = journalCaptureLastPendingCaptureId?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !pid.isEmpty else {
            journalCaptureScreenshotError =
                "Finalize a capture first, then attach a screenshot."
            return
        }
        guard !daemonSecret.isEmpty else {
            journalCaptureScreenshotError = "Daemon secret not configured."
            return
        }

        journalCaptureScreenshotBusy = true
        journalCaptureScreenshotError = nil
        defer { journalCaptureScreenshotBusy = false }

        var tempURL: URL?
        do {
            let captureURL = try await JournalInteractiveScreenshot.captureRegionToTempPNG()
            tempURL = captureURL
            journalCaptureScreenshotTempURL = captureURL
            let pngData = try Data(contentsOf: captureURL)

            guard let presignURL = URL(string: baseURL() + "/api/daemon/screenshot/presign") else {
                journalCaptureScreenshotError = "Bad daemon URL"
                return
            }
            let fileName = "toolbar-\(Int(Date().timeIntervalSince1970)).png"
            let presignBody: [String: Any] = [
                "pending_capture_id": pid,
                "content_type": "image/png",
                "filename": fileName,
            ]
            guard let presignPayload = try? JSONSerialization.data(withJSONObject: presignBody) else {
                journalCaptureScreenshotError = "Could not build presign JSON"
                return
            }
            let presignReq = authorizedRequest(url: presignURL, method: "POST", body: presignPayload)
            let (presignData, presignResp) = try await URLSession.shared.data(for: presignReq)
            let presignCode = (presignResp as? HTTPURLResponse)?.statusCode ?? 0
            guard presignCode == 200 else {
                journalCaptureScreenshotError = screenshotFlowProtocolMessage(
                    data: presignData,
                    statusCode: presignCode
                )
                return
            }
            daemonProtocolError = nil

            guard
                let presignObj = try? JSONSerialization.jsonObject(with: presignData) as? [String: Any],
                let presignOk = presignObj["success"] as? Bool, presignOk,
                let dataObj = presignObj["data"] as? [String: Any],
                let uploadUrlStr = dataObj["uploadUrl"] as? String,
                let uploadURL = URL(string: uploadUrlStr),
                let r2Key = dataObj["key"] as? String
            else {
                journalCaptureScreenshotError = "Unexpected presign response"
                return
            }

            let putCode = try await putPngToPresignedUrlWithRetry(uploadURL: uploadURL, pngData: pngData)
            guard (200...299).contains(putCode) else {
                if putCode == 429 {
                    journalCaptureScreenshotError = "Upload rate limited — try again shortly."
                } else {
                    journalCaptureScreenshotError = "Upload failed (\(putCode))"
                }
                return
            }

            let patchPath = "/api/daemon/journal/toolbar-capture/pending/" + pid
            guard let patchURL = URL(string: baseURL() + patchPath) else {
                journalCaptureScreenshotError = "Bad daemon URL"
                return
            }
            let patchBody: [String: Any] = ["r2_key": r2Key]
            guard let patchPayload = try? JSONSerialization.data(withJSONObject: patchBody) else {
                journalCaptureScreenshotError = "Could not build patch JSON"
                return
            }
            let patchReq = authorizedRequest(url: patchURL, method: "PATCH", body: patchPayload)
            let (patchData, patchResp) = try await URLSession.shared.data(for: patchReq)
            let patchCode = (patchResp as? HTTPURLResponse)?.statusCode ?? 0
            guard patchCode == 200 else {
                journalCaptureScreenshotError = screenshotFlowProtocolMessage(
                    data: patchData,
                    statusCode: patchCode
                )
                return
            }
            daemonProtocolError = nil
            journalCaptureScreenshotError = nil
            journalCaptureLastPendingCaptureId = nil
            writeJournalCaptureDefaults()
            journalCaptureLastSuccess = "Screenshot attached · \(pid.prefix(8))…"
            journalCaptureLastError = nil
        } catch is JournalInteractiveScreenshotError {
            journalCaptureScreenshotError = "Screenshot cancelled or failed."
        } catch {
            journalCaptureScreenshotError = error.localizedDescription
        }

        if let u = tempURL {
            JournalInteractiveScreenshot.removeTempFile(at: u)
        }
        journalCaptureScreenshotTempURL = nil
    }

    /// Presigned R2 PUT with bounded 429 retries (Phase 7).
    private func putPngToPresignedUrlWithRetry(uploadURL: URL, pngData: Data) async throws -> Int {
        var lastCode = 0
        for attempt in 0..<3 {
            var putReq = URLRequest(url: uploadURL)
            putReq.httpMethod = "PUT"
            putReq.setValue("image/png", forHTTPHeaderField: "Content-Type")
            let (_, putResp) = try await URLSession.shared.upload(for: putReq, from: pngData)
            let code = (putResp as? HTTPURLResponse)?.statusCode ?? 0
            lastCode = code
            if (200...299).contains(code) { return code }
            if code == 429, attempt < 2 {
                let ms = 300 * (1 << attempt)
                try await Task.sleep(nanoseconds: UInt64(ms) * 1_000_000)
                continue
            }
            break
        }
        return lastCode
    }

    func startPolling() {
        stopPolling()
        loadJournalCaptureDraftLocally()
        startDaemonEventsStream()
        connectionTick = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                self.daemonConnectionState = self.connectionFSM.onTick()
            }
        }
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

        pollTrades = Timer.scheduledTimer(withTimeInterval: 30, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                if self.isIstMarketSession() {
                    await self.fetchRecentTrades()
                }
            }
        }

        Task {
            await fetchWorkflowStatus()
            await fetchMorningBriefIfNeeded()
            await fetchRecentTrades()
            if isIstMarketSession() {
                await fetchPulseData()
                await fetchPositions()
            }
        }
    }

    func stopPolling() {
        toolbarShowCoalesceTask?.cancel()
        toolbarShowCoalesceTask = nil
        daemonEventsTask?.cancel()
        daemonEventsTask = nil
        pollFast?.invalidate()
        pollFast = nil
        pollWorkflows?.invalidate()
        pollWorkflows = nil
        pollBrief?.invalidate()
        pollBrief = nil
        pollTrades?.invalidate()
        pollTrades = nil
        connectionTick?.invalidate()
        connectionTick = nil
        daemonConnectionState = .idle
        daemonProtocolError = nil
        connectionFSM = DaemonConnectionFSM()
        pinnedAgentBootId = nil
        pinnedAgentSsePubKeyB64 = nil
        agentLocalDiagnostics = ""
        stopHoverTimers()
        if let u = journalCaptureScreenshotTempURL {
            JournalInteractiveScreenshot.removeTempFile(at: u)
            journalCaptureScreenshotTempURL = nil
        }
        dictationSession?.stopAllForHostTeardown()
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

    private func startDaemonEventsStream() {
        daemonEventsTask?.cancel()
        daemonEventsTask = Task { @MainActor [weak self] in
            guard let self else { return }
            while !Task.isCancelled {
                self.connectionFSM.onConnectStart()
                self.daemonConnectionState = self.connectionFSM.state
                await self.refreshPinnedAgentTrust()
                guard self.pinnedAgentSsePubKeyB64 != nil else {
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                    continue
                }
                guard let url = URL(string: self.baseURL() + "/api/daemon/events/stream") else {
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    return
                }
                var req = self.authorizedRequest(url: url)
                req.setValue("text/event-stream", forHTTPHeaderField: "Accept")
                do {
                    let (bytes, resp) = try await URLSession.shared.bytes(for: req)
                    let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
                    if statusCode != 200 {
                        var body = Data()
                        for try await line in bytes.lines {
                            body.append(contentsOf: line.utf8)
                            if body.count >= 2048 { break }
                        }
                        self.handleDaemonErrorResponse(
                            data: body.isEmpty ? nil : body,
                            statusCode: statusCode
                        )
                        self.connectionFSM.onTransportFailure()
                        self.daemonConnectionState = self.connectionFSM.state
                        try? await Task.sleep(nanoseconds: 2_000_000_000)
                        continue
                    }
                    self.daemonProtocolError = nil
                    self.connectionFSM.onStreamOpened()
                    self.daemonConnectionState = self.connectionFSM.state
                    for try await line in bytes.lines {
                        if Task.isCancelled { return }
                        guard line.hasPrefix("data:") else { continue }
                        let raw = String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces)
                        if self.consumeDaemonEvent(raw) {
                            self.daemonConnectionState = self.connectionFSM.state
                        }
                    }
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                } catch {
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                }
            }
        }
    }

    private func consumeDaemonEvent(_ jsonString: String) -> Bool {
        guard
            let data = jsonString.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return false }
        guard let type = obj["type"] as? String else { return false }

        if let pk = pinnedAgentSsePubKeyB64 {
            guard
                let sig = obj["sig"] as? String,
                let eventId = obj["event_id"] as? String
            else {
                daemonProtocolError = .sigInvalid
                lastAlert = "SSE event missing signature fields."
                connectionFSM.onTransportFailure()
                daemonConnectionState = connectionFSM.state
                return false
            }
            guard verifySseEd25519(
                rawEnvelopeJson: jsonString,
                eventId: eventId,
                typeStr: type,
                sigB64: sig,
                pubKeyB64: pk
            ) else {
                daemonProtocolError = .sigInvalid
                lastAlert = "Invalid SSE event signature."
                connectionFSM.onTransportFailure()
                daemonConnectionState = connectionFSM.state
                return false
            }
        }

        if type == "agent_health" {
            daemonProtocolError = nil
            connectionFSM.onHeartbeat()
            return true
        }

        guard let payload = obj["payload"] as? [String: Any] else { return false }
        return applyDaemonEventPayload(type: type, payload: payload, immediateToolbarShow: false)
    }

    private func scheduleToolbarShowPayload(_ payload: [String: Any]) {
        toolbarShowCoalesceTask?.cancel()
        toolbarShowCoalesceTask = Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: 50_000_000)
            guard let self, !Task.isCancelled else { return }
            self.applyToolbarShowPayload(payload)
        }
    }

    private func applyToolbarShowPayload(_ payload: [String: Any]) {
        let tradeId = payload["trade_id"] as? String
        let symbol = payload["symbol"] as? String
        let side = (payload["side"] as? String)?.uppercased() ?? ""
        let qty: Int = {
            if let q = payload["qty"] as? Int { return q }
            if let d = payload["qty"] as? Double { return Int(d) }
            return 0
        }()
        let price: Double = {
            if let p = payload["price"] as? Double { return p }
            if let i = payload["price"] as? Int { return Double(i) }
            return 0
        }()

        let banner: String
        if let sym = symbol, !sym.isEmpty {
            let priceText = formatINR(price)
            banner = "Trade detected — \(sym) \(side) \(qty)@\(priceText)"
        } else {
            banner = "Trade detected — details syncing…"
        }
        journalCaptureBanner = banner
        if !journalCaptureLinkLocked {
            journalCaptureTradeIdRaw = tradeId ?? ""
        }
        Task { [weak self] in
            await self?.notchHost?.expandToCapture()
        }
    }

    /// Applies verified SSE payload-only updates (for production after `consumeDaemonEvent` verification, and tests).
    func applyDaemonEventPayload(type: String, payload: [String: Any], immediateToolbarShow: Bool) -> Bool {
        switch type {
        case "kill_switch_state":
            let oldKs = killSwitchActive
            let active = payload["active"] as? Bool ?? false
            killSwitchActive = active
            if active, !oldKs {
                checkKillSwitchTransition(from: false, to: true)
            }
            if !active {
                killSwitchCountdownSecs = nil
                if oldKs {
                    pulseAttention = .none
                }
            } else if let c = payload["countdown_secs"] as? Int {
                killSwitchCountdownSecs = c
            } else if let d = payload["countdown_secs"] as? Double {
                killSwitchCountdownSecs = Int(d)
            } else {
                killSwitchCountdownSecs = nil
            }
            return true

        case "broker_sync_state":
            if let c = payload["class"] as? String,
               let mapped = BrokerSyncClass(rawValue: c) {
                brokerSyncClass = mapped
            } else {
                brokerSyncClass = .notConnected
            }
            brokerLastError = payload["last_error"] as? String
            return true

        case "toolbar_show":
            if immediateToolbarShow {
                applyToolbarShowPayload(payload)
            } else {
                scheduleToolbarShowPayload(payload)
            }
            return true

        case "session_state", "state_changed":
            let authed = payload["authenticated"] as? Bool ?? true
            sessionAuthenticated = authed
            sessionLabel = payload["session_label"] as? String
            if !authed {
                journalCaptureLastError = "Session expired — sign in to continue"
            } else {
                journalCaptureLastError = nil
            }
            return true

        case "auth_state":
            let valid = payload["valid"] as? Bool ?? true
            if !valid {
                sessionAuthenticated = false
                journalCaptureLastError = "Session expired — sign in to continue"
            } else {
                sessionAuthenticated = true
                journalCaptureLastError = nil
            }
            return true

        default:
            return false
        }
    }

    func fetchRecentTrades() async {
        guard let url = URL(string: baseURL() + "/api/daemon/toolbar/recent-trades") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            if let sc = j["syncState"] as? String {
                brokerSyncClass = BrokerSyncClass(rawValue: sc) ?? .notConnected
            }
            if let arr = j["trades"] as? [[String: Any]] {
                recentTrades = arr.compactMap { row in
                    let id = (row["trade_id"] as? String) ?? (row["id"] as? String)
                    guard let rid = id, let sym = row["symbol"] as? String else { return nil }
                    let qty: Int = {
                        if let q = row["qty"] as? Int { return q }
                        if let d = row["qty"] as? Double { return Int(d) }
                        if let q = row["quantity"] as? Int { return q }
                        if let d = row["quantity"] as? Double { return Int(d) }
                        return 0
                    }()
                    let price: Double = {
                        if let p = row["price"] as? Double { return p }
                        if let p = row["fill_price"] as? Double { return p }
                        if let i = row["price"] as? Int { return Double(i) }
                        return 0
                    }()
                    let side = (row["side"] as? String) ?? (row["direction"] as? String) ?? ""
                    let filledAt = (row["filled_at"] as? String) ?? (row["detected_at"] as? String) ?? ""
                    return RecentTrade(
                        id: rid,
                        symbol: sym,
                        side: side,
                        qty: qty,
                        price: price,
                        filledAt: filledAt
                    )
                }
            }
        } catch {}
    }

    /// Phase 9 — GET `/health` establishes pubkey pinning lifecycle vs `boot_id` (design §4.4).
    private func refreshPinnedAgentTrust() async {
        guard let url = URL(string: baseURL() + "/api/daemon/health") else { return }
        let req = authorizedRequest(url: url)
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
            rebuildAgentLocalDiagnostics(from: json)
            guard let boot = json["boot_id"] as? String else { return }
            guard let pk = json["sse_signing_pubkey_b64"] as? String else { return }

            if let prevBoot = pinnedAgentBootId, prevBoot == boot,
               let prevPk = pinnedAgentSsePubKeyB64, prevPk != pk {
                daemonProtocolError = .sigInvalid
                lastAlert = "Agent SSE signing key changed unexpectedly — possible tampering."
                pinnedAgentSsePubKeyB64 = nil
                return
            }

            if pinnedAgentBootId != boot {
                pinnedAgentBootId = boot
            }
            pinnedAgentSsePubKeyB64 = pk
        } catch {}
    }

    /// Phase 9 — surfaces agent observability for Pulse tab (plan: founder-support snapshot).
    private func rebuildAgentLocalDiagnostics(from json: [String: Any]) {
        var lines: [String] = []
        if let v = json["version"] as? String {
            lines.append("agent \(v)")
        }
        if let u = json["uptime_secs"] as? Int {
            lines.append("uptime \(u)s")
        } else if let u = json["uptime_secs"] as? Double {
            lines.append("uptime \(Int(u))s")
        }
        if let mp = json["metrics_listen_port"], !(mp is NSNull) {
            if let p = mp as? Int {
                lines.append("prometheus http://127.0.0.1:\(p)/metrics")
            }
        }
        if let obs = json["observability"] as? [String: Any] {
            if let tick = obs["agent_uptime_seconds"] as? Int {
                lines.append("gauge_uptime \(tick)s")
            } else if let tick = obs["agent_uptime_seconds"] as? Double {
                lines.append("gauge_uptime \(Int(tick))s")
            }
            if let sse = obs["sse_events_published"] as? [String: Any],
               let n = sse["agent_health"] as? Int {
                lines.append("sse_health_emitted \(n)")
            }
            if let hf = obs["hmac_verify_failures_total"] as? Int {
                lines.append("wire_auth_failures \(hf)")
            }
        }
        if let pk = json["sse_signing_pubkey_b64"] as? String {
            lines.append("sse_pk \(pk.prefix(10))…")
        }
        agentLocalDiagnostics = lines.joined(separator: "\n")
    }

    private func verifySseEd25519(
        rawEnvelopeJson: String,
        eventId: String,
        typeStr: String,
        sigB64: String,
        pubKeyB64: String
    ) -> Bool {
        guard let payloadUtf8 = extractJsonObjectUtf8(forKey: "payload", inEnvelopeJsonLine: rawEnvelopeJson) else {
            return false
        }
        let digest = SHA256.hash(data: payloadUtf8)
        let digestHex = digest.map { String(format: "%02x", $0) }.joined()
        let msg = "\(eventId)\n\(typeStr)\n\(digestHex)"
        guard let sigData = Data(base64Encoded: sigB64, options: [.ignoreUnknownCharacters]) else { return false }
        guard let pkData = Data(base64Encoded: pubKeyB64, options: [.ignoreUnknownCharacters]),
              pkData.count == 32 else { return false }
        guard let pk = try? Curve25519.Signing.PublicKey(rawRepresentation: pkData) else { return false }
        guard let msgData = msg.data(using: .utf8) else { return false }
        return pk.isValidSignature(sigData, for: msgData)
    }

    /// Raw UTF-8 slice of the JSON object for `"payload"` — must match agent `serde_json::to_vec` bytes for hashing.
    private func extractJsonObjectUtf8(forKey key: String, inEnvelopeJsonLine line: String) -> Data? {
        let needle = "\"\(key)\":"
        guard let range = line.range(of: needle) else { return nil }
        var idx = range.upperBound
        while idx < line.endIndex, line[idx].isWhitespace {
            line.formIndex(after: &idx)
        }
        guard idx < line.endIndex, line[idx] == "{" else { return nil }
        let start = idx
        var depth = 0
        while idx < line.endIndex {
            let ch = line[idx]
            if ch == "{" { depth += 1 }
            else if ch == "}" {
                depth -= 1
                if depth == 0 {
                    let end = line.index(after: idx)
                    let sub = line[start..<end]
                    return String(sub).data(using: .utf8)
                }
            }
            line.formIndex(after: &idx)
        }
        return nil
    }

    private func handleDaemonErrorResponse(data: Data?, statusCode: Int) {
        if let data, let envelope = try? JSONDecoder().decode(DaemonErrorEnvelope.self, from: data) {
            daemonProtocolError = envelope.errorClass
            if !envelope.message.isEmpty {
                lastAlert = envelope.message
            }
        } else {
            daemonProtocolError = statusCode == 429 ? .rateLimited : .unknown
        }
        daemonConnectionState = .disconnected
    }

    /// Wire error text for toolbar screenshot flow without mutating SSE connection health.
    private func screenshotFlowProtocolMessage(data: Data?, statusCode: Int) -> String {
        if let data, let envelope = try? JSONDecoder().decode(DaemonErrorEnvelope.self, from: data) {
            if !envelope.message.isEmpty { return envelope.message }
            return envelope.errorClass.rawValue
        }
        if statusCode == 429 {
            return "Rate limited — try again shortly."
        }
        return "Request failed (\(statusCode))"
    }

    private func authorizedRequest(url: URL, method: String = "GET", body: Data? = nil) -> URLRequest {
        var r = URLRequest(url: url)
        r.httpMethod = method
        let payload = body ?? Data()
        let path = url.path.isEmpty ? "/" : url.path
        let timestamp = DateFormatter.wireTimestamp.string(from: Date())
        let requestId = Self.makeULID()
        let nonce = Self.makeNonceBase64()
        let sig = makeWireSignature(
            method: method,
            path: path,
            timestamp: timestamp,
            requestId: requestId,
            body: payload
        )

        r.setValue("1", forHTTPHeaderField: "x-proto-version")
        r.setValue(daemonSecret, forHTTPHeaderField: "x-daemon-secret")
        r.setValue(daemonUserId, forHTTPHeaderField: "x-user-id")
        r.setValue(requestId, forHTTPHeaderField: "x-request-id")
        r.setValue(timestamp, forHTTPHeaderField: "x-timestamp")
        r.setValue(nonce, forHTTPHeaderField: "x-nonce")
        r.setValue(sig, forHTTPHeaderField: "x-signature")
        r.setValue("notch", forHTTPHeaderField: "x-daemon-source")
        if body != nil {
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
            r.httpBody = body
        }
        #if os(macOS)
        if let bundle = Bundle.main.bundleIdentifier {
            r.setValue(bundle, forHTTPHeaderField: "x-tradeautopsy-caller-bundle-id")
        }
        r.setValue(String(getpid()), forHTTPHeaderField: "x-tradeautopsy-caller-pid")
        #endif
        return r
    }

    private func makeWireSignature(
        method: String,
        path: String,
        timestamp: String,
        requestId: String,
        body: Data
    ) -> String {
        let bodyHash = SHA256.hash(data: body).map { String(format: "%02x", $0) }.joined()
        let canonical = "\(method.uppercased())\n\(path)\n\(timestamp)\n\(requestId)\n\(bodyHash)"
        let key = SymmetricKey(data: Data(daemonSecret.utf8))
        let sig = HMAC<SHA256>.authenticationCode(for: Data(canonical.utf8), using: key)
        return Data(sig).base64EncodedString()
    }

    private static func makeNonceBase64() -> String {
        var bytes = [UInt8](repeating: 0, count: 16)
        _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        return Data(bytes).base64EncodedString()
    }

    /// ULID generator for wire `x-request-id` contract.
    private static func makeULID() -> String {
        let ms = UInt64(Date().timeIntervalSince1970 * 1000.0)
        var randomness = [UInt8](repeating: 0, count: 10)
        _ = SecRandomCopyBytes(kSecRandomDefault, randomness.count, &randomness)

        var bytes = [UInt8](repeating: 0, count: 16)
        bytes[0] = UInt8((ms >> 40) & 0xFF)
        bytes[1] = UInt8((ms >> 32) & 0xFF)
        bytes[2] = UInt8((ms >> 24) & 0xFF)
        bytes[3] = UInt8((ms >> 16) & 0xFF)
        bytes[4] = UInt8((ms >> 8) & 0xFF)
        bytes[5] = UInt8(ms & 0xFF)
        for i in 0..<10 { bytes[6 + i] = randomness[i] }
        return encodeULID(bytes)
    }

    private static func encodeULID(_ bytes: [UInt8]) -> String {
        let alphabet = Array("0123456789ABCDEFGHJKMNPQRSTVWXYZ")
        precondition(bytes.count == 16)
        var out: [Character] = []
        out.reserveCapacity(26)
        var buffer = 0
        var bitCount = 0

        for byte in bytes {
            buffer = (buffer << 8) | Int(byte)
            bitCount += 8
            while bitCount >= 5 {
                let idx = (buffer >> (bitCount - 5)) & 0x1F
                out.append(alphabet[idx])
                bitCount -= 5
            }
        }

        if bitCount > 0 {
            let idx = (buffer << (5 - bitCount)) & 0x1F
            out.append(alphabet[idx])
        }

        if out.count < 26 {
            out = Array(repeating: "0", count: 26 - out.count) + out
        } else if out.count > 26 {
            out = Array(out.suffix(26))
        }

        return String(out)
    }

    func fetchPulseData() async {
        guard let url = URL(string: baseURL() + "/api/daemon/pulse") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
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
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
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
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
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
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
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
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
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

    func beginOAuthSignIn() async {
        guard let url = URL(string: baseURL() + "/api/daemon/auth/begin") else { return }
        let body = try? JSONSerialization.data(withJSONObject: ["source": "notch"])
        let req = authorizedRequest(url: url, method: "POST", body: body)
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            guard (resp as? HTTPURLResponse)?.statusCode == 200,
                  let j = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let loginUrl = j["loginUrl"] as? String,
                  !loginUrl.isEmpty
            else { return }
            openDeepLink(loginUrl)
        } catch {}
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

    func ensureDictationWired() {
        guard dictationSession == nil else { return }
        let s = NotchOnDeviceDictationSession()
        s.onText = { [weak self] t in
            self?.dictationApplyFullFieldText(t)
        }
        s.onWaveform = { [weak self] levels in
            self?.dictationWaveform = levels
        }
        s.onPermissionBlocked = { [weak self] blocked in
            self?.dictationPermissionDenied = blocked
        }
        s.onRequiresOnDeviceUnsupported = { [weak self] bad in
            self?.dictationOnDeviceOnlyUnsupported = bad
        }
        s.onRecordingState = { [weak self] on in
            self?.isDictating = on
        }
        dictationSession = s
        s.preparePermissions()
    }

    func toggleDictationLatch(reduceMotion: Bool) {
        ensureDictationWired()
        dictationSession?.setReduceMotion(reduceMotion)
        dictationSession?.toggleLatch(userPrefix: dictationUserPrefixForSession())
    }

    func dictationPushToTalk(fnDown: Bool, reduceMotion: Bool) {
        ensureDictationWired()
        dictationSession?.setReduceMotion(reduceMotion)
        dictationSession?.setFnHeld(fnDown, userPrefix: dictationUserPrefixForSession())
    }

    func updateDictationReduceMotion(_ on: Bool) {
        dictationSession?.setReduceMotion(on)
    }

    func openDictationPrivacySettings() {
        dictationSession?.openSystemPrivacySettings()
    }

    private func dictationUserPrefixForSession() -> String {
        dictationUsesCaptureDraft ? journalCaptureDraft : taiInput
    }

    private func dictationApplyFullFieldText(_ text: String) {
        if dictationUsesCaptureDraft {
            journalCaptureDraft = text
        } else {
            taiInput = text
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

private extension DateFormatter {
    static let wireTimestamp: DateFormatter = {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss.SSS'Z'"
        return formatter
    }()
}
