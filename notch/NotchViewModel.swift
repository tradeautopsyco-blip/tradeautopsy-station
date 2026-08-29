import AppKit
import Combine
import CryptoKit
import Darwin
import Foundation
import SwiftUI

/// Local optimistic “armed” state after `POST …/declare` until `live-state` shows matching `pending_declaration`.
struct BarOptimisticArmedSnapshot: Equatable {
    let declarationId: String
    let submittedAt: TimeInterval
    let symbol: String
    let side: String
    let quantityLabel: String
}

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

public struct NotchPosition: Identifiable {
    public var id: String { symbol + "\(qty)" }
    public var symbol: String
    public var qty: Int
    public var unrealizedPnL: Double
    public var direction: String

    public init(symbol: String, qty: Int, unrealizedPnL: Double, direction: String) {
        self.symbol = symbol
        self.qty = qty
        self.unrealizedPnL = unrealizedPnL
        self.direction = direction
    }
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
    // MARK: Behavioral morning brief (#127)
    var tradeCount: Int
    var isNewUser: Bool
    var patterns: [BriefBehavioralPattern]
    var ownMetrics: BriefOwnMetrics?
    var behavioralDateLine: String?
    var behavioralHeadline: String?
    var sessionPnLKpi: Double?
    var planAdherencePct: Double?
    var winRateKpi: Double?
    var leftOnTableInr: Double?
    var nonNegotiableRule: String?
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

struct RecentTradeRow: Identifiable, Equatable {
    var id: String
    var symbol: String
    var side: String
    var qty: Int
    var price: Double
    var filledAtMs: Int
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
    /// Behavioral circuit — consumes hosted `notch` (M4/M5/M9 projection).
    case plan = "PLAN"
    /// Journal toolbar capture — design §6.3 (expanded panel).
    case capture = "CAPTURE"
}

public enum BarSurfacePhase: String {
    case livePlan
    case armed
    case declaration
    case debrief
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
public final class NotchViewModel: ObservableObject {
    @Published var compositeScore: Double = 0
    @Published var behavioralState: String = "CALM"
    /// Joined multiplier labels from hosted live-state (#184).
    @Published var barBehavioralMultiplierLabel: String?
    @Published public var sessionPnL: Double = 0
    @Published var winRate: Double = 0
    @Published var tradesToday: Int = 0
    @Published var signals = SignalBreakdown()
    @Published public var positions: [NotchPosition] = []
    @Published var dailyLossLimit: Double?
    @Published var chipCatalog = NotchChipCatalogState()

    /// Open broker positions mirrored in Notch; used by PLAN honesty ladder (**thesis unknown** when empty plan + non-empty positions).
    var hasOpenPositions: Bool { !positions.isEmpty }
    @Published var openOrders: Int = 0
    @Published public var killSwitchActive: Bool = false
    @Published public var killSwitchCountdownSecs: Int?
    /// Seconds since the last kill-switch state update (SSE or daemon poll). `Int.max` if never received.
    public var killSwitchStateAgeSecs: Int {
        guard let killSwitchStateReceivedAt else { return Int.max }
        return max(0, Int(Date().timeIntervalSince(killSwitchStateReceivedAt)))
    }
    private var killSwitchStateReceivedAt: Date?
    /// L1 / L2 / L3 from agent SSE `kill_switch_state`.
    @Published var killSwitchLevel: String?
    @Published var killSwitchRequiresAck: Bool = false
    /// Fullscreen overlay (#189) — L2/L3 with countdown.
    @Published var killSwitchOverlayVisible: Bool = false
    @Published public var killSwitchDismissBusy: Bool = false
    @Published var morningBrief: MorningBrief?
    @Published var activeWorkflows: [WorkflowStatus] = []
    @Published var recentWorkflowRuns: [WorkflowRun] = []
    @Published var taiMessages: [TAIMessage] = []
    @Published var taiInput: String = ""
    @Published var taiMode: TaiMode = .ambient
    @Published var isLoading: Bool = false
    @Published var activeTab: NotchTab = .pulse
    /// When true (Station-hosted), expanded surface stays on PLAN / `BarNotchShell` only.
    private(set) var planSurfaceOnly: Bool = false
    @Published var lastAlert: String?
    @Published var pulseAttention: PulseAttention = .none
    @Published public var brokerSessionActive: Bool = false
    /// Catalog slug of the active UBI sync (`binance_com` / `kotak_neo`) — desk honesty (R7).
    @Published public var activeBrokerSlug: String?
    /// Quote currency for Notch/Today formatting; follows active connection (never FX-blend).
    @Published public var deskQuoteCurrency: String?
    /// Calc profile id for the active connection (`crypto_spot_usd` / `equities_inr_cash`).
    @Published public var deskCalcProfileId: String?
    /// True while broker sync is running or has fresh/stale data — drives Today session mirror polling.
    public var isBrokerSyncActiveForTodayMirror: Bool {
        switch brokerSyncClass {
        case "syncing", "synced", "stale":
            return true
        default:
            return false
        }
    }
    /// User click + programmatic expansion (never hover-to-expand).
    @Published var isExpanded: Bool = false
    /// Set by `NotchPanelController`: the `NSPanel` currently sits at the expanded frame.
    /// True during the collapse exit fade — the pill must not render centered in the big frame.
    @Published var summonPanelAtExpandedFrame: Bool = false
    /// Expanded panel size from `NotchPanelController.layoutPanel` — lets the expanded surface
    /// pre-warm its layout at final size while the window is still the collapsed pill.
    @Published var expandedSurfaceSize: CGSize = .zero
    /// One-shot ring scale pulse after smart trigger (0.3s).
    @Published var scoreRingPulseScale: CGFloat = 1.0
    /// Full-panel tint pulse (smart triggers).
    @Published var backgroundPulseColor: Color = .clear
    @Published var backgroundPulseOpacity: Double = 0
    @Published var daemonConnectionState: DaemonConnectionState = .idle
    @Published var daemonProtocolError: DaemonProtocolErrorClass?
    @Published var brokerSyncClass: String = "not_connected"
    /// Last successful `/api/daemon/broker/sync-state` poll (`lastPollAtMs` or now).
    @Published var brokerSyncLastPollAtMs: Int?
    @Published var sessionState: String = "active"
    @Published var isAuthenticated: Bool = true
    @Published var authProvider: String?
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
    @Published var unpostedCaptures: [UnpostedCaptureRecord] = []
    @Published var stagedCapturePreview: NSImage?
    @Published var unpostedSweepDropped: Int = 0
    @Published var unpostedSendBusyId: UUID?
    @Published var unpostedLinkTradeId: String = ""
    @Published var unpostedSearch: String = ""
    @Published var replaceConfirmTradeId: String?
    /// Trade UUIDs that already have a screenshot this device has sent (one image per trade).
    @Published var tradeIdsWithChart: Set<String> = []
    /// Paste / shutter should jump PLAN sidebar to Live trade (ignored during debrief).
    @Published var requestLiveCaptureScreen: Bool = false
    private var stagedCaptureData: Data?
    private var stagedCaptureType: String = "image/png"
    private let unpostedStore = UnpostedCaptureStore(directory: UnpostedCaptureStore.defaultDirectory())
    private var capturePasteboardWatch: Timer?
    private var capturePasteboardChangeCount: Int = 0
    @Published var recentTrades: [RecentTradeRow] = []

    // MARK: - Bar / Plan (notch v1 — server projection only)

    @Published var barLiveState: BarLiveStateResponse?
    @Published var barFeaturesActiveFromApi: Bool?
    /// Live-state GET in flight (`/api/daemon/bar/live-state`).
    @Published var barStateLoading: Bool = false
    @Published var barStateError: String?
    /// True when `barStateError` is specifically "device login never completed" — lets the UI
    /// offer an "Open Station" CTA instead of a generic dead-end error, and keeps it visually
    /// distinct from the broker sync pill (unrelated subsystem, see `BarLiveStateErrorPresentation`).
    @Published var barStateRequiresDeviceLogin: Bool = false
    @Published var barLastFetched: Date?
    @Published var barSurfacePhase: BarSurfacePhase = .declaration
    @Published var barDebriefPending: Bool = false
    @Published var planKillPhase: BarPlanKillPhase = .idle
    @Published var stopMeStep: Int = 0
    /// Exit trade / cancel declaration — 0 hidden, 1 confirm, 2 submitting (#144).
    @Published var cancelDeclStep: Int = 0
    @Published var cancelDeclError: String?
    /// Confirms intent before reason pick — requires `stopMeRequiredTaps` on the entry control (#124 parity).
    @Published var stopMeTapCount: Int = 0
    @Published var stopMeReason: String = ""
    @Published var barStopMeBusy: Bool = false
    @Published var barStopMeClearBusy: Bool = false
    let stopMeRequiredTaps: Int = 3
    let stopMeReasonChips: [String] = [
        "Tilt / revenge",
        "Overtrading",
        "Moved my stop",
        "Size mistake",
        "Done for the day",
    ]
    @Published var barDeclarationBusy: Bool = false
    @Published var barProtectiveBusy: Bool = false
    /// #5 — post-trade debrief PATCH via agent.
    @Published var barPostTradeDebriefBusy: Bool = false
    @Published var barPostTradeDebriefLastError: String?
    /// #4 — swing daily check-in POST via agent.
    @Published var barSwingCheckInBusy: Bool = false
    @Published var barSwingCheckInLastError: String?
    /// Broker slug for `place_sl` — must match server `resolveBrokerOrderPort`. Empty = no broker selected.
    @Published var barProtectiveBrokerSlug: String = ""
    @Published var barDeclarationLastError: String?
    @Published private(set) var barOptimisticArmedDisplay: BarOptimisticArmedSnapshot?
    /// Server `declaration_id` after successful declare; cleared when optimistic armed is cleared (#119).
    @Published private(set) var barPublishedDeclarationId: String?
    @Published var barDeclarationConfirmWarning: String?

    /// Live plan "interference" question — chosen chip id: `no` | `maybe` | `yes`.
    @Published var barInterferenceChoice: String?
    @Published var barInterferenceEcho: String?

    /// When `barSurfacePhase` is `.declaration`, gate the heavy form behind this affordance.
    @Published public var showingDeclarationForm: Bool = false

    /// Pre-trade symbol field — shared with declaration form autocomplete (#148).
    @Published var barDeclarationSymbol: String = ""
    @Published var symbolSuggestions: [InstrumentResult] = []
    @Published var showSymbolSuggestions: Bool = false
    @Published var symbolSearchHint: String?

    /// Derived from pending declaration kind, then persisted last-known, then `intraday` (#114).
    @Published private(set) var activeArchetype: TraderArchetype = .intraday

    /// Optional mirror for a 6-step declaration UX (wired when integrating with notch declare).
    @Published var declarationStep: Int = 1

    @Published var declEmotionalCalm: Int = 0
    @Published var declEmotionalConfidence: Int = 0
    @Published var declEntryPrice: String = ""
    /// Set when LTP fetch returns `session_expired` — show reconnect hint on entry field (#149).
    @Published var barLtpFetchError: String?
    @Published var declStopLoss: String = ""
    @Published var declTarget: String = ""
    @Published var declSetupType: String = ""
    @Published var declInvalidationType: String = ""
    @Published var declInvalidationCondition: String = ""
    @Published var declProtectiveSLConsent: Bool = true

    /// Spot / equity / options — not the Intraday/Swing style tabs.
    @Published var declareAssetClass: BarDeclareAssetClass = .spot {
        didSet {
            guard oldValue != declareAssetClass else { return }
            reconcileDeskLastForAssetClass()
        }
    }
    @Published var declOptionStrike: String = ""
    @Published var declOptionExpiry: String = ""
    @Published var wantOptionsChain: Bool = true
    @Published var wantOptionsOI: Bool = true
    @Published var declOptionRight: String = "CE"
    @Published var declLots: String = ""
    @Published var declHorizonDays: Int = 1
    @Published var declHorizonMode: BarPlanHorizon.Mode = .today
    @Published var declMaxPlannedLossText: String = ""
    @Published var optionLegs: [BarIntradayDeclarationPayload.OptionLeg] = []
    @Published var deskLastStatus: String = "unavailable"
    @Published var deskHistoryStatus: String = "unavailable"
    @Published var deskHistoryIneligible: [String] = []
    @Published var deskYahooHistoryStatus: String = "unavailable"
    @Published var deskYahooHistoryIneligible: [String] = []
    @Published var deskChainStatus: String = "unavailable"
    @Published var deskOiStatus: String = "unavailable"
    /// Desk-level `capabilities.quote` from sync-state — independent of funds/fills.
    @Published var deskQuoteCapability: String = "unavailable"
    /// Desk-level `capabilities.funds` from sync-state — independent of quote.
    @Published var deskFundsCapability: String = "unavailable"
    /// Desk-level `capabilities.fills` from sync-state — independent of quote.
    @Published var deskFillsCapability: String = "unavailable"
    /// Desk-level `capabilities.instruments` from sync-state — independent of quote/account.
    @Published var deskInstrumentsCapability: String = "unavailable"
    /// Last selected TickBook id (`nse_cm|2885`, `nse_fo|token`, or Binance pair).
    @Published var deskSelectedInstrumentId: String = ""

    /// Called from `NotchPanelController` to front the panel when expanding explicitly.
    var onRequestOrderFront: (() -> Void)?
    /// Hide the whole Notch HUD (pill gone) until ⌥Space / show brings it back.
    var onRequestHidePill: (() -> Void)?
    /// Collapsed pill drag — 1:1 follow from screen mouse (grab offset preserved).
    var onCollapsedPillDragFromScreen: (() -> Void)?
    /// Collapsed pill drag ended — snap to edges and persist origin.
    var onCollapsedPillDragEnded: (() -> Void)?
    /// Station bridge: open desk Brokers + Connect sheet.
    public var onRequestOpenBrokerConnect: ((String) -> Void)?
    /// Station bridge: open desk Brokers + Edit/remint sheet.
    public var onRequestOpenBrokerReauth: ((String) -> Void)?
    /// Station bridge: open Settings (device login) — distinct from broker Connect/Reauth above.
    public var onRequestOpenDeviceLogin: (() -> Void)?

    @Published public var brokerActionBusy: Bool = false
    @Published public var brokerActionResultMessage: String?
    @Published public var brokerActionError: String?

    /// Station bridge reports Connect Start/remint outcome into Settings.
    public func reportBrokerBridgeOutcome(result: String?, error: String?) {
        brokerActionBusy = false
        brokerActionResultMessage = result
        brokerActionError = error
    }

    /// Collapse + hide the floating pill/panel (sidebar “Hide notch”).
    public func requestHidePill() {
        if isExpanded {
            isExpanded = false
        }
        onRequestHidePill?()
    }

    func applyCollapsedPillDragFromScreen() {
        guard !isExpanded else { return }
        onCollapsedPillDragFromScreen?()
    }

    func endCollapsedPillDrag() {
        onCollapsedPillDragEnded?()
    }

    private var resolvedBrokerSlugForBridge: String {
        let slug = (activeBrokerSlug ?? barProtectiveBrokerSlug)
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        // Empty protective slug still bridges Connect to first-pair dogfood default.
        if slug.isEmpty { return "kotak_neo" }
        return slug
    }

    public func requestOpenBrokerConnect() {
        collapseExpandedFromChromeTap()
        onRequestOpenBrokerConnect?(resolvedBrokerSlugForBridge)
    }

    public func requestOpenBrokerReauth() {
        collapseExpandedFromChromeTap()
        onRequestOpenBrokerReauth?(resolvedBrokerSlugForBridge)
    }

    public func requestOpenDeviceLogin() {
        collapseExpandedFromChromeTap()
        onRequestOpenDeviceLogin?()
    }

    private var symbolSearchTask: Task<Void, Never>?
    private var ignoreSymbolSearchUntilEdit = false

    /// Mirrors `NSScreen.safeAreaInsets.top` for layout (notch camera strip).
    @Published public var notchTopInset: CGFloat = 0

    /// `true` when the built-in display reports a top safe-area inset (physical notch / housing).
    var hasPhysicalNotch: Bool { notchTopInset > 0 }

    /// Collapsed score dot — breathe when elevated risk.
    var shouldPulse: Bool { compositeScore > 0.25 }

    var formattedSessionPnL: String {
        formatDeskMoney(sessionPnL)
    }

    var accountImpact: AccountImpact {
        AccountImpact.compute(
            positions: positions.map { (symbol: $0.symbol, unrealizedPnL: $0.unrealizedPnL) },
            pinnedSymbols: chipCatalog.pinnedNameSymbols,
            dailyLossLimit: dailyLossLimit,
        )
    }

    /// Collapsed macOS strip (#36) — intervention wins; else account impact.
    var collapsedNotchPresentation: CollapsedNotchPresentation {
        CollapsedNotchPresentation.build(
            notch: barLiveState,
            impact: accountImpact,
        )
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
                return "Agent error"
            }
        }
        switch daemonConnectionState {
        case .idle, .connecting:
            return "Connecting"
        case .connected:
            return "Agent connected"
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
            return BarDS.Accent.teal
        case .reconnecting:
            return Color(hex: "#F5A524")
        case .disconnected:
            return Color(hex: "#FF3B30")
        case .idle, .connecting:
            return Color.white.opacity(0.5)
        }
    }

    public var totalUnrealizedPnL: Double { unrealizedTotal }

    var totalExposure: Double { totalExposureApprox }

    var quickWorkflows: [QuickWorkflowItem] { quickWorkflowItems }

    var daemonSecret: String = ""
    var daemonPort: UInt16 = 9137
    /// Loopback wire UUID only (machine integrity). Never Console / brain identity (A8).
    var loopbackWireUserId: String = StationWireClient.loopbackWireUserId
    var webBaseURL: String = "https://localhost:3000"

    private var pollFast: Timer?
    private var pollWorkflows: Timer?
    private var pollBrief: Timer?
    private var pollTrades: Timer?
    private var pollBrokerSync: Timer?
    private var connectionTick: Timer?
    private var lastBriefFetchDay: String?
    private var killSwitchCountdownTimer: Timer?
    private var daemonEventsTask: Task<Void, Never>?
    private var toolbarShowCoalesceTask: Task<Void, Never>?
    private var barLiveStatePollTimer: Timer?
    /// Incremented when `live-state` fails while [hybrid armed](BarOptimisticArmedSnapshot) is active; cleared on 200.
    private var barOptimisticReconcilePollFailures: Int = 0
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
        static let chartTradesKey = "tradeautopsy.notch.journalCapture.chartTradeIds"
    }

    private enum BarOptimisticArmedPersistence {
        static let declarationIdKey = "tradeautopsy.notch.bar.optimisticDeclarationId"
        static let submittedAtKey = "tradeautopsy.notch.bar.optimisticSubmittedAt"
        static let symbolKey = "tradeautopsy.notch.bar.optimisticSymbol"
        static let sideKey = "tradeautopsy.notch.bar.optimisticSide"
        static let qtyKey = "tradeautopsy.notch.bar.optimisticQuantityLabel"
    }

    private let barOptimisticArmedMaxAgeSeconds: TimeInterval = BarOptimisticArmedReconcilePolicy.maxAgeSeconds
    private let barOptimisticArmedMaxPollFailures = BarOptimisticArmedReconcilePolicy.maxPollFailures

    private let barArchetypeStore: BarArchetypeStore
    private let chipCatalogStore: NotchChipCatalogStoring
    private let barDeclareHTTPExecutor: BarDeclareHTTPExecuting
    /// Tests pass a no-op to avoid `fetchBarLiveState()` hitting `URLSession.shared` (#120).
    private let barDeclareSuccessFollowUp: (@MainActor () async -> Void)?

    public convenience init() {
        self.init(
            barArchetypeStore: UserDefaultsBarArchetypeStore(),
            chipCatalogStore: UserDefaultsNotchChipCatalogStore(),
            barDeclareHTTPExecutor: URLSessionBarDeclareHTTPExecutor(),
            barDeclareSuccessFollowUp: nil,
            planSurfaceOnly: false
        )
    }

    init(
        barArchetypeStore: BarArchetypeStore = UserDefaultsBarArchetypeStore(),
        chipCatalogStore: NotchChipCatalogStoring = UserDefaultsNotchChipCatalogStore(),
        barDeclareHTTPExecutor: BarDeclareHTTPExecuting = URLSessionBarDeclareHTTPExecutor(),
        barDeclareSuccessFollowUp: (@MainActor () async -> Void)? = nil,
        planSurfaceOnly: Bool = false
    ) {
        self.barArchetypeStore = barArchetypeStore
        self.chipCatalogStore = chipCatalogStore
        self.barDeclareHTTPExecutor = barDeclareHTTPExecutor
        self.barDeclareSuccessFollowUp = barDeclareSuccessFollowUp
        self.planSurfaceOnly = planSurfaceOnly
        if planSurfaceOnly {
            activeTab = .plan
        }
        chipCatalog = chipCatalogStore.load()
        restoreOptimisticArmedFromDefaults()
        refreshActiveArchetype()
        recomputeBarSurfacePhase()
        reloadUnpostedCaptures()
        loadChartTradeIds()
    }

    func toggleChipExtra(_ id: String) {
        chipCatalog = NotchChipCatalogMutations.toggleExtra(id, in: chipCatalog)
        chipCatalogStore.save(chipCatalog)
    }

    func applyDailyLossLimit(_ value: Double?) {
        if let value, value > 0, value.isFinite {
            dailyLossLimit = value
        } else {
            dailyLossLimit = nil
        }
    }

    func fetchDailyLossLimit() async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/profile/loss-limits") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200 ... 299).contains(code),
                  let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let limits = json["limits"] as? [String: Any]
            else { return }
            if let n = limits["dailyLossLimit"] as? NSNumber {
                applyDailyLossLimit(n.doubleValue)
            } else if let s = limits["dailyLossLimit"] as? String, let v = Double(s) {
                applyDailyLossLimit(v)
            }
        } catch {
            return
        }
    }

    /// Station-hosted Notch: clamp navigation to PLAN (matches approved PLAN-only mock).
    func enablePlanSurfaceOnly() {
        planSurfaceOnly = true
        activeTab = .plan
        dictationUsesCaptureDraft = false
        syncBarLiveStatePollingForVisibility()
    }

    private func refreshActiveArchetype() {
        let pendingKind = barLiveState?.pendingDeclaration?.declarationKind
        let liveArch = barLiveState?.archetype
        let last = barArchetypeStore.loadLastKnownArchetype()
        let resolved = TraderArchetype.resolve(
            pendingDeclarationKind: pendingKind,
            liveArchetype: liveArch,
            lastKnown: last,
        )
        activeArchetype = resolved
        barArchetypeStore.saveLastKnownArchetype(resolved)
    }

    /// Persists archetype tab selection and updates routing for the native declaration shell (#121).
    func setUserDeclarationArchetype(_ archetype: TraderArchetype) {
        barArchetypeStore.saveLastKnownArchetype(archetype)
        activeArchetype = archetype
    }

    private func persistOptimisticArmed(_ snapshot: BarOptimisticArmedSnapshot) {
        let d = UserDefaults.standard
        d.set(snapshot.declarationId, forKey: BarOptimisticArmedPersistence.declarationIdKey)
        d.set(snapshot.submittedAt, forKey: BarOptimisticArmedPersistence.submittedAtKey)
        d.set(snapshot.symbol, forKey: BarOptimisticArmedPersistence.symbolKey)
        d.set(snapshot.side, forKey: BarOptimisticArmedPersistence.sideKey)
        d.set(snapshot.quantityLabel, forKey: BarOptimisticArmedPersistence.qtyKey)
        barOptimisticArmedDisplay = snapshot
        barPublishedDeclarationId = BarDeclarationIdReducer.apply(
            event: .declareSucceeded(snapshot.declarationId),
            state: barPublishedDeclarationId,
        )
    }

    private func clearOptimisticArmedStorage() {
        let d = UserDefaults.standard
        d.removeObject(forKey: BarOptimisticArmedPersistence.declarationIdKey)
        d.removeObject(forKey: BarOptimisticArmedPersistence.submittedAtKey)
        d.removeObject(forKey: BarOptimisticArmedPersistence.symbolKey)
        d.removeObject(forKey: BarOptimisticArmedPersistence.sideKey)
        d.removeObject(forKey: BarOptimisticArmedPersistence.qtyKey)
        barOptimisticArmedDisplay = nil
        barPublishedDeclarationId = BarDeclarationIdReducer.apply(
            event: .declarationClosed,
            state: barPublishedDeclarationId,
        )
        barOptimisticReconcilePollFailures = 0
    }

    private func restoreOptimisticArmedFromDefaults() {
        let d = UserDefaults.standard
        guard let id = d.string(forKey: BarOptimisticArmedPersistence.declarationIdKey), !id.isEmpty,
              let ts = d.object(forKey: BarOptimisticArmedPersistence.submittedAtKey) as? TimeInterval
        else {
            barOptimisticArmedDisplay = nil
            barPublishedDeclarationId = BarDeclarationIdReducer.apply(
                event: .declarationClosed,
                state: barPublishedDeclarationId,
            )
            return
        }
        if Date().timeIntervalSince1970 - ts > barOptimisticArmedMaxAgeSeconds {
            clearOptimisticArmedStorage()
            return
        }
        let sym = d.string(forKey: BarOptimisticArmedPersistence.symbolKey) ?? "—"
        let side = d.string(forKey: BarOptimisticArmedPersistence.sideKey) ?? "—"
        let qty = d.string(forKey: BarOptimisticArmedPersistence.qtyKey) ?? "—"
        barOptimisticArmedDisplay = BarOptimisticArmedSnapshot(
            declarationId: id,
            submittedAt: ts,
            symbol: sym,
            side: side,
            quantityLabel: qty,
        )
        barPublishedDeclarationId = BarDeclarationIdReducer.apply(
            event: .declareSucceeded(id),
            state: barPublishedDeclarationId,
        )
    }

    private func recordOptimisticPollFailure() {
        guard barOptimisticArmedDisplay != nil else { return }
        barOptimisticReconcilePollFailures += 1
    }

    private func resetOptimisticPollFailures() {
        barOptimisticReconcilePollFailures = 0
    }

    private func parseDeclarationPostBodySummary(_ body: Data) -> (symbol: String, side: String, qtyLabel: String)? {
        guard let o = try? JSONSerialization.jsonObject(with: body) as? [String: Any],
              let sym = o["symbol"] as? String,
              let side = o["side"] as? String
        else { return nil }
        let qtyLabel: String
        if let q = o["quantity"] as? Double {
            qtyLabel = q == floor(q) ? String(Int(q)) : String(q)
        } else if let q = o["quantity"] as? Int {
            qtyLabel = String(q)
        } else if let qs = o["quantity"] as? String {
            qtyLabel = qs
        } else { return nil }
        return (sym.uppercased(), upperTrimSide(side), qtyLabel)
    }

    private func upperTrimSide(_ s: String) -> String {
        s.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
    }

    /// §6.6 Policy C (swift slice): while the draft has non-whitespace text, trade UUID + pending toggle are fixed.
    var journalCaptureLinkLocked: Bool {
        !journalCaptureDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func selectTab(_ tab: NotchTab) {
        let resolved = planSurfaceOnly ? NotchTab.plan : tab
        activeTab = resolved
        if resolved != .plan {
            showingDeclarationForm = false
        }
        dictationUsesCaptureDraft = planSurfaceOnly ? false : (resolved == .capture)
        syncBarLiveStatePollingForVisibility()
    }

    /// Starts the 2s live-state timer when the expanded panel shows the PLAN tab; stops otherwise.
    func syncBarLiveStatePollingForVisibility() {
        if isExpanded, activeTab == .plan {
            startBarPolling()
        } else {
            stopBarPolling()
        }
    }

    func startBarPolling() {
        startBarLiveStatePolling()
    }

    func stopBarPolling() {
        stopBarLiveStatePolling()
    }

    func showExitConfirmation() {
        openPlanExitInBrowser()
    }

    func dismissBarDebrief() {
        barDebriefPending = false
        recomputeBarSurfacePhase()
    }

    func resetStopMeFlow() {
        stopMeStep = 0
        stopMeTapCount = 0
        stopMeReason = ""
    }

    var planKillAgentUp: Bool { daemonConnectionState == .connected }

    func presentPlanKillWarning() {
        guard planKillAgentUp else { return }
        planKillPhase = .warning
    }

    func cancelPlanKillWarning() {
        planKillPhase = .idle
    }

    func confirmPlanKill() async {
        await activateKillSwitch()
        planKillPhase = .idle
    }

    func resetCancelDecl() {
        cancelDeclStep = 0
        cancelDeclError = nil
    }

    /// Pending or optimistic declaration id while armed — for cancel (#144).
    var barCancelDeclarationId: String? {
        let pending = barLiveState?.pendingDeclaration?.id.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !pending.isEmpty { return pending }
        if let pub = barPublishedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines), !pub.isEmpty {
            return pub
        }
        let opt = barOptimisticArmedDisplay?.declarationId.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !opt.isEmpty { return opt }
        return nil
    }

    func openPlanExitInBrowser() {
        // Notch is standalone — never open the web dashboard in a browser.
    }

    private func startBarLiveStatePolling() {
        stopBarLiveStatePolling()
        let t = Timer(timeInterval: 2, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.isExpanded, self.activeTab == .plan else { return }
                await self.fetchBarLiveState()
            }
        }
        RunLoop.main.add(t, forMode: .common)
        barLiveStatePollTimer = t
        Task {
            // Defer the first fetch until the summon spring settles — decode work must not
            // land in the first frames of the intro (content is pre-warmed at start()).
            try? await Task.sleep(nanoseconds: 300_000_000)
            guard isExpanded, activeTab == .plan else { return }
            await fetchBarLiveState()
        }
    }

    private func stopBarLiveStatePolling() {
        barLiveStatePollTimer?.invalidate()
        barLiveStatePollTimer = nil
    }

    func fetchBarLiveState() async {
        guard let url = BarLiveStatePollingTarget.url(agentBase: baseURL()) else { return }
        let isFirstFetch = barLiveState == nil
        if isFirstFetch {
            barStateLoading = true
        }
        defer {
            if isFirstFetch {
                barStateLoading = false
            }
        }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard code == 200 else {
                recordOptimisticPollFailure()
                barStateError = BarLiveStateErrorPresentation.message(httpStatus: code, body: data)
                barStateRequiresDeviceLogin = BarLiveStateErrorPresentation.requiresDeviceLogin(body: data)
                recomputeBarSurfacePhase()
                return
            }
            let decoded = try JSONDecoder().decode(BarLiveStateAPIResponse.self, from: data)
            let newState = decoded.notch
            let newFeaturesActive = decoded.barFeaturesActive
            let oldState = barLiveState

            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                if self.daemonProtocolError != nil {
                    self.daemonProtocolError = nil
                }
                withAnimation(.none) {
                    var needsRecompute = false
                    var didPublishMeaningfulData = false

                    if self.barLiveState != newState {
                        self.barLiveState = newState
                        needsRecompute = true
                        didPublishMeaningfulData = true
                        if oldState?.archetype != newState?.archetype {
                            self.refreshActiveArchetype()
                        }
                    }

                    // Hosted `notch.behavioral_score` wins over pulse/SSE — apply every poll (#180).
                    newState?.applyBehavioralToViewModel(self)

                    if let slug = newState?.protectiveBrokerSlug?
                        .trimmingCharacters(in: .whitespacesAndNewlines),
                        !slug.isEmpty
                    {
                        self.barProtectiveBrokerSlug = slug
                    }

                    if self.barFeaturesActiveFromApi != newFeaturesActive {
                        self.barFeaturesActiveFromApi = newFeaturesActive
                        needsRecompute = true
                        didPublishMeaningfulData = true
                        if newFeaturesActive == false {
                            self.clearOptimisticArmedStorage()
                        }
                    }

                    if needsRecompute {
                        self.recomputeBarSurfacePhase()
                    }

                    if didPublishMeaningfulData {
                        self.barLastFetched = Date()
                    }

                    if self.barStateError != nil {
                        self.barStateError = nil
                    }
                    if self.barStateRequiresDeviceLogin {
                        self.barStateRequiresDeviceLogin = false
                    }
                }
                self.resetOptimisticPollFailures()
            }
        } catch {
            recordOptimisticPollFailure()
            barStateError = error.localizedDescription
            barStateRequiresDeviceLogin = false
            recomputeBarSurfacePhase()
        }
    }

    func recomputeBarSurfacePhase() {
        let matchedDeclTrimmed =
            barLiveState?.matchedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if matchedDeclTrimmed.isEmpty {
            stopMeStep = 0
        }
        if barDebriefPending {
            barSurfacePhase = .debrief
            return
        }
        let hasPos = !positions.isEmpty
        if hasPos {
            clearOptimisticArmedStorage()
            barSurfacePhase = .livePlan
            return
        }
        if let pending = barLiveState?.pendingDeclaration, pending.status.uppercased() == "PENDING" {
            clearOptimisticArmedStorage()
            barSurfacePhase = .armed
            return
        }
        if let snap = barOptimisticArmedDisplay {
            if BarOptimisticArmedReconcilePolicy.shouldClearOptimistic(
                snapshot: snap,
                pollFailures: barOptimisticReconcilePollFailures
            ) {
                clearOptimisticArmedStorage()
                barDeclarationConfirmWarning = BarOptimisticArmedReconcilePolicy.confirmWarningMessage
                barSurfacePhase = .declaration
                return
            }
            barSurfacePhase = .armed
            return
        }
        let planRaw = barLiveState?.planState?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let hasPlanSignal = !planRaw.isEmpty
        // Hosted `plan_state` can be set without a matched declaration; native declare UX lives on `.declaration`.
        if hasPlanSignal, !showingDeclarationForm {
            barSurfacePhase = .livePlan
        } else {
            barSurfacePhase = .declaration
        }
    }

    /// PLAN tab — surfaces `declarationFormRoot` immediately (no poll wait).
    func presentBarDeclarationForm() {
        showingDeclarationForm = true
        recomputeBarSurfacePhase()
    }

    func searchSymbols(_ query: String) {
        symbolSearchTask?.cancel()
        if ignoreSymbolSearchUntilEdit {
            ignoreSymbolSearchUntilEdit = false
            return
        }
        guard query.count >= 2 else {
            symbolSuggestions = []
            showSymbolSuggestions = false
            symbolSearchHint = nil
            return
        }
        symbolSearchTask = Task { @MainActor [weak self] in
            guard let self else { return }
            try? await Task.sleep(nanoseconds: 80_000_000)
            guard !Task.isCancelled else { return }
            guard
                let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed),
                let url = URL(string: self.baseURL() + "/instruments/search?q=\(encoded)")
            else { return }
            let req = self.authorizedRequest(url: url)
            do {
                let (data, _) = try await URLSession.shared.data(for: req)
                let resp = try JSONDecoder().decode(InstrumentSearchResponse.self, from: data)
                let filtered = DeskCatalogAllowlist.filterSymbols(
                    resp.symbols,
                    deskSlug: self.resolvedDeskSlug
                )
                self.symbolSuggestions = filtered
                self.showSymbolSuggestions = !filtered.isEmpty
                if filtered.isEmpty {
                    let master = (resp.master_status?.isEmpty == false)
                        ? resp.master_status
                        : self.deskInstrumentsCapability
                    self.symbolSearchHint = DeskCapabilityChrome.emptySearchHint(
                        masterStatus: master,
                        connectedInstrumentDesk: self.connectedInstrumentCatalogDesk
                    )
                } else {
                    self.symbolSearchHint = nil
                }
            } catch {
                // silent — don't surface search errors to user
            }
        }
    }

    func selectSymbol(_ result: InstrumentResult) {
        symbolSearchTask?.cancel()
        ignoreSymbolSearchUntilEdit = true
        if DeskCatalogAllowlist.refusesKotakSelection(result, deskSlug: resolvedDeskSlug) {
            barDeclarationLastError = kotakInstrumentHint
            showSymbolSuggestions = false
            symbolSuggestions = []
            symbolSearchHint = nil
            return
        }
        guard let ticker = BarBrokerTicker.normalize(raw: result.trading_symbol) else {
            barDeclarationLastError = "Symbol must be a broker ticker (e.g. RELIANCE), not a company name."
            showSymbolSuggestions = false
            symbolSuggestions = []
            symbolSearchHint = nil
            return
        }
        let kotakDesk = BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
        let tickId = result.tickBookInstrumentId(for: declareAssetClass, deskSlug: resolvedDeskSlug)
        if kotakDesk, declareAssetClass == .options, tickId == nil {
            barDeclarationLastError = kotakInstrumentHint
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
            showSymbolSuggestions = false
            symbolSuggestions = []
            symbolSearchHint = nil
            return
        }
        if kotakDesk, tickId == nil {
            barDeclarationLastError = kotakInstrumentHint
            showSymbolSuggestions = false
            symbolSuggestions = []
            symbolSearchHint = nil
            return
        }
        barDeclarationSymbol = ticker
        barDeclarationLastError = nil
        barLtpFetchError = nil
        showSymbolSuggestions = false
        symbolSuggestions = []
        symbolSearchHint = nil
        if kotakDesk, let tickId {
            deskSelectedInstrumentId = tickId
            if result.last_price > 0 {
                applyLTP(result.last_price)
            }
            fetchStationQuote(instrument: tickId)
            refreshDeskExtracts(symbol: chainUnderlying(from: result, ticker: ticker), instrumentId: tickId)
            return
        }
        let exchangeNorm = result.exchange.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug)
            || exchangeNorm == "binance_com"
            || exchangeNorm == "binance"
        {
            deskSelectedInstrumentId = ticker
            if declareAssetClass == .options {
                deskLastStatus = "unavailable"
                deskQuoteCapability = "unavailable"
                refreshDeskExtracts(symbol: ticker, instrumentId: ticker)
                return
            }
            if result.last_price > 0 {
                applyLTP(result.last_price)
            }
            fetchStationQuote(instrument: ticker)
            refreshDeskExtracts(symbol: ticker, instrumentId: ticker)
            return
        }
        if kotakDesk, declareAssetClass == .options {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
            return
        }
        deskSelectedInstrumentId = result.tickBookInstrumentId ?? ticker
        if result.last_price > 0 {
            applyLTP(result.last_price)
        }
        fetchLTP(
            symbol: ticker,
            exchange: result.exchange,
            segment: result.segment ?? result.exchange
        )
        refreshDeskExtracts(symbol: chainUnderlying(from: result, ticker: ticker), instrumentId: deskSelectedInstrumentId)
    }

    private var kotakInstrumentHint: String {
        declareAssetClass == .options
            ? "Select a Kotak NFO instrument"
            : "Select a Kotak cash instrument"
    }

    /// NFO chain instrument is `pSymbolName` (BANKNIFTY / NIFTY), not a cash token or trading-symbol.
    /// Binance options keep the ticker — `deskBookId` is nil, so `book=` is omitted.
    private func chainUnderlying(from result: InstrumentResult, ticker: String) -> String {
        guard declareAssetClass == .options,
              BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
        else { return ticker }
        let name = result.name.trimmingCharacters(in: .whitespacesAndNewlines)
        return name.isEmpty ? ticker : name
    }

    /// Active catalog slug for desk-honest Last/history routing.
    var resolvedDeskSlug: String? {
        let raw = (activeBrokerSlug ?? barProtectiveBrokerSlug)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return raw.isEmpty ? nil : raw
    }

    /// Live Kotak/Binance desk — empty slug still counts when session is up (bridge defaults kotak_neo).
    var connectedInstrumentCatalogDesk: Bool {
        let connected = brokerSessionActive || brokerSyncClass == "syncing"
        guard connected else { return false }
        if BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
            || DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug)
        {
            return true
        }
        if resolvedDeskSlug == nil {
            return BarDeskTemplate.isKotakNeoDesk(slug: resolvedBrokerSlugForBridge)
                || DeskCatalogAllowlist.isBinanceDesk(resolvedBrokerSlugForBridge)
        }
        return false
    }

    func fetchLTP(symbol: String, exchange: String, segment: String) {
        let exchangeNorm = exchange.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if declareAssetClass == .options {
            return
        }
        if BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug) {
            return
        }
        if DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug), exchangeNorm == "nse" {
            return
        }
        let sym = InstrumentTickBookId.queryEncode(symbol)
        guard
            let exc = exchange.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed),
            let seg = segment.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed),
            let url = URL(string: "\(baseURL())/instruments/ltp?symbol=\(sym)&exchange=\(exc)&segment=\(seg)")
        else { return }
        Task {
            let req = authorizedRequest(url: url)
            do {
                let (data, _) = try await URLSession.shared.data(for: req)
                let resp = try JSONDecoder().decode(LTPResponse.self, from: data)
                await MainActor.run {
                    if resp.error == "session_expired" {
                        barLtpFetchError = "Reconnect broker to load live price"
                        return
                    }
                    barLtpFetchError = nil
                    if let ltp = resp.ltp, ltp > 0 {
                        applyLTP(ltp)
                        deskLastStatus = resp.quote_status ?? resp.source ?? "fresh"
                    } else {
                        deskLastStatus = resp.quote_status ?? resp.source ?? "unavailable"
                    }
                }
            } catch {
                // silent — LTP failure must not block declaration
            }
        }
    }

    func fetchStationQuote(instrument: String) {
        if !shouldBindQuoteLast(adapter: nil, instrumentId: instrument) {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
            return
        }
        let encoded = InstrumentTickBookId.queryEncode(instrument)
        guard let url = URL(string: "\(baseURL())/api/station/quote?instrument=\(encoded)") else {
            return
        }
        Task {
            let req = authorizedRequest(url: url)
            do {
                let (data, _) = try await URLSession.shared.data(for: req)
                guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                    return
                }
                await MainActor.run {
                    applyStationQuoteEnvelope(json)
                }
            } catch {
                // silent — quote failure must not block declaration
            }
        }
    }

    /// Bind Last from a Station quote extract (testable without network).
    func applyStationQuoteEnvelope(_ json: [String: Any]) {
        let adapter = ((json["provenance"] as? [String: Any])?["adapter_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        let instrumentId = (json["instrument_id"] as? String) ?? ""
        if !shouldBindQuoteLast(adapter: adapter, instrumentId: instrumentId) {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
            return
        }
        let status = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? "unavailable"
        if status == "unavailable" {
            let ineligible = stringList(json["ineligible"])
            if ineligible.contains("quotes_http") {
                deskLastStatus = "quotes_http"
            } else if ineligible.contains("session") {
                deskLastStatus = "session"
            } else if ineligible.contains("quotes_unusable") {
                deskLastStatus = "quotes_unusable"
            } else {
                deskLastStatus = status
            }
        } else {
            deskLastStatus = status
        }
        deskQuoteCapability = deskLastStatus
        if let data = json["data"] as? [String: Any], let last = Self.parseQuoteLast(data), last > 0 {
            barLtpFetchError = nil
            applyLTP(last)
        }
    }

    /// Options last is NFO TickBook only. Binance spot / eapi last must not paint options declare.
    func shouldBindQuoteLast(adapter: String?, instrumentId: String) -> Bool {
        if declareAssetClass == .options {
            if DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug) {
                return false
            }
            if adapter == "binance_com" || adapter == "binance" {
                return false
            }
            guard BarDeskTemplate.isKotakNfoDesk(slug: resolvedDeskSlug, assetClass: declareAssetClass) else {
                return false
            }
            return InstrumentTickBookId.isNfoIdentity(instrumentId)
        }
        if BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug),
           adapter == "binance_com" || adapter == "binance"
        {
            return false
        }
        return true
    }

    func applyLTP(_ ltp: Double) {
        if declareAssetClass == .options {
            if DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug) { return }
            if !BarDeskTemplate.isKotakNfoDesk(slug: resolvedDeskSlug, assetClass: declareAssetClass) {
                return
            }
        }
        if declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            || declEntryPrice == "0"
        {
            declEntryPrice = String(format: "%.2f", ltp)
        }
    }

    private func reconcileDeskLastForAssetClass() {
        if declareAssetClass == .options {
            if shouldBindQuoteLast(adapter: nil, instrumentId: deskSelectedInstrumentId) {
                fetchStationQuote(instrument: deskSelectedInstrumentId)
            } else {
                deskLastStatus = "unavailable"
                deskQuoteCapability = "unavailable"
            }
            refreshDeskExtracts(symbol: barDeclarationSymbol, instrumentId: deskSelectedInstrumentId)
            return
        }
        if InstrumentTickBookId.isNfoIdentity(deskSelectedInstrumentId) {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
        }
        refreshDeskExtracts(symbol: barDeclarationSymbol, instrumentId: deskSelectedInstrumentId)
    }

    static func parseQuoteLast(_ data: [String: Any]) -> Double? {
        if let s = data["last"] as? String {
            return Double(s)
        }
        if let d = data["last"] as? Double {
            return d
        }
        if let i = data["last"] as? Int {
            return Double(i)
        }
        return nil
    }

    /// Bind History from a Station history extract (testable without network).
    /// Pre-trade stays a status string — never candles, never Yahoo stitch.
    func applyStationHistoryEnvelope(_ json: [String: Any]) {
        deskYahooHistoryStatus = "unavailable"
        deskYahooHistoryIneligible = []

        if BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug) {
            deskHistoryStatus = "unsupported"
            deskHistoryIneligible = []
            return
        }

        let adapter = ((json["provenance"] as? [String: Any])?["adapter_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let dataSource = ((json["data"] as? [String: Any])?["source"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let status = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? "unavailable"
        if adapter == "yahoo" || adapter == "yahoo_chart"
            || dataSource == "yahoo" || dataSource == "yahoo_chart"
            || status == "research_segment"
        {
            return
        }

        deskHistoryStatus = status
        deskHistoryIneligible = stringList(json["ineligible"]).filter {
            $0 != "rights_forbid_canonical"
        }
    }

    /// Testable chain glance path. `book=` comes from the current declare class; instrument is the typed underlying.
    func deskChainExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.path(
            bookId: BarDeskTemplate.deskBookId(slug: resolvedDeskSlug, assetClass: declareAssetClass),
            underlying: DeskChainExtractQuery.underlyingTicker(
                preferred: symbol,
                declarationSymbol: barDeclarationSymbol
            )
        )
    }

    /// Same `book=` + underlying as chain. Do not send a TickBook token as the OI instrument.
    func deskOiExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.oiPath(
            bookId: BarDeskTemplate.deskBookId(slug: resolvedDeskSlug, assetClass: declareAssetClass),
            underlying: DeskChainExtractQuery.underlyingTicker(
                preferred: symbol,
                declarationSymbol: barDeclarationSymbol
            )
        )
    }

    func refreshDeskExtracts(symbol: String, instrumentId: String? = nil) {
        let selected = deskSelectedInstrumentId
        let raw: String
        if let instrumentId, !instrumentId.isEmpty {
            raw = instrumentId
        } else if !selected.isEmpty {
            raw = selected
        } else {
            raw = symbol
        }
        let encoded = InstrumentTickBookId.queryEncode(raw)
        let kotak = BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
        let chainPath = deskChainExtractPath(symbol: symbol)
        let oiPath = deskOiExtractPath(symbol: symbol)
        Task { [weak self] in
            guard let self else { return }
            async let chain = self.getExtractJSON(chainPath)
            async let oi = self.getExtractJSON(oiPath)
            let licensedJSON: [String: Any]?
            if kotak {
                licensedJSON = await self.getExtractJSON(
                    "/api/station/obtain?adapter=kotak_neo&operation=history"
                )
            } else {
                licensedJSON = await self.getExtractJSON(
                    "/api/station/history?instrument=\(encoded)"
                )
            }
            let chainJSON = await chain
            let oiJSON = await oi
            await MainActor.run {
                self.applyStationHistoryEnvelope(licensedJSON ?? [:])
                self.deskChainStatus = chainJSON?["status"] as? String ?? "unavailable"
                self.deskOiStatus = oiJSON?["status"] as? String ?? "unavailable"
            }
        }
    }

    private func getExtractJSON(_ pathAndQuery: String) async -> [String: Any]? {
        guard let url = URL(string: baseURL() + pathAndQuery) else { return nil }
        let req = authorizedRequest(url: url)
        do {
            let (data, _) = try await URLSession.shared.data(for: req)
            return try JSONSerialization.jsonObject(with: data) as? [String: Any]
        } catch {
            return nil
        }
    }

    private func stringList(_ raw: Any?) -> [String] {
        (raw as? [Any])?.compactMap { $0 as? String } ?? []
    }

    func dismissSymbolSuggestions() {
        symbolSearchTask?.cancel()
        symbolSearchTask = nil
        ignoreSymbolSearchUntilEdit = false
        showSymbolSuggestions = false
        symbolSuggestions = []
        symbolSearchHint = nil
    }

    /// Broker-backed defaults for the declare form — wire ticker + open qty, not display names.
    func barDeclarationPrefillDefaults() -> (symbol: String, sideBuy: Bool, quantity: String)? {
        if let pos = positions.first(where: { $0.qty != 0 }) {
            guard let sym = BarBrokerTicker.normalize(raw: pos.symbol), !sym.isEmpty else { return nil }
            let sideBuy = !pos.direction.uppercased().contains("SELL")
            return (sym, sideBuy, String(abs(pos.qty)))
        }
        if let trade = recentTrades.first {
            guard let sym = BarBrokerTicker.normalize(raw: trade.symbol), !sym.isEmpty else { return nil }
            let sideBuy = !trade.side.uppercased().contains("SELL")
            let qty = trade.qty > 0 ? String(trade.qty) : ""
            return (sym, sideBuy, qty)
        }
        return nil
    }

    private struct BarDeclareOkResponse: Decodable {
        let ok: Bool?
        let declarationId: String?
    }

    func submitBarDeclaration(body: Data) async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/declare") else { return }
        let summary = parseDeclarationPostBodySummary(body)
        barDeclarationBusy = true
        barDeclarationLastError = nil
        barDeclarationConfirmWarning = nil
        defer { barDeclarationBusy = false }
        do {
            let (data, resp) =
                try await barDeclareHTTPExecutor.data(for: authorizedRequest(url: url, method: "POST", body: body))
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            if (200...299).contains(code) {
                showingDeclarationForm = false
                daemonProtocolError = nil
                barStateError = nil
                barDeclarationLastError = nil
                barDeclarationConfirmWarning = nil
                if let parsed = try? JSONDecoder().decode(BarDeclareOkResponse.self, from: data),
                   let declId = parsed.declarationId, !declId.isEmpty
                {
                    let now = Date().timeIntervalSince1970
                    let snap = BarOptimisticArmedSnapshot(
                        declarationId: declId,
                        submittedAt: now,
                        symbol: summary?.symbol ?? "—",
                        side: summary?.side ?? "—",
                        quantityLabel: summary?.qtyLabel ?? "—",
                    )
                    persistOptimisticArmed(snap)
                    barOptimisticReconcilePollFailures = 0
                }
                recomputeBarSurfacePhase()
                if let follow = barDeclareSuccessFollowUp {
                    await follow()
                } else {
                    await fetchBarLiveState()
                }
            } else {
                barDeclarationLastError = BarDeclareHTTPErrorPresentation.message(httpStatus: code, body: data)
                clearOptimisticArmedStorage()
                recomputeBarSurfacePhase()
            }
        } catch {
            barDeclarationLastError = error.localizedDescription
            recomputeBarSurfacePhase()
        }
    }

    private func notePositionTransitionForBar(previousCount: Int, newCount: Int) {
        barDebriefPending = BarDebriefArming.applyPoll(
            previousCount: previousCount,
            newCount: newCount,
            pending: barDebriefPending,
        )
        if newCount > 0 {
            clearOptimisticArmedStorage()
        }
    }

    func submitBarStopMe() async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/stop-me") else { return }
        let ms = Int(Date().timeIntervalSince1970 * 1000)
        let reason = stopMeReason.isEmpty ? "notch_stop_me" : stopMeReason
        let body: [String: Any] = [
            "reason": reason,
            "triggered_at_ms": ms,
        ]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        barStopMeBusy = true
        defer { barStopMeBusy = false }
        do {
            let (_, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url, method: "POST", body: payload))
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                barStateError = "Stop me failed (\(code))"
                return
            }
            daemonProtocolError = nil
            resetStopMeFlow()
            killSwitchActive = true
            recordKillSwitchStateReceived()
            barStateError = nil
        } catch {
            barStateError = error.localizedDescription
        }
    }

    func clearBarStopMeKillSwitch() async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/stop-me/clear") else { return }
        let ms = Int(Date().timeIntervalSince1970 * 1000)
        let body: [String: Any] = ["released_at_ms": ms]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        barStopMeClearBusy = true
        defer { barStopMeClearBusy = false }
        do {
            let (_, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url, method: "POST", body: payload))
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                barDeclarationLastError = "Could not resume trading (\(code))"
                return
            }
            daemonProtocolError = nil
            killSwitchActive = false
            recordKillSwitchStateReceived()
            barDeclarationLastError = nil
            barStateError = nil
            await fetchBarLiveState()
        } catch {
            barDeclarationLastError = error.localizedDescription
        }
    }

    private struct BarCancelErrResponse: Decodable {
        struct ErrDetail: Decodable {
            let code: String?
        }

        let error: ErrDetail?
    }

    func submitCancelDeclaration(declarationId: String, reasonChip: String) async {
        guard cancelDeclStep == 1 else { return }
        cancelDeclStep = 2
        cancelDeclError = nil
        guard let url = URL(string: baseURL() + "/api/daemon/bar/cancel-declaration") else {
            cancelDeclStep = 0
            cancelDeclError = "Could not cancel — try again"
            return
        }
        let trimmedId = declarationId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else {
            cancelDeclStep = 0
            cancelDeclError = "Could not cancel — try again"
            return
        }
        let body: [String: Any] = [
            "declaration_id": trimmedId,
            "cancel_reason_chip": reasonChip,
        ]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else {
            cancelDeclStep = 0
            cancelDeclError = "Could not cancel — try again"
            return
        }
        do {
            let (data, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: payload),
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            if (200...299).contains(code) {
                daemonProtocolError = nil
                cancelDeclStep = 0
                clearOptimisticArmedStorage()
                recomputeBarSurfacePhase()
                await fetchBarLiveState()
                return
            }
            if code == 409,
               let parsed = try? JSONDecoder().decode(BarCancelErrResponse.self, from: data),
               parsed.error?.code == "declaration_cancel_blocked"
            {
                cancelDeclError = "SL order active — cancel SL first"
                cancelDeclStep = 0
                return
            }
            cancelDeclError = "Could not cancel — try again"
            cancelDeclStep = 0
        } catch {
            cancelDeclError = "Could not cancel — try again"
            cancelDeclStep = 0
        }
    }

    /// Live-trade interference chips — POSTs choice to hosted `ingestSignal` via agent.
    func applyBarInterferenceTap(_ choice: String) {
        barInterferenceChoice = choice
        let planRed = BarInterferenceEchoReducer.planIsRed(
            planState: barLiveState?.planState,
            isRedTerminal: barLiveState?.isRedTerminal ?? false,
        )
        let c = choice.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch c {
        case "no":
            barInterferenceEcho = "Good. Nothing to act on. Let the plan run."
        case "maybe":
            barInterferenceEcho =
                "Uncertainty is not a reason to act. If no rule triggered, do nothing. Write the feeling below."
        case "yes":
            barInterferenceEcho =
                planRed
                ? "Your thesis is already dead. This exit is the plan. Execute it."
                : "Your time invalidation is approaching. Do not add risk — write the impulse below and wait."
        default:
            barInterferenceEcho = nil
        }
        if choice == "no" {
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: 3_000_000_000)
                if barInterferenceChoice == "no" { barInterferenceEcho = nil }
            }
        }
        Task { await submitBarLiveInterference(choice: choice) }
    }

    /// Footer “Hold and watch” — dismisses interference UI and logs the same path as “No — following plan” (#113).
    func holdAndWatchBarLive() {
        barInterferenceChoice = nil
        barInterferenceEcho = nil
        Task { await submitBarLiveInterference(choice: "no") }
    }

    func submitBarLiveInterference(choice: String) async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/live-interference") else { return }
        let planRaw = barLiveState?.planState?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let declFromMatch = barLiveState?.matchedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines)
        let declPending = barLiveState?.pendingDeclaration?.id.trimmingCharacters(in: .whitespacesAndNewlines)
        let declId: String? = {
            if let m = declFromMatch, !m.isEmpty { return m }
            if let p = declPending, !p.isEmpty { return p }
            return nil
        }()
        var body: [String: Any] = [
            "choice": choice,
            "at_ms": Int(Date().timeIntervalSince1970 * 1000),
        ]
        if !planRaw.isEmpty {
            body["plan_state"] = planRaw
        }
        if let declId {
            body["declaration_id"] = declId
        }
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: payload),
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                barDeclarationLastError = AgentHTTPErrorPresentation.message(httpStatus: code, body: data)
                return
            }
            daemonProtocolError = nil
            barDeclarationLastError = nil
        } catch {
            barDeclarationLastError = error.localizedDescription
        }
    }

    /// Forward modify/cancel protective SL JSON to hosted engine via agent (`ActiveSLRecord` body).
    /// T5 K3 — PLAN must not call this with `action: place_sl`.
    func submitBarProtective(body: Data) async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/protective") else { return }
        barProtectiveBusy = true
        defer { barProtectiveBusy = false }
        do {
            let (data, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: body),
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                barDeclarationLastError = AgentHTTPErrorPresentation.message(httpStatus: code, body: data)
                return
            }
            daemonProtocolError = nil
            barDeclarationLastError = nil
            await fetchBarLiveState()
        } catch {
            barDeclarationLastError = error.localizedDescription
        }
    }

    /// #5 — forward full JSON to hosted `ingestSignal` path (agent unwraps daemon auth).
    func submitBarPostTradeDebrief(body: Data) async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/post-trade-debrief") else { return }
        barPostTradeDebriefBusy = true
        barPostTradeDebriefLastError = nil
        defer { barPostTradeDebriefBusy = false }
        do {
            let (data, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "PATCH", body: body),
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                barPostTradeDebriefLastError = AgentHTTPErrorPresentation.message(httpStatus: code, body: data)
                return
            }
            daemonProtocolError = nil
            barPostTradeDebriefLastError = nil
            dismissBarDebrief()
        } catch {
            barPostTradeDebriefLastError = error.localizedDescription
        }
    }

    /// #4 — swing capitulation / thesis check-in → `bar_notch_swing_check_in`.
    func submitSwingCheckIn(body: Data) async {
        guard let url = URL(string: baseURL() + "/api/daemon/bar/swing-check-in") else { return }
        barSwingCheckInBusy = true
        barSwingCheckInLastError = nil
        defer { barSwingCheckInBusy = false }
        do {
            let (data, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: body),
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                barSwingCheckInLastError = AgentHTTPErrorPresentation.message(httpStatus: code, body: data)
                return
            }
            daemonProtocolError = nil
            barSwingCheckInLastError = nil
            await fetchBarLiveState()
        } catch {
            barSwingCheckInLastError = error.localizedDescription
        }
    }

    func persistJournalCaptureDraftLocally() {
        writeJournalCaptureDefaults()
        journalCaptureLastError = nil
        journalCaptureLastSuccess = "Draft saved locally."
    }

    func reloadUnpostedCaptures() {
        unpostedSweepDropped = unpostedStore.sweep()
        unpostedCaptures = unpostedStore.all()
    }

    var capturePhaseForJournal: String {
        switch barSurfacePhase {
        case .declaration, .armed: return "pre"
        case .livePlan: return "during"
        case .debrief: return "post"
        }
    }

    func ingestImageData(_ data: Data, hintedType: String?) {
        do {
            let encoded = try JournalImageEncoder.encode(data, hintedType: hintedType)
            stagedCaptureData = encoded.data
            stagedCaptureType = encoded.contentType
            stagedCapturePreview = NSImage(data: encoded.data)
            journalCaptureScreenshotError = nil
            noteCaptureStaged()
            Task { await notchHost?.expandToCapture() }
        } catch JournalImageEncoderError.unsupportedType {
            journalCaptureScreenshotError = "Paste a PNG or JPEG."
        } catch JournalImageEncoderError.tooLarge {
            journalCaptureScreenshotError = "Image too large."
        } catch {
            journalCaptureScreenshotError = error.localizedDescription
        }
    }

    func ingestPastedImage() {
        let pb = NSPasteboard.general
        if let types = pb.types, types.contains(.png), let data = pb.data(forType: .png) {
            ingestImageData(data, hintedType: "image/png")
            return
        }
        if let types = pb.types, types.contains(.tiff), let data = pb.data(forType: .tiff) {
            ingestImageData(data, hintedType: "image/tiff")
            return
        }
        if let data = pb.data(forType: NSPasteboard.PasteboardType("public.jpeg")) {
            ingestImageData(data, hintedType: "image/jpeg")
            return
        }
        if let url = pb.readObjects(forClasses: [NSURL.self], options: nil)?.first as? URL {
            let ext = url.pathExtension.lowercased()
            guard ext == "png" || ext == "jpg" || ext == "jpeg" else {
                journalCaptureScreenshotError = "Paste a PNG or JPEG."
                return
            }
            guard let data = try? Data(contentsOf: url) else { return }
            ingestImageData(data, hintedType: ext == "png" ? "image/png" : "image/jpeg")
            return
        }
        journalCaptureScreenshotError = "Paste a PNG or JPEG."
    }

    func ingestPastedImageIfPresent() {
        guard CapturePasteboard.containsImage(NSPasteboard.general) else { return }
        ingestPastedImage()
    }

    func startCapturePasteboardWatch() {
        stopCapturePasteboardWatch()
        capturePasteboardChangeCount = NSPasteboard.general.changeCount
        let t = Timer(timeInterval: 0.5, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                let pb = NSPasteboard.general
                let count = pb.changeCount
                guard count != self.capturePasteboardChangeCount else { return }
                self.capturePasteboardChangeCount = count
                self.ingestPastedImageIfPresent()
            }
        }
        RunLoop.main.add(t, forMode: .common)
        capturePasteboardWatch = t
    }

    func stopCapturePasteboardWatch() {
        capturePasteboardWatch?.invalidate()
        capturePasteboardWatch = nil
    }

    func stageScreenshotFromRegion() async {
        journalCaptureScreenshotBusy = true
        journalCaptureScreenshotError = nil
        defer { journalCaptureScreenshotBusy = false }
        notchHost?.hideChromeForInteractiveCapture()
        try? await Task.sleep(nanoseconds: 200_000_000)
        do {
            let captureURL = try await JournalInteractiveScreenshot.captureRegionToTempPNG()
            let data = try Data(contentsOf: captureURL)
            JournalInteractiveScreenshot.removeTempFile(at: captureURL)
            ingestImageData(data, hintedType: "image/png")
            notchHost?.restoreChromeAfterInteractiveCapture()
        } catch let err as JournalInteractiveScreenshotError {
            journalCaptureScreenshotError = err.userMessage
            notchHost?.restoreChromeAfterInteractiveCapture()
        } catch {
            journalCaptureScreenshotError = error.localizedDescription
            notchHost?.restoreChromeAfterInteractiveCapture()
        }
    }

    @discardableResult
    func keepStagedCaptureInTray() -> UnpostedCaptureRecord? {
        guard let data = stagedCaptureData else { return nil }
        do {
            let rec = try unpostedStore.insert(
                imageData: data,
                contentType: stagedCaptureType,
                caption: journalCaptureDraft.trimmingCharacters(in: .whitespacesAndNewlines),
                capturePhase: capturePhaseForJournal
            )
            stagedCaptureData = nil
            stagedCapturePreview = nil
            reloadUnpostedCaptures()
            journalCaptureLastSuccess = "Kept on this Mac"
            journalCaptureLastError = nil
            return rec
        } catch {
            journalCaptureLastError = error.localizedDescription
            return nil
        }
    }

    func deleteUnpostedCapture(_ id: UUID) {
        unpostedStore.delete(id: id)
        reloadUnpostedCaptures()
    }

    func unpostedFileURL(for rec: UnpostedCaptureRecord) -> URL {
        unpostedStore.fileURL(for: rec)
    }

    var linkableRecentTrades: [RecentTradeRow] {
        let q = unpostedSearch.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if q.isEmpty { return recentTrades }
        return recentTrades.filter {
            $0.symbol.lowercased().contains(q) || $0.id.lowercased().contains(q)
        }
    }

    func sendUnpostedCapture(_ id: UUID, tradeId: String) async {
        if let msg = JournalCaptureLinkValidator.validationMessage(
            draftTrimmed: "",
            explicitPending: false,
            tradeRawTrimmed: tradeId,
            imageOnly: true
        ) {
            if let rec = unpostedCaptures.first(where: { $0.id == id }) {
                var next = rec
                next.lastError = msg
                unpostedStore.update(next)
                reloadUnpostedCaptures()
            }
            return
        }
        guard let rec = unpostedCaptures.first(where: { $0.id == id }) else { return }
        let fileURL = unpostedStore.fileURL(for: rec)
        guard let pngData = try? Data(contentsOf: fileURL) else {
            var next = rec
            next.lastError = "Local file missing."
            unpostedStore.update(next)
            reloadUnpostedCaptures()
            return
        }
        unpostedSendBusyId = id
        defer { unpostedSendBusyId = nil }

        do {
            try await uploadScreenshotToTrade(
                imageData: pngData,
                contentType: rec.contentType,
                caption: rec.caption,
                tradeId: tradeId,
                existing: rec
            )
            unpostedStore.delete(id: id)
            reloadUnpostedCaptures()
            tradeIdsWithChart.insert(tradeId)
            persistChartTradeIds()
            journalCaptureLastSuccess = "On the trade note"
            journalCaptureLastError = nil
        } catch {
            var next = rec
            next.lastError = error.localizedDescription
            unpostedStore.update(next)
            reloadUnpostedCaptures()
        }
    }

    func sendStagedCapture(tradeId: String) async {
        guard keepStagedCaptureInTray() != nil else { return }
        guard let latest = unpostedCaptures.first else { return }
        await sendUnpostedCapture(latest.id, tradeId: tradeId)
    }

    func tradeHasChart(_ tradeId: String) -> Bool {
        tradeIdsWithChart.contains(tradeId)
    }

    /// Tap a today's-trade row: send the latest unposted shot, or confirm replace if that trade already has one.
    func requestLinkUnposted(to tradeId: String) {
        guard unpostedCaptures.first != nil else { return }
        if tradeIdsWithChart.contains(tradeId) {
            replaceConfirmTradeId = tradeId
            return
        }
        Task { await sendLatestUnposted(to: tradeId) }
    }

    func confirmReplaceChart() async {
        guard let tid = replaceConfirmTradeId else { return }
        replaceConfirmTradeId = nil
        await sendLatestUnposted(to: tid)
    }

    func cancelReplaceChart() {
        replaceConfirmTradeId = nil
    }

    func sendLatestUnposted(to tradeId: String) async {
        guard let latest = unpostedCaptures.first else { return }
        await sendUnpostedCapture(latest.id, tradeId: tradeId)
    }

    /// PLAN paste/shutter: jump to Live trade unless debrief is showing Post-trade.
    func consumeLiveCaptureScreenRequest() -> Bool {
        guard requestLiveCaptureScreen else { return false }
        requestLiveCaptureScreen = false
        return barSurfacePhase != .debrief
    }

    private func noteCaptureStaged() {
        if planSurfaceOnly {
            dictationUsesCaptureDraft = true
            requestLiveCaptureScreen = true
        }
    }

    private func loadChartTradeIds() {
        let arr = UserDefaults.standard.stringArray(forKey: JournalCapturePersistence.chartTradesKey) ?? []
        tradeIdsWithChart = Set(arr)
    }

    private func persistChartTradeIds() {
        UserDefaults.standard.set(Array(tradeIdsWithChart), forKey: JournalCapturePersistence.chartTradesKey)
    }

    private func uploadScreenshotToTrade(
        imageData: Data,
        contentType: String,
        caption: String,
        tradeId: String,
        existing: UnpostedCaptureRecord
    ) async throws {
        guard isAuthenticated && (sessionState == "active" || sessionState == "expiring_soon") else {
            throw NSError(domain: "Notch", code: 401, userInfo: [NSLocalizedDescriptionKey: "Sign in required to finalize captures."])
        }
        let idem = existing.idempotencyKey?.isEmpty == false ? existing.idempotencyKey! : StationWireClient.makeULID()
        var pendingId = existing.consolePendingId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if pendingId.isEmpty {
            pendingId = try await acceptImageOnlyCapture(caption: caption, tradeId: tradeId, idempotencyKey: idem)
            var next = existing
            next.consolePendingId = pendingId
            next.idempotencyKey = idem
            unpostedStore.update(next)
        }
        let (uploadURL, r2Key) = try await presignScreenshot(pendingId: pendingId, contentType: contentType)
        let putCode = try await putPngToPresignedUrlWithRetry(
            uploadURL: uploadURL,
            pngData: imageData,
            contentType: contentType
        )
        guard (200 ... 299).contains(putCode) else {
            throw NSError(domain: "Notch", code: putCode, userInfo: [NSLocalizedDescriptionKey: "Upload failed (\(putCode))"])
        }
        try await patchPendingR2Key(pendingId: pendingId, r2Key: r2Key)
        try await waitUntilPendingFinalized(pendingId: pendingId)
    }

    private func acceptImageOnlyCapture(caption: String, tradeId: String, idempotencyKey: String) async throws -> String {
        guard let url = URL(string: baseURL() + "/api/daemon/journal/toolbar-capture/accept") else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Bad daemon URL"])
        }
        let body: [String: Any] = [
            "draftText": caption,
            "imageOnly": true,
            "tradeId": tradeId,
            "explicitPending": false,
            "idempotencyKey": idempotencyKey,
        ]
        let payload = try JSONSerialization.data(withJSONObject: body)
        let req = authorizedRequest(url: url, method: "POST", body: payload)
        let (data, resp) = try await URLSession.shared.data(for: req)
        let status = (resp as? HTTPURLResponse)?.statusCode ?? 0
        guard status == 200 else {
            handleDaemonErrorResponse(data: data, statusCode: status)
            throw NSError(domain: "Notch", code: status, userInfo: [NSLocalizedDescriptionKey: daemonProtocolError.map(\.rawValue) ?? "Finalize failed (\(status))"])
        }
        guard
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any],
            let ok = j["success"] as? Bool, ok,
            let inner = j["data"] as? [String: Any],
            let pendingId = inner["pending_capture_id"] as? String
        else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Unexpected server response"])
        }
        return pendingId
    }

    private func presignScreenshot(pendingId: String, contentType: String) async throws -> (URL, String) {
        guard let url = URL(string: baseURL() + "/api/daemon/screenshot/presign") else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Bad daemon URL"])
        }
        let ext = contentType == "image/jpeg" ? "jpg" : "png"
        let body: [String: Any] = [
            "pending_capture_id": pendingId,
            "content_type": contentType,
            "filename": "toolbar-\(Int(Date().timeIntervalSince1970)).\(ext)",
        ]
        let payload = try JSONSerialization.data(withJSONObject: body)
        let req = authorizedRequest(url: url, method: "POST", body: payload)
        let (data, resp) = try await URLSession.shared.data(for: req)
        let status = (resp as? HTTPURLResponse)?.statusCode ?? 0
        guard status == 200 else {
            throw NSError(domain: "Notch", code: status, userInfo: [NSLocalizedDescriptionKey: screenshotFlowProtocolMessage(data: data, statusCode: status)])
        }
        guard
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any],
            let ok = j["success"] as? Bool, ok,
            let inner = j["data"] as? [String: Any],
            let uploadUrlStr = inner["uploadUrl"] as? String,
            let uploadURL = URL(string: uploadUrlStr),
            let r2Key = inner["key"] as? String
        else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Unexpected presign response"])
        }
        return (uploadURL, r2Key)
    }

    private func patchPendingR2Key(pendingId: String, r2Key: String) async throws {
        let patchPath = "/api/daemon/journal/toolbar-capture/pending/" + pendingId
        guard let url = URL(string: baseURL() + patchPath) else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Bad daemon URL"])
        }
        let payload = try JSONSerialization.data(withJSONObject: ["r2_key": r2Key])
        let req = authorizedRequest(url: url, method: "PATCH", body: payload)
        let (data, resp) = try await URLSession.shared.data(for: req)
        let status = (resp as? HTTPURLResponse)?.statusCode ?? 0
        guard status == 200 else {
            throw NSError(domain: "Notch", code: status, userInfo: [NSLocalizedDescriptionKey: screenshotFlowProtocolMessage(data: data, statusCode: status)])
        }
    }

    private func waitUntilPendingFinalized(pendingId: String) async throws {
        let deadline = Date().addingTimeInterval(30)
        while Date() < deadline {
            if try await pendingStatus(pendingId: pendingId) == "finalized" {
                return
            }
            try await Task.sleep(nanoseconds: 400_000_000)
        }
        throw NSError(domain: "Notch", code: 408, userInfo: [NSLocalizedDescriptionKey: "Journal send timed out — retry."])
    }

    private func pendingStatus(pendingId: String) async throws -> String {
        let path = "/api/daemon/journal/toolbar-capture/pending/" + pendingId
        guard let url = URL(string: baseURL() + path) else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Bad daemon URL"])
        }
        let req = authorizedRequest(url: url, method: "GET")
        let (data, resp) = try await URLSession.shared.data(for: req)
        let status = (resp as? HTTPURLResponse)?.statusCode ?? 0
        guard status == 200 else {
            throw NSError(domain: "Notch", code: status, userInfo: [NSLocalizedDescriptionKey: "Status check failed (\(status))"])
        }
        guard
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any],
            let inner = j["data"] as? [String: Any],
            let st = inner["status"] as? String
        else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Unexpected status response"])
        }
        return st
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
        guard isAuthenticated && (sessionState == "active" || sessionState == "expiring_soon") else {
            journalCaptureLastError = "Sign in required to finalize captures."
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
            "idempotencyKey": StationWireClient.makeULID(),
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
        } catch let err as JournalInteractiveScreenshotError {
            journalCaptureScreenshotError = err.userMessage
        } catch {
            journalCaptureScreenshotError = error.localizedDescription
        }

        if let u = tempURL {
            JournalInteractiveScreenshot.removeTempFile(at: u)
        }
        journalCaptureScreenshotTempURL = nil
    }

    /// Presigned R2 PUT with bounded 429 retries (Phase 7).
    private func putPngToPresignedUrlWithRetry(
        uploadURL: URL,
        pngData: Data,
        contentType: String = "image/png"
    ) async throws -> Int {
        var lastCode = 0
        for attempt in 0..<3 {
            var putReq = URLRequest(url: uploadURL)
            putReq.httpMethod = "PUT"
            putReq.setValue(contentType, forHTTPHeaderField: "Content-Type")
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

    public func startPolling() {
        stopPolling()
        loadJournalCaptureDraftLocally()
        startDaemonEventsStream()
        connectionTick = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                self.setIfChanged(\.daemonConnectionState, self.connectionFSM.onTick())
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

        // Always poll sync-state (not market-hours gated) so Settings mirrors Station Start.
        pollBrokerSync = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in
            Task { @MainActor in
                await self?.refreshBrokerSyncState()
            }
        }

        Task {
            await refreshBrokerSyncState()
            await fetchDailyLossLimit()
            await fetchWorkflowStatus()
            await fetchMorningBriefIfNeeded()
            await fetchRecentTrades()
            if isIstMarketSession() {
                await fetchPulseData()
                await fetchPositions()
            }
        }
    }

    public func stopPolling() {
        toolbarShowCoalesceTask?.cancel()
        toolbarShowCoalesceTask = nil
        stopBarLiveStatePolling()
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
        pollBrokerSync?.invalidate()
        pollBrokerSync = nil
        connectionTick?.invalidate()
        connectionTick = nil
        stopKillSwitchCountdownTimer()
        stopCapturePasteboardWatch()
        setIfChanged(\.daemonConnectionState, .idle)
        daemonProtocolError = nil
        connectionFSM = DaemonConnectionFSM()
        pinnedAgentBootId = nil
        pinnedAgentSsePubKeyB64 = nil
        agentLocalDiagnostics = ""
        if let u = journalCaptureScreenshotTempURL {
            JournalInteractiveScreenshot.removeTempFile(at: u)
            journalCaptureScreenshotTempURL = nil
        }
        dictationSession?.stopAllForHostTeardown()
    }

    func expandFromCollapsedChromeTap() {
        guard !isExpanded else { return }
        // `.now` — haptic on the same frame as the first pixels, not the next draw.
        NotchHaptics.play(.light, at: .now)
        withAnimation(NotchTheme.expandCollapseAnimation) {
            isExpanded = true
        }
        onRequestOrderFront?()
        syncBarLiveStatePollingForVisibility()
    }

    /// Global monitor only receives clicks from *other* apps — use for dismiss-outside-expanded-panel.
    func collapseExpandedFromOutsideClick() {
        guard isExpanded else { return }
        withAnimation(NotchTheme.expandCollapseAnimation) {
            isExpanded = false
        }
        syncBarLiveStatePollingForVisibility()
    }

    public func collapseExpandedFromChromeTap() {
        guard isExpanded else { return }
        NotchHaptics.play(.light, at: .now)
        withAnimation(NotchTheme.expandCollapseAnimation) {
            isExpanded = false
        }
        syncBarLiveStatePollingForVisibility()
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
                self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
                await self.refreshPinnedAgentTrust()
                guard self.pinnedAgentSsePubKeyB64 != nil else {
                    self.connectionFSM.onTransportFailure()
                    self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                    continue
                }
                guard let url = URL(string: self.baseURL() + "/api/daemon/events/stream") else {
                    self.connectionFSM.onTransportFailure()
                    self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
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
                        self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
                        try? await Task.sleep(nanoseconds: 2_000_000_000)
                        continue
                    }
                    self.daemonProtocolError = nil
                    self.connectionFSM.onStreamOpened()
                    self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
                    for try await line in bytes.lines {
                        if Task.isCancelled { return }
                        guard line.hasPrefix("data:") else { continue }
                        let raw = String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces)
                        if self.consumeDaemonEvent(raw) {
                            self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
                        }
                    }
                    self.connectionFSM.onTransportFailure()
                    self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                } catch {
                    self.connectionFSM.onTransportFailure()
                    self.setIfChanged(\.daemonConnectionState, self.connectionFSM.state)
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
                setIfChanged(\.daemonConnectionState, connectionFSM.state)
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
                setIfChanged(\.daemonConnectionState, connectionFSM.state)
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
        let tradeIdRaw = payload["trade_id"] as? String
        let tradeId = tradeIdRaw.flatMap { s in
            let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
            return t.isEmpty ? nil : t
        }
        let symbolRaw = payload["symbol"] as? String
        let symbol = symbolRaw.flatMap { s in
            let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
            return t.isEmpty ? nil : t
        }
        let side = payload["side"] as? String ?? ""
        let qtyStr: String = {
            if let q = payload["qty"] as? Int { return String(q) }
            if let d = payload["qty"] as? Double { return String(Int(d)) }
            return ""
        }()
        let priceStr: String = {
            if let p = payload["price"] as? Double { return String(format: "%.2f", p) }
            if let i = payload["price"] as? Int { return String(format: "%.2f", Double(i)) }
            return "—"
        }()

        let banner: String
        if tradeId != nil, let sym = symbol {
            banner = "Trade detected · \(sym) \(side) \(qtyStr) @ \(priceStr)"
        } else {
            banner = "New trade · tap to capture"
        }

        journalCaptureBanner = banner
        dictationUsesCaptureDraft = true
        if let tid = tradeId, !journalCaptureLinkLocked {
            journalCaptureTradeIdRaw = tid
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
            recordKillSwitchStateReceived()
            if active {
                if !oldKs {
                    checkKillSwitchTransition(from: oldKs, to: true)
                }
                if let lv = payload["level"] as? String, !lv.isEmpty {
                    killSwitchLevel = lv
                }
                killSwitchRequiresAck = payload["requires_ack"] as? Bool ?? false
                if let c = payload["countdown_secs"] as? Int {
                    killSwitchCountdownSecs = c
                } else if let d = payload["countdown_secs"] as? Double {
                    killSwitchCountdownSecs = Int(d)
                } else {
                    killSwitchCountdownSecs = nil
                }
                startKillSwitchCountdownTimerIfNeeded()
            } else {
                killSwitchActive = false
                killSwitchCountdownSecs = nil
                killSwitchLevel = nil
                killSwitchRequiresAck = false
                stopKillSwitchCountdownTimer()
                if oldKs {
                    pulseAttention = .none
                }
            }
            refreshKillSwitchOverlayVisibility()
            return true

        case "broker_sync_state":
            let c = (payload["class"] as? String)?.lowercased() ?? "not_connected"
            brokerSyncClass = c
            brokerSessionActive = (brokerSyncClass == "synced" || brokerSyncClass == "stale")
            applyDeskHonesty(from: payload)
            applyDeskCapabilities(from: payload)
            if brokerSyncClass == "synced" || brokerSyncClass == "syncing" || brokerSyncClass == "stale" {
                brokerSyncLastPollAtMs = Int(Date().timeIntervalSince1970 * 1000)
            }
            return true

        case "toolbar_show":
            if immediateToolbarShow {
                applyToolbarShowPayload(payload)
            } else {
                scheduleToolbarShowPayload(payload)
            }
            return true

        case "session_state":
            let st = (payload["state"] as? String)?.lowercased() ?? "active"
            sessionState = st
            if st == "expired" || st == "signed_out" {
                isAuthenticated = false
                daemonProtocolError = .sigInvalid
                lastAlert = "Session expired — please sign in again."
            }
            return true

        case "auth_state":
            isAuthenticated = payload["authenticated"] as? Bool ?? true
            authProvider = payload["provider"] as? String
            if !isAuthenticated {
                daemonProtocolError = .sigInvalid
                lastAlert = "Sign in required."
            }
            return true

        case "state_changed":
            let old = compositeScore
            if let bs = payload["behavioral_state"] as? String, !bs.isEmpty {
                behavioralState = bs
            }
            if let s = payload["composite_score"] as? Double {
                compositeScore = s
            } else if let i = payload["composite_score"] as? Int {
                compositeScore = Double(i)
            }
            checkSmartTriggers(oldScore: old, newScore: compositeScore)
            return true

        case "trade_exit":
            barDebriefPending = BarDebriefArming.applyExplicitTradeExit()
            recomputeBarSurfacePhase()
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
                brokerSyncClass = sc.lowercased()
                brokerSessionActive = (brokerSyncClass == "synced" || brokerSyncClass == "stale")
            }
            applyDeskHonesty(from: j)
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
                    let filledAtMs: Int = {
                        if let m = row["filled_at_ms"] as? Int { return m }
                        if let m = row["filled_at_ms"] as? Double { return Int(m) }
                        return 0
                    }()
                    return RecentTradeRow(
                        id: rid,
                        symbol: sym,
                        side: side,
                        qty: qty,
                        price: price,
                        filledAtMs: filledAtMs
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
        setIfChanged(\.daemonConnectionState, .disconnected)
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

    /// Wire v1 loopback request via shared `StationWireClient` (T1 — bridge harden).
    /// `x-daemon-secret` + HMAC are machine integrity only; `loopbackWireUserId` is a
    /// fixed wire hint, never Console identity — see `StationWireClient` doc comment.
    private func authorizedRequest(url: URL, method: String = "GET", body: Data? = nil) -> URLRequest {
        let payload = body ?? Data()
        let path = url.path.isEmpty ? "/" : url.path
        var r = StationWireClient.signedRequest(
            method: method,
            path: path,
            body: payload,
            daemonSecret: daemonSecret
        )
        r.url = url
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

    func fetchPulseData() async {
        guard let url = URL(string: baseURL() + "/api/daemon/today") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
            let j = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
            applyDeskHonesty(from: j)
            let hero = j["hero"] as? [String: Any] ?? [:]
            if j["degradedReason"] is String {
                sessionPnL = 0
                winRate = 0
                tradesToday = 0
                return
            }
            if let p = hero["pnlTodayUsd"] as? Double {
                sessionPnL = p
            } else {
                sessionPnL = 0
            }
            if let w = hero["winRate"] as? Double { winRate = w }
            if let t = hero["tradesToday"] as? Int {
                let prev = tradesToday
                if t > prev {
                    fireTradeFillRingPulse()
                }
                tradesToday = t
            } else if let t = hero["tradesToday"] as? Double {
                tradesToday = Int(t)
            }
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    func fetchPositions() async {
        guard let url = URL(string: baseURL() + "/api/daemon/positions") else { return }
        let previousPositionCount = positions.count
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
            if let k = j["kill_switch_active"] as? Bool {
                killSwitchActive = k
                recordKillSwitchStateReceived()
            }
            if let o = j["open_orders"] as? Int { openOrders = o }
            var out: [NotchPosition] = []
            if let arr = j["positions"] as? [[String: Any]] {
                for p in arr {
                    let sym =
                        BarBrokerTicker.fromPositionRow(p)
                        ?? (p["tradingSymbol"] as? String)
                        ?? (p["symbol"] as? String)
                        ?? "—"
                    let qty = (p["quantity"] as? Int)
                        ?? (p["qty"] as? Int)
                        ?? Int(((p["qty"] as? Double) ?? (p["quantity"] as? Double) ?? 0).rounded())
                    let pnl = (p["unrealizedPnl"] as? Double) ?? (p["unrealized_pnl"] as? Double) ?? 0
                    let dir = (p["direction"] as? String) ?? (p["side"] as? String) ?? ""
                    out.append(NotchPosition(symbol: sym, qty: qty, unrealizedPnL: pnl, direction: dir))
                }
            }
            notePositionTransitionForBar(previousCount: previousPositionCount, newCount: out.count)
            positions = out
            let remembered = NotchChipCatalogMutations.rememberSymbols(out.map(\.symbol), in: chipCatalog)
            if remembered != chipCatalog {
                chipCatalog = remembered
                chipCatalogStore.save(chipCatalog)
            }
            recomputeBarSurfacePhase()
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
            let decoded = try JSONDecoder().decode(BriefMorningBriefResponse.self, from: data)
            morningBrief = decoded.makeMorningBrief(fetchedAt: Date())
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

    /// Brief tab CTA — jump to PLAN and surface declaration entry (#127).
    func startTradingFromMorningBrief() {
        activeTab = .plan
        presentBarDeclarationForm()
    }

    /// Test seam — apply a daemon morning-brief JSON fixture without network.
    func applyMorningBriefFixtureForTesting(_ data: Data) throws {
        let decoded = try JSONDecoder().decode(BriefMorningBriefResponse.self, from: data)
        morningBrief = decoded.makeMorningBrief(fetchedAt: Date(timeIntervalSince1970: 0))
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
        switch KillSwitchFirePayloadBuilder.build(protectiveBrokerSlug: barProtectiveBrokerSlug) {
        case let .failure(error):
            lastAlert = error.localizedDescription
            return
        case let .success(bodyDict):
            guard let url = URL(string: baseURL() + "/api/daemon/kill-switch") else { return }
            guard let body = try? JSONSerialization.data(withJSONObject: bodyDict) else { return }
            let req = authorizedRequest(url: url, method: "POST", body: body)
            do {
                let (data, resp) = try await URLSession.shared.data(for: req)
                let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
                guard (200...299).contains(code) else {
                    if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                       let err = json["error"] as? String, !err.isEmpty {
                        lastAlert = err
                    } else {
                        handleDaemonErrorResponse(data: data, statusCode: code)
                    }
                    return
                }
                let levelInt = bodyDict["level"] as? Int ?? 3
                let levelStr = levelInt >= 3 ? "L3" : (levelInt == 2 ? "L2" : "L\(levelInt)")
                _ = applyDaemonEventPayload(
                    type: "kill_switch_state",
                    payload: [
                        "active": true,
                        "level": levelStr,
                        "countdown_secs": 90,
                        "requires_ack": levelInt >= 3,
                    ],
                    immediateToolbarShow: false
                )
            } catch {
                lastAlert = error.localizedDescription
            }
        }
    }

    /// Overlay "I'm Calm" — ack telemetry then dismiss DNS + fog (#189).
    public func dismissKillSwitchFromOverlay() async {
        guard KillSwitchOverlayPresentation.calmButtonEnabled(countdownSecs: killSwitchCountdownSecs) else {
            return
        }
        killSwitchDismissBusy = true
        defer { killSwitchDismissBusy = false }
        if killSwitchRequiresAck {
            await ackKillSwitchOverlay()
        }
        guard let url = URL(string: baseURL() + "/api/daemon/dismiss-kill-switch") else { return }
        let body: [String: Any] = ["reason": "user_calm"]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        do {
            let (_, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: payload),
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else {
                lastAlert = "Could not dismiss kill switch (\(code))"
                return
            }
            killSwitchActive = false
            killSwitchCountdownSecs = nil
            killSwitchLevel = nil
            killSwitchRequiresAck = false
            recordKillSwitchStateReceived()
            stopKillSwitchCountdownTimer()
            refreshKillSwitchOverlayVisibility()
            pulseAttention = .none
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    /// `POST /api/daemon/kill-switch/ack` — ingestSignal telemetry (Slice B #189).
    func ackKillSwitchOverlay() async {
        guard let url = URL(string: baseURL() + "/api/daemon/kill-switch/ack") else { return }
        var body: [String: Any] = [
            "countdown_remaining_secs": killSwitchCountdownSecs ?? 0,
        ]
        if let lv = killSwitchLevel, !lv.isEmpty {
            body["level"] = lv
        }
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        let req = authorizedRequest(url: url, method: "POST", body: payload)
        _ = try? await URLSession.shared.data(for: req)
    }

    private func refreshKillSwitchOverlayVisibility() {
        killSwitchOverlayVisible = KillSwitchOverlayPresentation.shouldShowFullscreenOverlay(
            active: killSwitchActive,
            level: killSwitchLevel,
            countdownSecs: killSwitchCountdownSecs
        )
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
        // Notch must not open page URLs in the system browser or foreground the Next.js app.
        _ = urlString
    }

    func sessionSummaryForClose() -> String {
        let wr = String(format: "%.0f%%", winRate * 100)
        return "Session · P&L \(formatDeskMoney(sessionPnL)) · \(tradesToday) trades · WR \(wr)"
    }

    /// Collapsed / center strip label — aligns with validated ladder: warning 0.25 · soft 0.35 · hard 0.45 (`AGENTS.md`).
    var displayBehavioralState: String {
        let raw = behavioralState.uppercased()
        if raw.contains("TILT") || raw.contains("REVENGE") { return "TILTED" }
        if compositeScore >= 0.45 { return "DANGER" }
        if compositeScore >= 0.25 { return "CAUTION" }
        if compositeScore < 0.15 { return "FLOW" }
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
            self?.setIfChanged(\.dictationWaveform, levels)
        }
        s.onPermissionBlocked = { [weak self] blocked in
            self?.setIfChanged(\.dictationPermissionDenied, blocked)
        }
        s.onRequiresOnDeviceUnsupported = { [weak self] bad in
            self?.dictationOnDeviceOnlyUnsupported = bad
        }
        s.onRecordingState = { [weak self] on in
            self?.setIfChanged(\.isDictating, on)
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

    private func startKillSwitchCountdownTimerIfNeeded() {
        guard killSwitchCountdownSecs != nil else { return }
        stopKillSwitchCountdownTimer()
        let t = Timer(timeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                guard let remaining = self.killSwitchCountdownSecs, remaining > 0 else {
                    self.stopKillSwitchCountdownTimer()
                    return
                }
                self.killSwitchCountdownSecs = remaining - 1
                self.refreshKillSwitchOverlayVisibility()
            }
        }
        RunLoop.main.add(t, forMode: .common)
        killSwitchCountdownTimer = t
    }

    private func stopKillSwitchCountdownTimer() {
        killSwitchCountdownTimer?.invalidate()
        killSwitchCountdownTimer = nil
    }

    private func recordKillSwitchStateReceived() {
        killSwitchStateReceivedAt = Date()
    }

    /// Inject a cached kill-switch timestamp (unit tests only).
    public func setKillSwitchStateReceivedAt(_ date: Date?) {
        killSwitchStateReceivedAt = date
    }
}

// MARK: - VoiceOver (macOS SwiftUI has no `View.accessibilityLiveRegion`)

enum NotchVoiceOver {
    static func announce(_ message: String, assertive: Bool) {
        let element =
            NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first(where: \.isVisible)
        guard let element else { return }
        let level: NSAccessibilityPriorityLevel = assertive ? .high : .medium
        NSAccessibility.post(
            element: element,
            notification: .announcementRequested,
            userInfo: [
                .announcement: message as NSString,
                .priority: NSNumber(value: level.rawValue),
            ]
        )
    }
}

extension NotchViewModel {
    /// Desk-honest money format: follows `deskQuoteCurrency` from active connection (R7).
    /// Falls back to INR only when no active desk currency is known (legacy Bar equities default).
    public func formatDeskMoney(_ v: Double) -> String {
        let ccy = deskQuoteCurrency ?? "INR"
        return DeskMoneyFormatting.formatSigned(v, quoteCurrency: ccy)
    }

    /// Legacy name — routes through desk currency (no longer hardcodes INR when COM is active).
    public func formatINR(_ v: Double) -> String {
        formatDeskMoney(v)
    }

    func applyDeskHonesty(from payload: [String: Any]) {
        let slug = (payload["brokerSlug"] as? String)
            ?? (payload["active_broker_slug"] as? String)
            ?? (payload["broker_slug"] as? String)
        if let slug, !slug.isEmpty {
            activeBrokerSlug = slug
        }
        if let ccy = payload["quoteCurrency"] as? String, !ccy.isEmpty {
            deskQuoteCurrency = ccy.uppercased()
        } else if let ccy = payload["quote_currency"] as? String, !ccy.isEmpty {
            deskQuoteCurrency = ccy.uppercased()
        } else if let mapped = DeskMoneyFormatting.quoteCurrency(forBrokerSlug: activeBrokerSlug) {
            deskQuoteCurrency = mapped
        }
        if let calc = payload["calcProfileId"] as? String, !calc.isEmpty {
            deskCalcProfileId = calc
        } else if let calc = payload["calc_profile_id"] as? String, !calc.isEmpty {
            deskCalcProfileId = calc
        } else if let mapped = DeskMoneyFormatting.calcProfileId(forBrokerSlug: activeBrokerSlug) {
            deskCalcProfileId = mapped
        }
        if brokerSyncClass == "not_connected" || brokerSyncClass == "disconnected" {
            // Keep last known currency for display honesty; clear slug so dual logic stays accurate.
            activeBrokerSlug = nil
        }
    }

    /// Apply `GET /api/daemon/broker/sync-state` JSON (testable without network).
    func applyBrokerSyncStatePayload(_ json: [String: Any]) {
        if let sc = json["syncState"] as? String, !sc.isEmpty {
            brokerSyncClass = sc.lowercased()
            brokerSessionActive = (brokerSyncClass == "synced" || brokerSyncClass == "stale")
        }
        if let ms = json["lastPollAtMs"] as? Int {
            brokerSyncLastPollAtMs = ms
        } else if let ms = json["lastPollAtMs"] as? Double {
            brokerSyncLastPollAtMs = Int(ms)
        } else if let ms = json["lastSuccessAtMs"] as? Int {
            brokerSyncLastPollAtMs = ms
        } else if brokerSyncClass == "synced" || brokerSyncClass == "syncing" || brokerSyncClass == "stale" {
            brokerSyncLastPollAtMs = Int(Date().timeIntervalSince1970 * 1000)
        }
        applyDeskHonesty(from: json)
        applyDeskCapabilities(from: json)
    }

    /// Quote vs funds/fills stay independent — one broker pill is not enough.
    func applyDeskCapabilities(from payload: [String: Any]) {
        let caps = payload["capabilities"] as? [String: Any]
        if let q = caps?["quote"] as? String, !q.isEmpty {
            deskQuoteCapability = q.lowercased()
        }
        if let funds = caps?["funds"] as? String, !funds.isEmpty {
            deskFundsCapability = funds.lowercased()
        }
        if let fills = caps?["fills"] as? String, !fills.isEmpty {
            deskFillsCapability = fills.lowercased()
        }
        if let instruments = caps?["instruments"] as? String, !instruments.isEmpty {
            deskInstrumentsCapability = instruments.lowercased()
        }
        if caps == nil,
           brokerSyncClass == "not_connected" || brokerSyncClass == "disconnected"
        {
            deskQuoteCapability = "unavailable"
            deskFundsCapability = "unavailable"
            deskFillsCapability = "unavailable"
            deskInstrumentsCapability = "unavailable"
        }
    }

    /// Poll Enforcer sync-state so Notch mirrors Station Brokers Start (SSE alone is not enough).
    func refreshBrokerSyncState() async {
        guard let url = URL(string: baseURL() + "/api/daemon/broker/sync-state") else { return }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            guard (resp as? HTTPURLResponse)?.statusCode == 200,
                  let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
            else {
                // Leave last known sync class — do not invent connected.
                return
            }
            applyBrokerSyncStatePayload(json)
        } catch {
            // Agent down: keep last known; UI already has daemon FSM / live-state strips.
        }
    }

    static func brokerDisplayName(forSlug slug: String?) -> String {
        switch slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "kotak_neo", "kotak": return "Kotak Neo"
        case "binance_com": return "Binance.com"
        case nil, "": return "No broker"
        case let other?: return other.replacingOccurrences(of: "_", with: " ").capitalized
        }
    }

    /// Pure pill chrome mapping (testable).
    static func brokerPillChrome(
        brokerSyncClass: String,
        slug: String?
    ) -> (dotName: String, label: String) {
        let name = brokerDisplayName(forSlug: slug)
        switch brokerSyncClass.lowercased() {
        case "synced":
            return ("teal", "\(name) · live")
        case "syncing":
            return ("amber", "\(name) · connecting")
        case "stale":
            return ("amber", "\(name) · degraded")
        default:
            return ("red", "No broker · offline")
        }
    }

    /// Settings primary CTA: open Station Brokers only when sync is offline.
    static func showsOpenStationBrokersCta(brokerSyncClass: String) -> Bool {
        switch brokerSyncClass.lowercased() {
        case "synced", "syncing", "stale":
            return false
        default:
            return true
        }
    }

    /// Settings connection badge title (mirrors BarSettingsView SyncPosture).
    static func brokerSettingsConnectionBadge(brokerSyncClass: String) -> String {
        switch brokerSyncClass.lowercased() {
        case "synced": return "Connected"
        case "syncing": return "Connecting"
        case "stale": return "Degraded"
        default: return "Not connected"
        }
    }

    func testBrokerConnection() async {
        brokerActionError = nil
        brokerActionResultMessage = nil
        guard let url = URL(string: baseURL() + "/api/daemon/broker/sync-state") else { return }
        brokerActionBusy = true
        defer { brokerActionBusy = false }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            guard (resp as? HTTPURLResponse)?.statusCode == 200,
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let status = json["runtimeStatus"] as? String
            else {
                brokerActionError = "Agent offline — Retry in Station"
                return
            }
            applyBrokerSyncStatePayload(json)
            brokerActionResultMessage = "Agent reports: \(status.replacingOccurrences(of: "_", with: " "))"
        } catch {
            brokerActionError = "Agent offline — Retry in Station"
        }
    }

    func retryInstruments() async {
        guard let url = URL(string: baseURL() + "/api/daemon/broker/sync/retry") else { return }
        do {
            let (_, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: Data())
            )
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else {
                return
            }
            await refreshBrokerSyncState()
            if barDeclarationSymbol.count >= 2 {
                searchSymbols(barDeclarationSymbol)
            }
        } catch {
            return
        }
    }

    func disconnectBrokerSync() async {
        brokerActionError = nil
        brokerActionResultMessage = nil
        guard let url = URL(string: baseURL() + "/api/daemon/broker/sync/stop") else { return }
        brokerActionBusy = true
        defer { brokerActionBusy = false }
        do {
            let (_, resp) = try await URLSession.shared.data(
                for: authorizedRequest(url: url, method: "POST", body: Data())
            )
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else {
                brokerActionError = "Disconnect failed — Retry in Station"
                return
            }
            brokerSyncClass = "not_connected"
            activeBrokerSlug = nil
            brokerSessionActive = false
            brokerSyncLastPollAtMs = nil
            deskQuoteCapability = "unavailable"
            deskFundsCapability = "unavailable"
            deskFillsCapability = "unavailable"
            deskInstrumentsCapability = "unavailable"
            brokerActionResultMessage = "Disconnected"
        } catch {
            brokerActionError = "Agent offline — Retry in Station"
        }
    }
}
