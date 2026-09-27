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

/// One eapi `openInterest` row. Strings only — the venue's own text.
/// Empty field = missing, not `"0"`. Mixed-case `symbol` is the identity.
struct DeskOiExpiryRow: Identifiable, Equatable {
    var id: String { symbol }
    let symbol: String
    let sumOpenInterest: String?
    let sumOpenInterestUsd: String?
    let timestamp: String?
}

/// One `optionSymbols` catalog row. Not a strike grid: symbol / strike_raw /
/// side / expiry_raw as published. `last` is TickBook overlay only — missing
/// means absent, never `"0"`.
struct DeskChainCatalogRow: Identifiable, Equatable {
    var id: String { symbol }
    let symbol: String
    let instrumentId: String?
    let instrumentType: String?
    let strikeRaw: String?
    let side: String?
    let expiryRaw: String?
    let last: String?
}

/// One eapi kline for the crypto Options session chart. Venue strings — never a fake zero.
struct DeskSessionCandle: Identifiable, Equatable {
    var id: String { "\(openTimeMs)" }
    let openTimeMs: Int64
    let open: String
    let high: String
    let low: String
    let close: String
    let volume: String
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

struct TodayClosedTripCite: Equatable {
    var symbol: String
    var net: Double
}

/// One meter inside a venue egress posture (Rust `MeterPosture`). Parsed loosely from SSE JSON.
struct VenueMeterPosture: Equatable {
    var meter: String
    var posture: String
    var untilMs: Int64?
    var budgetLimit: UInt32?
    var used: UInt32?
    var inflight: UInt32

    init(
        meter: String,
        posture: String,
        untilMs: Int64? = nil,
        budgetLimit: UInt32? = nil,
        used: UInt32? = nil,
        inflight: UInt32 = 0
    ) {
        self.meter = meter
        self.posture = posture
        self.untilMs = untilMs
        self.budgetLimit = budgetLimit
        self.used = used
        self.inflight = inflight
    }

    init?(payload: [String: Any]) {
        let meter = (payload["meter"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !meter.isEmpty else { return nil }
        self.meter = meter
        self.posture = (payload["posture"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? "live"
        self.untilMs = VenuePosture.parseInt64(payload["until_ms"])
        self.budgetLimit = VenuePosture.parseUInt32(payload["budget_limit"])
        self.used = VenuePosture.parseUInt32(payload["used"])
        self.inflight = VenuePosture.parseUInt32(payload["inflight"]) ?? 0
    }
}

/// Slot-wide venue egress posture (Rust `VenuePosture`). Keys: `binance_com`, `kotak_neo`.
struct VenuePosture: Equatable {
    var venue: String
    var posture: String
    var untilMs: Int64?
    var meters: [VenueMeterPosture]

    init(venue: String, posture: String, untilMs: Int64? = nil, meters: [VenueMeterPosture] = []) {
        self.venue = venue
        self.posture = posture
        self.untilMs = untilMs
        self.meters = meters
    }

    init?(payload: [String: Any]) {
        let venue = (payload["venue"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !venue.isEmpty else { return nil }
        self.venue = venue
        self.posture = (payload["posture"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? "live"
        self.untilMs = Self.parseInt64(payload["until_ms"])
        let rawMeters = payload["meters"] as? [[String: Any]] ?? []
        self.meters = rawMeters.compactMap { VenueMeterPosture(payload: $0) }
    }

    static func parseInt64(_ value: Any?) -> Int64? {
        switch value {
        case let v as Int64: return v
        case let v as Int: return Int64(v)
        case let v as UInt64: return Int64(bitPattern: v)
        case let v as Double: return Int64(v)
        case let v as NSNumber: return v.int64Value
        default: return nil
        }
    }

    static func parseUInt32(_ value: Any?) -> UInt32? {
        switch value {
        case let v as UInt32: return v
        case let v as Int: return v >= 0 ? UInt32(v) : nil
        case let v as Int64: return v >= 0 && v <= Int64(UInt32.max) ? UInt32(v) : nil
        case let v as Double: return v >= 0 ? UInt32(v) : nil
        case let v as NSNumber: return v.uint32Value
        default: return nil
        }
    }
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

    /// Pulse / TAI / Workflows / Positions are unreachable theater on the desk.
    static let unreachableOnDesk: Set<NotchTab> = [.pulse, .tai, .workflows, .positions]

    var isReachableOnDesk: Bool { !Self.unreachableOnDesk.contains(self) }
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
    var hasOpenPositions: Bool {
        if declareAssetClass.isNamedComFutures {
            return accountChrome.bookId == declareBookId
                && accountChrome.positionsStatus == "success"
                && accountChrome.positionsCount > 0
        }
        return !positions.isEmpty
    }
    @Published var openOrders: Int = 0
    @Published public var killSwitchActive: Bool = false
    @Published public var killSwitchCountdownSecs: Int?
    /// Policy latch expiry from the agent. Overlay remaining time is derived from this.
    @Published public var killSwitchExpiresAtMs: Int64?
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
    @Published var activeTab: NotchTab = .plan
    /// When true (Station-hosted), expanded surface stays on PLAN / `BarNotchShell` only.
    private(set) var planSurfaceOnly: Bool = false
    @Published var lastAlert: String?
    @Published var pulseAttention: PulseAttention = .none
    @Published public var brokerSessionActive: Bool = false
    /// Catalog slug of the active UBI sync (`binance_com` / `kotak_neo`) — execution desk (R7).
    /// The slug picks desk-honest money formatting; market books follow instrument shape.
    @Published public var activeExecutionBrokerSlug: String? {
        didSet {
            guard oldValue != activeExecutionBrokerSlug else { return }
            invalidateDeskMarketExtracts(reason: "broker-slug")
            reconcileDeclareAssetClassForConnectedDesk()
        }
    }

    @available(*, deprecated, renamed: "activeExecutionBrokerSlug")
    public var activeBrokerSlug: String? {
        get { activeExecutionBrokerSlug }
        set { activeExecutionBrokerSlug = newValue }
    }

    /// Named market book for the selected instrument (`kotak-nse-nfo`, `binance-com-options`, …).
    @Published public var selectedMarketBookId: String?
    /// Quote currency for Notch/Today formatting; follows active connection (never FX-blend).
    @Published public var deskQuoteCurrency: String?
    /// Calc profile id for the active connection (`crypto_spot_usd` / `equities_inr_cash`).
    @Published public var deskCalcProfileId: String?

    /// LiveBook declare `book_id`. Nil on Start/spot so CATIUSDT letters stay bookless there.
    var declareBookId: String? {
        BarAccountChrome.namedBookId(forAssetClass: declareAssetClass, startSlug: resolvedDeskSlug)
    }

    func adoptDeskTicket() {
        if !deskTicket.bookId.isEmpty {
            var stored = deskTicket
            stored.triggerPrice = ticketTriggerPrice
            if stored.sizeMode == .quote {
                stored.quoteOrderQty = Double(quoteOrderQtyText.trimmingCharacters(in: .whitespacesAndNewlines))
            }
            deskTicketByBook[deskTicket.bookId] = stored.coerced()
        }
        guard let book = BarDeskTicketSurface.bookId(
            for: declareAssetClass,
            slug: resolvedDeskSlug,
            instrumentId: deskSelectedInstrumentId
        ) else {
            deskTicket = .blank
            quoteOrderQtyText = ""
            ticketTriggerPrice = ""
            return
        }
        if let kept = deskTicketByBook[book], kept.bookId == book {
            deskTicket = kept.coerced()
        } else {
            deskTicket = .defaults(bookId: book)
        }
        deskTicketByBook[book] = deskTicket
        quoteOrderQtyText = deskTicket.quoteOrderQty.map { String($0) } ?? ""
        ticketTriggerPrice = deskTicket.triggerPrice
    }

    func persistDeskTicket() {
        if deskTicket.bookId.isEmpty {
            adoptDeskTicket()
        }
        guard !deskTicket.bookId.isEmpty else { return }
        var stored = deskTicket.coerced()
        stored.triggerPrice = ticketTriggerPrice
        if stored.sizeMode == .quote {
            stored.quoteOrderQty = Double(quoteOrderQtyText.trimmingCharacters(in: .whitespacesAndNewlines))
        } else {
            stored.quoteOrderQty = nil
        }
        deskTicket = stored
        deskTicketByBook[stored.bookId] = stored
    }

    func futuresMarginReadout(symbol: String) -> (mode: String?, leverage: String?, liq: String?) {
        BarAccountChrome.futuresMargin(from: accountChrome, symbol: symbol)
    }
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
    /// Closed island press — scales the whole chin from the top, not just the row.
    @Published var collapsedPillPressed: Bool = false
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
    /// Per-venue egress posture from SSE `venue_egress_state` (`binance_com` / `kotak_neo`).
    @Published var venuePostureBySlug: [String: VenuePosture] = [:]
    /// Last successful `/api/daemon/broker/sync-state` poll (`lastPollAtMs` or now).
    @Published var brokerSyncLastPollAtMs: Int?
    @Published var sessionState: String = "active"
    @Published var isAuthenticated: Bool = true
    @Published var authProvider: String?
    /// Phase 9 — founder QA snapshot text from `/api/daemon/health` (+ metrics URL hint).
    @Published var agentLocalDiagnostics: String = ""
    /// S7 vendor fence — Health of bindings, never a vendor last.
    @Published var vendorFenceRows: [VendorFenceRow] = []
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
    @Published var todayClosedTrips: [TodayClosedTripCite] = []
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
    @Published var declEmotionalFrustration: Int = 0
    @Published var declEmotionalExcitement: Int = 0
    @Published var declStance: String = ""
    @Published var declIntent: String = ""
    @Published var openNonNegotiable: String = ""
    @Published var declEmotionNow: Int = 0
    @Published var declInvalidationPrice: String = ""
    @Published var declCashProduct: String = ""
    @Published var declGateCapital: Bool = false
    @Published var declGateOnePercent: Bool = false
    @Published var declGateMaxLoss: Bool = false
    @Published var declGateHedge: Bool = false
    @Published var declGateReview: Bool = false
    @Published var declEntryPrice: String = ""
    /// Set when LTP fetch returns `session_expired` — show reconnect hint on entry field (#149).
    @Published var barLtpFetchError: String?
    @Published var declStopLoss: String = ""
    @Published var declTarget: String = ""
    @Published var declSetupType: String = ""
    @Published var declInvalidationType: String = ""
    @Published var declInvalidationCondition: String = ""
    @Published var declProtectiveSLConsent: Bool = true

    /// Spot / equity / options / USDM / Coin-M — not the Intraday/Swing style tabs.
    @Published var declareAssetClass: BarDeclareAssetClass = .spot {
        didSet {
            guard oldValue != declareAssetClass else { return }
            if declareAssetClass.isNamedComFutures {
                declProtectiveSLConsent = false
                sessionPnL = 0
            }
            if oldValue.isNamedComFutures, !declareAssetClass.isNamedComFutures {
                usdmPositionbookCount = 0
            }
            adoptDeskTicket()
            reconcileDeskLastForAssetClass(previousClass: oldValue)
            if brokerSessionActive {
                Task { await refreshAccountChrome() }
            }
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
    /// Venue ticket keyed by `book_id`. Class switch adopts that book's slot — never leftover type/sizeMode.
    @Published var deskTicket: BarDeskTicketIntent = .blank
    @Published var deskTicketByBook: [String: BarDeskTicketIntent] = [:]
    @Published var quoteOrderQtyText: String = ""
    @Published var ticketTriggerPrice: String = ""
    @Published var deskLastStatus: String = "unavailable"
    /// Venue `PRICE_FILTER.tickSize` from the named book's quote envelope. Never `pricePrecision`.
    @Published var deskTickSize: String?
    /// Venue `LOT_SIZE.stepSize` from the named book's quote envelope.
    @Published var deskStepSize: String?
    @Published var deskHistoryStatus: String = "unavailable"
    @Published var deskHistoryIneligible: [String] = []
    @Published var deskHistoryCandles: [DeskSessionCandle] = []
    /// Envelope `data.interval` while a licensed series is lit. Cleared with the candles.
    @Published var deskHistoryInterval: String?
    /// Bound `data.last` from the quote envelope. Chart last line reads `sessionChartLast`.
    @Published var deskQuoteLast: Double?
    @Published var deskHistoryProductUse: String?
    @Published var deskHistoryBookId: String?
    @Published var deskYahooHistoryStatus: String = "unavailable"
    @Published var deskYahooHistoryIneligible: [String] = []
    @Published var deskChainStatus: String = "unavailable"
    /// `optionSymbols` catalog rows for the bound underlying+expiry. Empty is a
    /// hole, not a guessed strike list.
    @Published var deskChainRows: [DeskChainCatalogRow] = []
    @Published var deskOiStatus: String = "unavailable"
    /// Exact-match `GET /eapi/v1/openInterest` fields. Venue strings, never a summed total.
    @Published var deskOiSumOpenInterest: String? = nil
    @Published var deskOiSumOpenInterestUsd: String? = nil
    @Published var deskOiTimestamp: String? = nil
    @Published var deskOiSymbol: String? = nil
    /// Expiry row list when there is no exact symbol match. Do not sum these.
    @Published var deskOiRows: [DeskOiExpiryRow] = []
    /// NFO `open_int` string. Never copied into `deskOiSumOpenInterest` (eapi).
    @Published var deskOiOpenInt: String? = nil
    @Published var deskOiField: String? = nil
    /// Depth glance. Unusable keeps an empty ladder — never last-good levels.
    @Published var deskDepthStatus: String = "unavailable"
    @Published var deskDepthDisplay: Bool = false
    @Published var deskDepthBids: [DeskDepthLevel] = []
    @Published var deskDepthAsks: [DeskDepthLevel] = []
    @Published var deskDepthPhysics: String = "bounded_snapshot"
    /// `GET /api/station/greeks` — the venue's own mark table, passed through. Never a pricer.
    @Published var deskGreeksStatus: String = "unavailable"
    /// The venue's published text for each greek. String, never Double: reparsing a
    /// venue number invents precision the venue did not publish.
    @Published var deskGreeksDelta: String? = nil
    @Published var deskGreeksGamma: String? = nil
    @Published var deskGreeksTheta: String? = nil
    @Published var deskGreeksVega: String? = nil
    /// `rights.display` from the greeks envelope. False (or missing) means Station may
    /// not print the number even when the envelope carries one.
    @Published var deskGreeksDisplay: Bool = false
    /// `model · path` for the lit greeks row — empty while dark.
    @Published var deskGreeksProv: String = ""
    /// `ineligible` reasons from the glance envelope. NFO provenance reads this
    /// (`pricing_model_unspecified`) instead of inventing “missing F&O master”.
    @Published var deskGreeksIneligible: [String] = []
    /// Did the desk actually ask for greeks on this generation? Distinguishes "no dated
    /// contract yet, so nothing was requested" from "asked the venue and got a hole" —
    /// two different sentences, and only one of them is "waiting".
    @Published private(set) var deskGreeksAsked: Bool = false
    /// LatestState `indexPrice` string. Missing stays nil — never 0, never lastPrice.
    @Published var deskIndexStatus: String = "unavailable"
    @Published var deskIndexPrice: String? = nil
    @Published var deskIndexUnderlying: String? = nil
    /// Desk-level `capabilities.quote` from sync-state — independent of funds/fills.
    @Published var deskQuoteCapability: String = "unavailable"
    /// Desk-level `capabilities.funds` from sync-state — independent of quote.
    @Published var deskFundsCapability: String = "unavailable"
    /// Desk-level `capabilities.fills` from sync-state — independent of quote.
    @Published var deskFillsCapability: String = "unavailable"
    /// Desk-level `capabilities.instruments` from sync-state — independent of quote/account.
    @Published var deskInstrumentsCapability: String = "unavailable"
    /// S8 obtain pulse + ledger for the shipping book of Start. Never Today fill-inventory.
    @Published var accountChrome: BarAccountChrome.Snapshot = .empty
    /// Last successful USDM `positionbook` row count — DualNoBlend vs daemon `/positions`.
    private var usdmPositionbookCount: Int = 0
    /// Drop stale account obtain replies when Start slug / session changes.
    private var accountChromeGeneration: UInt64 = 0
    /// Last selected TickBook id (`nse_cm|2885`, `nse_fo|token`, or Binance pair).
    @Published var deskSelectedInstrumentId: String = ""
    /// Scrip `instrument_type` (`OPTIDX` / `FUTIDX` / …). Empty is unknown — NFO stays three-zone.
    @Published var deskSelectedInstrumentType: String = ""
    /// Bumped on every desk rebind (symbol, asset class, broker slug). An extract
    /// Task carries the generation it started under and drops its apply once this moves,
    /// so a slow reply for the previous instrument can never paint the current one.
    private(set) var deskExtractGeneration: UInt64 = 0
    /// What caused the last invalidation — `select-symbol` / `asset-class` / `broker-slug`.
    private(set) var deskExtractInvalidationReason: String?

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
        let slug = (activeExecutionBrokerSlug ?? barProtectiveBrokerSlug)
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
        guard !sessionPnLOwnerMissing else { return "—" }
        return formatDeskMoney(sessionPnL)
    }

    /// NFO + Binance Options have no realized-PnL owner — pill dashes, does not steal spot/cash.
    var sessionPnLOwnerMissing: Bool {
        if declareAssetClass == .options { return true }
        let book = selectedMarketBookId?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        if book == BarDeskTemplate.kotakNfoBookId || book == BarDeskTemplate.binanceComOptionsBookId {
            return true
        }
        return BarDeskTemplate.isKotakNfoDesk(slug: resolvedDeskSlug, assetClass: declareAssetClass)
    }

    /// Spot/equity Today hero only. Named futures use income; NFO/options stay dark.
    var paintsTodayHero: Bool {
        !declareAssetClass.isNamedComFutures && !sessionPnLOwnerMissing
    }

    var startedDeskSlugs: [String] {
        var slugs: [String] = []
        var seen = Set<String>()
        func append(_ raw: String?) {
            let slug = raw?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
            guard !slug.isEmpty, !seen.contains(slug) else { return }
            seen.insert(slug)
            slugs.append(slug)
        }
        let started = brokerSessionActive
            || brokerSyncClass == "synced"
            || brokerSyncClass == "stale"
            || brokerSyncClass == "syncing"
        if started {
            append(activeExecutionBrokerSlug)
        }
        for (slug, posture) in venuePostureBySlug {
            if posture.posture.lowercased() == "live" {
                append(slug)
            }
        }
        return slugs
    }

    var sessionClockPresentation: BookSessionClock.Presentation {
        BookSessionClock.presentation(
            bookId: selectedMarketBookId,
            brokerSlug: activeExecutionBrokerSlug,
            at: Date()
        )
    }

    var sessionClockLabel: String { sessionClockPresentation.label }

    var deskSessionStale: Bool { sessionClockPresentation.stale }

    func shouldPollMarketReads(now: Date = Date()) -> Bool {
        BookSessionClock.shouldPollMarketReads(startedSlugs: startedDeskSlugs, at: now)
    }

    var accountImpact: AccountImpact {
        AccountImpact.compute(
            positions: positions.map { (symbol: $0.symbol, unrealizedPnL: $0.unrealizedPnL) },
            pinnedSymbols: chipCatalog.pinnedNameSymbols,
            dailyLossLimit: dailyLossLimit,
        )
    }

    /// Collapsed macOS strip — intervention wins; else logo + session P&L in the notch ears.
    var collapsedNotchPresentation: CollapsedNotchPresentation {
        CollapsedNotchPresentation.build(
            notch: barLiveState,
            impact: accountImpact,
            pnlText: formattedSessionPnL,
            sessionClockText: sessionClockPresentation.collapsedLabel,
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

    /// Dark until a real notional exists. Never qty × 1000.
    var totalExposureText: String { "—" }

    var killDnsHostRows: [(label: String, url: String)] {
        let armed = barProtectiveBrokerSlug.trimmingCharacters(in: .whitespacesAndNewlines)
        let slug = armed.isEmpty ? activeExecutionBrokerSlug : armed
        return KillDnsHosts.hosts(forBrokerSlug: slug)
    }

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
    private var barLiveStateKickoffTask: Task<Void, Never>?
    private var barLiveStateFetchInFlight = false
    /// Consecutive live-state chrome misses — not the hybrid-armed reconcile counter.
    private var barLiveStateStripFailures: Int = 0
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

    /// Drop leftover 45s hybrid-armed UserDefaults from builds before N1.
    private func restoreOptimisticArmedFromDefaults() {
        clearOptimisticArmedStorage()
    }

    private func recordOptimisticPollFailure() {
        guard barOptimisticArmedDisplay != nil else { return }
        barOptimisticReconcilePollFailures += 1
    }

    private func resetOptimisticPollFailures() {
        barOptimisticReconcilePollFailures = 0
    }

    /// §6.6 Policy C (swift slice): while the draft has non-whitespace text, trade UUID + pending toggle are fixed.
    var journalCaptureLinkLocked: Bool {
        !journalCaptureDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func selectTab(_ tab: NotchTab) {
        let resolved: NotchTab
        if planSurfaceOnly || !tab.isReachableOnDesk {
            resolved = .plan
        } else {
            resolved = tab
        }
        activeTab = resolved
        if resolved != .plan {
            showingDeclarationForm = false
        }
        dictationUsesCaptureDraft = planSurfaceOnly ? false : (resolved == .capture)
        syncBarLiveStatePollingForVisibility()
    }

    /// Extracts live-state once when the expanded panel shows PLAN. No 2s poll spine (N1).
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

    /// Working price-kind invalidation — lands Debrief without flattening.
    func openWorkingDebrief() {
        barDebriefPending = true
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
        barLiveStateKickoffTask?.cancel()
        barLiveStateKickoffTask = Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: 300_000_000)
            guard let self, !Task.isCancelled else { return }
            guard self.isExpanded, self.activeTab == .plan else { return }
            await self.fetchBarLiveState()
        }
    }

    private func stopBarLiveStatePolling() {
        barLiveStateKickoffTask?.cancel()
        barLiveStateKickoffTask = nil
    }

    func fetchBarLiveState() async {
        guard let url = BarLiveStatePollingTarget.url(agentBase: baseURL()) else { return }
        guard !barLiveStateFetchInFlight else { return }
        barLiveStateFetchInFlight = true
        defer { barLiveStateFetchInFlight = false }
        do {
            let (data, resp) = try await URLSession.shared.data(for: authorizedRequest(url: url))
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard code == 200 else {
                applyLiveStatePollFailure(
                    message: BarLiveStateErrorPresentation.message(httpStatus: code, body: data),
                    isDeviceLogin: BarLiveStateErrorPresentation.requiresDeviceLogin(body: data)
                )
                return
            }
            let decoded = try JSONDecoder().decode(BarLiveStateAPIResponse.self, from: data)
            applyLiveStatePollSuccess(decoded)
        } catch {
            applyLiveStatePollFailure(message: error.localizedDescription, isDeviceLogin: false)
        }
    }

    /// Test seam — success path without `URLSession.shared`.
    func applyLiveStatePollSuccess(_ decoded: BarLiveStateAPIResponse) {
        barLiveStateStripFailures = BarLiveStatePollChrome.nextConsecutiveFailures(
            previous: barLiveStateStripFailures,
            succeeded: true
        )
        setIfChanged(\.daemonProtocolError, nil)
        withAnimation(.none) {
            var needsRecompute = false
            let newState = decoded.notch
            let newFeaturesActive = decoded.barFeaturesActive
            let oldState = barLiveState

            if barLiveState != newState {
                barLiveState = newState
                needsRecompute = true
                if oldState?.archetype != newState?.archetype {
                    refreshActiveArchetype()
                }
            }

            // Hosted `notch.behavioral_score` wins over pulse/SSE — apply every poll (#180).
            newState?.applyBehavioralToViewModel(self)

            if let slug = newState?.protectiveBrokerSlug?
                .trimmingCharacters(in: .whitespacesAndNewlines),
                !slug.isEmpty
            {
                setIfChanged(\.barProtectiveBrokerSlug, slug)
            }

            if barFeaturesActiveFromApi != newFeaturesActive {
                barFeaturesActiveFromApi = newFeaturesActive
                needsRecompute = true
                if newFeaturesActive == false {
                    clearOptimisticArmedStorage()
                }
            }

            if needsRecompute {
                recomputeBarSurfacePhase()
            }

            barLastFetched = Date()

            setIfChanged(\.barStateError, nil)
            setIfChanged(\.barStateRequiresDeviceLogin, false)
        }
        resetOptimisticPollFailures()
    }

    /// Test seam — keep last-good `barLiveState`; chrome follows [`BarLiveStatePollChrome`].
    /// Transient poll misses never write `barStateError` (that strip is stop-me / action errors).
    func applyLiveStatePollFailure(message: String, isDeviceLogin: Bool) {
        _ = message
        recordOptimisticPollFailure()
        barLiveStateStripFailures = BarLiveStatePollChrome.nextConsecutiveFailures(
            previous: barLiveStateStripFailures,
            succeeded: false
        )
        let showDeviceLogin = BarLiveStatePollChrome.shouldPublishError(
            hasLiveState: barLiveState != nil,
            consecutiveFailures: barLiveStateStripFailures,
            isDeviceLogin: isDeviceLogin
        )
        if showDeviceLogin {
            setIfChanged(\.barStateRequiresDeviceLogin, true)
        }
        recomputeBarSurfacePhase()
    }

    func recomputeBarSurfacePhase() {
        let matchedDeclTrimmed =
            barLiveState?.matchedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if matchedDeclTrimmed.isEmpty {
            stopMeStep = 0
        }
        if barDebriefPending {
            setIfChanged(\.barSurfacePhase, .debrief)
            return
        }
        let hasPos: Bool
        if declareAssetClass.isNamedComFutures {
            hasPos = accountChrome.bookId == declareBookId
                && accountChrome.positionsStatus == "success"
                && accountChrome.positionsCount > 0
        } else {
            hasPos = !positions.isEmpty
        }
        if hasPos {
            clearOptimisticArmedStorage()
            setIfChanged(\.barSurfacePhase, .livePlan)
            return
        }
        if let pending = barLiveState?.pendingDeclaration, pending.status.uppercased() == "PENDING" {
            clearOptimisticArmedStorage()
            setIfChanged(\.barSurfacePhase, .armed)
            return
        }
        // N1: armed is the book. Do not keep a 45s hope after extract, and do not
        // paint “check web Bar” when the local book has no pending.
        if barOptimisticArmedDisplay != nil, barLastFetched != nil {
            clearOptimisticArmedStorage()
        }
        if barOptimisticArmedDisplay != nil, barLastFetched == nil {
            setIfChanged(\.barSurfacePhase, .armed)
            return
        }
        let planRaw = barLiveState?.planState?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let hasPlanSignal = !planRaw.isEmpty
        // Hosted `plan_state` can be set without a matched declaration; native declare UX lives on `.declaration`.
        if hasPlanSignal, !showingDeclarationForm {
            setIfChanged(\.barSurfacePhase, .livePlan)
        } else {
            setIfChanged(\.barSurfacePhase, .declaration)
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
            // 80ms retriggered search on every letter of BTCUSDT and kept the
            // form dirty. Wait for a pause; cancel still drops in-flight keys.
            try? await Task.sleep(nanoseconds: 250_000_000)
            guard !Task.isCancelled else { return }
            guard let url = URL(string: self.baseURL() + self.deskSearchExtractPath(query: query))
            else { return }
            let req = self.authorizedRequest(url: url)
            do {
                let (data, _) = try await URLSession.shared.data(for: req)
                if let expectedBook = self.declareBookId {
                    guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                        return
                    }
                    self.applyObtainSearchEnvelope(json, expectedBook: expectedBook)
                    return
                }
                let resp = try JSONDecoder().decode(InstrumentSearchResponse.self, from: data)
                let filtered = DeskCatalogAllowlist.filterSymbols(
                    resp.symbols,
                    deskSlug: self.resolvedDeskSlug
                )
                if self.symbolSuggestions != filtered {
                    self.symbolSuggestions = filtered
                }
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

    /// A new instrument is a new price: the Entry seeded from the previous binding's Last
    /// must not stand while the new book answers. Re-picking the same id is not a rebind,
    /// so a typed Entry survives a repeat selection or a blur.
    private func clearEntryIfRebinding(to newId: String) {
        if newId != deskSelectedInstrumentId {
            declEntryPrice = ""
        }
    }

    /// Binds the selected token. On a new id, clears `deskSelectedInstrumentType` unless
    /// search already supplied `instrument_type`.
    private func bindDeskSelectedInstrumentId(_ newId: String, instrumentTypeFromSearch: String? = nil) {
        let rebinding = newId != deskSelectedInstrumentId
        clearEntryIfRebinding(to: newId)
        deskSelectedInstrumentId = newId
        guard rebinding else { return }
        if let raw = instrumentTypeFromSearch?.trimmingCharacters(in: .whitespacesAndNewlines),
           !raw.isEmpty
        {
            deskSelectedInstrumentType = raw
        } else {
            deskSelectedInstrumentType = ""
        }
    }

    func selectSymbol(_ result: InstrumentResult) {
        symbolSearchTask?.cancel()
        ignoreSymbolSearchUntilEdit = true
        if DeskCatalogAllowlist.refusesKotakSelection(result, deskSlug: resolvedDeskSlug) {
            refuseSelection(hint: kotakInstrumentHint)
            return
        }
        guard let ticker = BarBrokerTicker.normalize(raw: result.trading_symbol) else {
            refuseSelection(
                hint: "Symbol must be a broker ticker (e.g. RELIANCE), not a company name."
            )
            return
        }
        // Resolve once, then act — no branch below re-derives the binding.
        let bind = DeskInstrumentBind.resolve(
            result,
            slug: resolvedDeskSlug,
            currentClass: declareAssetClass
        )
        let kotakDesk = BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
        if kotakDesk {
            let wanted: DeskInstrumentShape = declareAssetClass == .options ? .kotakNfo : .kotakCash
            guard bind.shape == wanted else {
                if declareAssetClass == .options {
                    deskLastStatus = "unavailable"
                    deskQuoteCapability = "unavailable"
                }
                refuseSelection(hint: kotakInstrumentHint)
                return
            }
        } else if bind.isKotakIdentity {
            // Pasted/stale Kotak row on a desk that cannot serve it — the NFO extracts
            // standing on screen belong to a book this desk never reads.
            invalidateDeskMarketExtracts(reason: "select-symbol")
            refuseSelection(hint: deskInstrumentHint)
            return
        }
        barDeclarationSymbol = ticker
        barDeclarationLastError = nil
        barLtpFetchError = nil
        showSymbolSuggestions = false
        symbolSuggestions = []
        symbolSearchHint = nil
        // The tab follows the instrument when it is showing Options for something that is
        // not an option (a Binance pair). Never the reverse — picking a contract does not
        // silently arm the Options surface. USDM / Coin-M must not snap a pair back to spot.
        if declareAssetClass == .options, bind.assetClass != .options {
            declareAssetClass = bind.assetClass
        }
        // Order matters: the id first, then invalidate (it reads the id to decide whether
        // Last may survive), then fetch — which captures the generation it must match.
        bindDeskSelectedInstrumentId(bind.tickBookId, instrumentTypeFromSearch: result.instrument_type)
        selectedMarketBookId = bind.bookId ?? Self.marketBook(for: bind.tickBookId)
        adoptDeskTicket()
        invalidateDeskMarketExtracts(reason: "select-symbol")
        switch bind.shape {
        case .kotakNfo, .kotakCash:
            if result.last_price > 0 {
                applyLTP(result.last_price)
            }
            fetchStationQuote(instrument: bind.tickBookId)
        case .binanceOption:
            // The dated contract is the binding, so the eapi book may answer for it. The
            // catalog row's own `last_price` is spot-shaped — it never seeds the premium,
            // and `/instruments/ltp` is a Kotak seam that no-ops here.
            fetchStationQuote(instrument: bind.tickBookId)
        case .binanceSpot:
            if declareAssetClass != .usdm, declareAssetClass != .coinm, result.last_price > 0 {
                applyLTP(result.last_price)
            }
            fetchStationQuote(instrument: bind.tickBookId)
        case .unknown:
            if result.last_price > 0 {
                applyLTP(result.last_price)
            }
            fetchLTP(
                symbol: ticker,
                exchange: result.exchange,
                segment: result.segment ?? result.exchange
            )
        }
        refreshDeskExtracts(symbol: bind.chainUnderlying, instrumentId: bind.tickBookId)
    }

    /// Refuse a selection without binding it: the symbol field and the previous
    /// instrument stay exactly as they were.
    private func refuseSelection(hint: String) {
        barDeclarationLastError = hint
        showSymbolSuggestions = false
        symbolSuggestions = []
        symbolSearchHint = nil
    }

    /// Symbol field commit — Return or focus loss. The Binance catalog is spot-only, so a
    /// dated contract never comes back as a suggestion row: the typed string is the only
    /// binding there is. USDM / Coin-M pairs commit on the named book (`CATIUSDT`,
    /// `BTCUSD_PERP`). A leftover pair on Options still commits nothing.
    func commitDeskSymbol() {
        let trimmed = barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines)
        // Blur fires on every focus toggle — rebinding the same id would re-invalidate
        // extracts that are already correct.
        guard !trimmed.isEmpty, trimmed != deskSelectedInstrumentId else { return }
        if declareAssetClass.isNamedComFutures {
            commitNamedFuturesPair(trimmed)
            return
        }
        guard InstrumentTickBookId.isDatedOptionContract(trimmed) else { return }

        let bind = DeskInstrumentBind.resolve(
            rawId: trimmed,
            slug: resolvedDeskSlug,
            currentClass: declareAssetClass
        )
        // A dated string on a desk that cannot serve it is a refusal, not a silent drop.
        guard bind.shape == .binanceOption, let parts = DeskDatedContractFields.parse(trimmed) else {
            refuseSelection(hint: deskInstrumentHint)
            return
        }

        symbolSearchTask?.cancel()
        ignoreSymbolSearchUntilEdit = true
        showSymbolSuggestions = false
        symbolSuggestions = []
        symbolSearchHint = nil

        // The contract is the source for these fields — an expiry it cannot name is cleared,
        // never left standing from the contract before it.
        barDeclarationSymbol = parts.underlying
        declOptionExpiry = parts.expiry
        declOptionStrike = parts.strike
        declOptionRight = parts.right
        barDeclarationLastError = nil
        barLtpFetchError = nil

        // The tab is not raised to `.options` here: picking a contract does not silently arm
        // the Options surface, same as `selectSymbol`.
        //
        // Order matters: the id first, then invalidate (it reads the id to decide whether
        // Last may survive), then fetch — which captures the generation it must match.
        bindDeskSelectedInstrumentId(bind.tickBookId)
        selectedMarketBookId = bind.bookId ?? Self.marketBook(for: bind.tickBookId)
        adoptDeskTicket()
        invalidateDeskMarketExtracts(reason: "commit-symbol")
        // No `applyLTP` seed: there is no catalog row here, and a spot-shaped last never
        // seeds a premium.
        fetchStationQuote(instrument: bind.tickBookId)
        refreshDeskExtracts(symbol: bind.chainUnderlying, instrumentId: bind.tickBookId)
    }

    /// Typed pair on USDM / Coin-M. Leftover dated contracts and Kotak tokens stay unbound.
    private func commitNamedFuturesPair(_ raw: String) {
        guard let ticker = BarBrokerTicker.normalize(raw: raw) else {
            refuseSelection(hint: "Symbol must be a broker ticker (e.g. CATIUSDT), not a company name.")
            return
        }
        if InstrumentTickBookId.isDatedOptionContract(ticker)
            || InstrumentTickBookId.isNfoIdentity(ticker)
            || InstrumentTickBookId.isCashIdentity(ticker)
        {
            refuseSelection(hint: deskInstrumentHint)
            return
        }
        guard DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug) else {
            refuseSelection(hint: deskInstrumentHint)
            return
        }
        let bind = DeskInstrumentBind.resolve(
            rawId: ticker,
            slug: resolvedDeskSlug,
            currentClass: declareAssetClass
        )
        symbolSearchTask?.cancel()
        ignoreSymbolSearchUntilEdit = true
        showSymbolSuggestions = false
        symbolSuggestions = []
        symbolSearchHint = nil
        barDeclarationSymbol = ticker
        barDeclarationLastError = nil
        barLtpFetchError = nil
        bindDeskSelectedInstrumentId(bind.tickBookId)
        selectedMarketBookId = bind.bookId ?? Self.marketBook(for: bind.tickBookId)
        adoptDeskTicket()
        invalidateDeskMarketExtracts(reason: "commit-symbol")
        fetchStationQuote(instrument: bind.tickBookId)
        refreshDeskExtracts(symbol: bind.chainUnderlying, instrumentId: bind.tickBookId)
    }

    /// Catalog row click — same path as paste. A leftover NFO token is not a
    /// dated contract, so it does not bind.
    func bindChainCatalogSymbol(_ symbol: String) {
        let trimmed = symbol.trimmingCharacters(in: .whitespacesAndNewlines)
        guard InstrumentTickBookId.isDatedOptionContract(trimmed) else { return }
        barDeclarationSymbol = trimmed
        commitDeskSymbol()
    }

    /// Hint for an instrument the active desk cannot serve.
    private var deskInstrumentHint: String {
        DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug)
            ? "Select a Binance instrument"
            : kotakInstrumentHint
    }

    private var kotakInstrumentHint: String {
        declareAssetClass == .options
            ? "Select a Kotak NFO instrument"
            : "Select a Kotak cash instrument"
    }

    /// Active execution broker slug for desk-honest Last/history routing.
    var resolvedDeskSlug: String? {
        let raw = (activeExecutionBrokerSlug ?? barProtectiveBrokerSlug)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return raw.isEmpty ? nil : raw
    }

    /// Asset-class pills on Plan — filtered by connected desk, not global `CaseIterable`.
    var planDeclareAssetClassTabs: [BarDeclareAssetClass] {
        BarDeclareAssetClass.supported(forDeskSlug: resolvedDeskSlug)
    }

    /// Snap declare tab when the execution desk changes (Binance default Spot, Kotak default Equity).
    func reconcileDeclareAssetClassForConnectedDesk() {
        let next = BarDeclareAssetClass.reconciled(
            current: declareAssetClass,
            forDeskSlug: resolvedDeskSlug
        )
        if next != declareAssetClass {
            declareAssetClass = next
        }
    }

    /// Named market book implied by instrument shape — independent of execution desk.
    static func marketBook(for instrumentId: String) -> String? {
        BarDeskTemplate.marketBook(for: instrumentId)
    }

    /// Whether the active execution broker can place orders for the selected instrument.
    /// Kotak market books require `kotak_neo`; `binance-com-options` is public data only.
    /// USDM may declare a plan; venue TRADE / auto-place SL stay off.
    func canExecuteSelectedInstrument() -> Bool {
        if declareAssetClass.isNamedComFutures {
            return false
        }
        let book = Self.marketBook(for: deskSelectedInstrumentId) ?? selectedMarketBookId
        guard let book else {
            return DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug)
                && !InstrumentTickBookId.isDatedOptionContract(deskSelectedInstrumentId)
        }
        if book == BarDeskTemplate.binanceComOptionsBookId {
            return false
        }
        if book == BarDeskTemplate.kotakNfoBookId || book == BarDeskTemplate.kotakCashBookId {
            return BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
        }
        return false
    }

    var showsConfirmControl: Bool {
        BarDeskTemplate.showsConfirmControl(for: declareAssetClass)
    }

    var requiresCashProduct: Bool { declareAssetClass == .equity }

    var declGateStripState: BarPlanGateStripState {
        get {
            BarPlanGateStripState(
                capitalAck: declGateCapital,
                onePercentAck: declGateOnePercent,
                maxLossAck: declGateMaxLoss,
                hedgeAck: declGateHedge,
                reviewAck: declGateReview,
            )
        }
        set {
            declGateCapital = newValue.capitalAck
            declGateOnePercent = newValue.onePercentAck
            declGateMaxLoss = newValue.maxLossAck
            declGateHedge = newValue.hedgeAck
            declGateReview = newValue.reviewAck
        }
    }

    var canSubmitBarDeclaration: Bool {
        BarDeskTemplate.canSubmitBarDeclaration(for: declareAssetClass)
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
        if declareAssetClass == .options || declareAssetClass.isNamedComFutures {
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
            deskQuoteLast = nil
            return
        }
        guard let url = URL(string: "\(baseURL())\(deskQuoteExtractPath(instrument: instrument))") else {
            return
        }
        let generation = deskExtractGeneration
        Task {
            let req = authorizedRequest(url: url)
            do {
                let (data, _) = try await URLSession.shared.data(for: req)
                guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                    return
                }
                await MainActor.run {
                    guard generation == self.deskExtractGeneration else { return }
                    applyStationQuoteEnvelope(json)
                    // COM `@depth` bind happens on this quote. Glance after
                    // apply — not in parallel with it — so DepthBook wait can see the bind.
                    scheduleDeferredComSpotDepthGlance()
                }
            } catch {
                // silent — quote failure must not block declaration
            }
        }
    }

    /// COM spot depth after quote bind. `refreshDeskExtracts` does not issue this
    /// glance: it races the ticker and reads an empty DepthBook as Unavailable.
    /// Reconstruction (WS + limit=5000) can land after Last is already Fresh —
    /// keep asking until Success, Unusable, or the 15s budget.
    func scheduleDeferredComSpotDepthGlance() {
        let plan = DeskExtractPlan.resolve(
            slug: resolvedDeskSlug,
            assetClass: declareAssetClass,
            instrumentId: deskSelectedInstrumentId
        )
        guard plan.defersComSpotDepth else { return }
        let generation = deskExtractGeneration
        let symbol = deskSelectedInstrumentId.isEmpty ? barDeclarationSymbol : deskSelectedInstrumentId
        let path = deskDepthExtractPath(symbol: symbol)
        Task { [weak self] in
            let deadline = Date().addingTimeInterval(15)
            while Date() < deadline {
                guard let self else { return }
                guard generation == self.deskExtractGeneration else { return }
                let json = await self.getExtractJSON(path)
                guard generation == self.deskExtractGeneration else { return }
                let status = (json?["status"] as? String)?
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                    .lowercased() ?? "unavailable"
                self.applyStationDepthEnvelope(json ?? [:])
                if !Self.shouldRetryComSpotDepthGlance(status: status) {
                    return
                }
                try? await Task.sleep(nanoseconds: 400_000_000)
            }
        }
    }

    /// Keep polling only while the book has not spoken. Unusable is a gap —
    /// do not wait it into last-good.
    static func shouldRetryComSpotDepthGlance(status: String) -> Bool {
        let trimmed = status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return trimmed != "success" && trimmed != "unusable"
    }

    /// Bind Last from a Station quote extract (testable without network).
    func applyStationQuoteEnvelope(_ json: [String: Any]) {
        let adapter = ((json["provenance"] as? [String: Any])?["adapter_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        let instrumentId = (json["instrument_id"] as? String) ?? ""
        let bookId = (json["book_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if !shouldBindQuoteLast(adapter: adapter, instrumentId: instrumentId, bookId: bookId) {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
            deskTickSize = nil
            deskStepSize = nil
            deskQuoteLast = nil
            return
        }
        deskTickSize = stringField(json["tick_size"])
        deskStepSize = stringField(json["step_size"])
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
        deskQuoteLast = nil
        if let data = json["data"] as? [String: Any],
           let last = Self.parseQuoteLastRaw(data),
           last.value > 0
        {
            barLtpFetchError = nil
            applyLTP(last.value, rawLast: last.raw)
            if SessionChartQuoteLast.isBound(status: deskLastStatus) {
                deskQuoteLast = last.value
            }
        }
    }

    /// Options last is market-book scoped: a Kotak NFO TickBook id, or a dated Binance
    /// contract on `binance-com-options`. A Binance *pair* must not paint the options
    /// declare, so the instrument shape is the discriminator there.
    /// USDM last is **class + `book=`** — `BTCUSDT` is shared with spot, so leftover
    /// TickBook / spot envelopes must not fill this hole.
    func shouldBindQuoteLast(adapter: String?, instrumentId: String, bookId: String? = nil) -> Bool {
        if declareAssetClass.isNamedComFutures {
            let trimmed = instrumentId.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty { return false }
            if InstrumentTickBookId.isDatedOptionContract(trimmed) { return false }
            if InstrumentTickBookId.isNfoIdentity(trimmed) { return false }
            if InstrumentTickBookId.isCashIdentity(trimmed) { return false }
            let wanted = declareBookId
                ?? (declareAssetClass == .coinm
                    ? BarDeskTemplate.binanceComCoinmBookId
                    : BarDeskTemplate.binanceComUsdmBookId)
            let named = bookId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if !named.isEmpty {
                return named == wanted
            }
            // Envelope without a named book is leftover spot TickBook last.
            if adapter != nil {
                return false
            }
            return DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug)
        }
        if declareAssetClass == .options {
            if BarDeskTemplate.isBinanceOptionsSelection(
                assetClass: declareAssetClass,
                instrumentId: instrumentId
            ) {
                return true
            }
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

    /// `rawLast` is the wire string the Double was parsed from, kept verbatim on the crypto
    /// options path where `%.2f` would round a sub-cent premium to `0.00`.
    func applyLTP(_ ltp: Double, rawLast: String? = nil) {
        let cryptoOptions = BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: deskSelectedInstrumentId
        )
        if declareAssetClass == .options, !cryptoOptions {
            if DeskCatalogAllowlist.isBinanceDesk(resolvedDeskSlug) { return }
            if !BarDeskTemplate.isKotakNfoDesk(slug: resolvedDeskSlug, assetClass: declareAssetClass) {
                return
            }
        }
        if declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            || declEntryPrice == "0"
        {
            if declareAssetClass.isNamedComFutures, deskTickSize != nil {
                declEntryPrice = VenueLotTick.format(ltp, stepSize: deskTickSize, rawLast: rawLast)
            } else {
                declEntryPrice = BarDeskLastFormatting.entryPrice(
                    rawLast: rawLast,
                    value: ltp,
                    preservesPrecision: cryptoOptions
                )
            }
        }
    }

    private func reconcileDeskLastForAssetClass(previousClass: BarDeclareAssetClass? = nil) {
        // Wipe first: the old class's chain/OI belong to a book this class may not read.
        invalidateDeskMarketExtracts(reason: "asset-class")
        // Same letters (`BTCUSDT`) on USDM vs spot are two books — leftover last must not stand.
        if declareAssetClass.isNamedComFutures || previousClass?.isNamedComFutures == true {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
            declEntryPrice = ""
            deskTickSize = nil
            deskStepSize = nil
            deskQuoteLast = nil
        }
        if declareAssetClass == .options {
            if shouldBindQuoteLast(adapter: nil, instrumentId: deskSelectedInstrumentId) {
                fetchStationQuote(instrument: deskSelectedInstrumentId)
            }
            refreshDeskExtracts(symbol: barDeclarationSymbol, instrumentId: deskSelectedInstrumentId)
            return
        }
        if InstrumentTickBookId.isNfoIdentity(deskSelectedInstrumentId) {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
        }
        if shouldBindQuoteLast(adapter: nil, instrumentId: deskSelectedInstrumentId) {
            fetchStationQuote(instrument: deskSelectedInstrumentId)
        }
        refreshDeskExtracts(symbol: barDeclarationSymbol, instrumentId: deskSelectedInstrumentId)
    }

    static func parseQuoteLast(_ data: [String: Any]) -> Double? {
        parseQuoteLastRaw(data)?.value
    }

    /// Parsed last plus the wire string it came from (nil when the wire sent a number).
    /// Sub-cent premiums survive only as the string — re-formatting the Double loses them.
    static func parseQuoteLastRaw(_ data: [String: Any]) -> (value: Double, raw: String?)? {
        if let s = data["last"] as? String {
            guard let d = Double(s) else { return nil }
            return (d, s)
        }
        if let d = data["last"] as? Double {
            return (d, nil)
        }
        if let i = data["last"] as? Int {
            return (Double(i), nil)
        }
        return nil
    }

    /// Bind History from a Station history extract (testable without network).
    /// Crypto Options session chart consumes eapi candles.
    /// Spot paints `binance_klines`. USDM paints `fapi_klines`. Coin-M paints `dapi_klines`.
    /// Kotak cash paints native Neo. Yahoo and licensed-history stay off a shipping COM Session.
    /// A declared gap vendor may paint labs series. Spot `/api/station/history` never paints a dated contract.
    func applyStationHistoryEnvelope(_ json: [String: Any]) {
        deskYahooHistoryStatus = "unavailable"
        deskYahooHistoryIneligible = []

        let adapter = ((json["provenance"] as? [String: Any])?["adapter_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let provenanceAdapter = (json["provenance_adapter_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let envelopeAdapter = (json["adapter_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let dataSource = ((json["data"] as? [String: Any])?["source"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let status = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? "unavailable"
        let bookId = (json["book_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if adapter == "yahoo" || adapter == "yahoo_chart"
            || dataSource == "yahoo" || dataSource == "yahoo_chart"
            || status == "research_segment"
            || provenanceAdapter == "yahoo" || provenanceAdapter == "yahoo_chart"
        {
            setDeskHistoryCandles([])
            return
        }

        let kotakDesk = BarDeskTemplate.isKotakNeoDesk(slug: resolvedDeskSlug)
        if kotakDesk {
            let binanceShaped = adapter == "binance_com"
                || provenanceAdapter == "binance_com"
                || envelopeAdapter == "binance_com"
                || bookId == "binance-com-spot"
                || dataSource == "binance_klines"
            let vendorGap = provenanceAdapter == "licensed_history"
                || envelopeAdapter == "licensed_history"
                || bookId == "licensed-history"
            let nativeNeo = !vendorGap && (
                provenanceAdapter == "kotak_neo"
                || envelopeAdapter == "kotak_neo"
                || adapter == "kotak_neo"
                || dataSource == "kotak_neo"
                || dataSource == "kotak_neo_historical"
                || bookId == BarDeskTemplate.kotakCashBookId
            )
            if binanceShaped && !vendorGap {
                deskHistoryStatus = "unsupported"
                deskHistoryIneligible = []
                setDeskHistoryCandles([])
                deskHistoryProductUse = nil
                deskHistoryBookId = nil
                return
            }
            if vendorGap {
                applyKotakVendorHistory(json: json, status: status)
                return
            }
            if nativeNeo {
                applyKotakNativeHistory(json: json, status: status)
                return
            }
            deskHistoryStatus = "unsupported"
            deskHistoryIneligible = []
            setDeskHistoryCandles([])
            deskHistoryProductUse = nil
            deskHistoryBookId = nil
            return
        }

        let cryptoOptions = BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: deskSelectedInstrumentId
        )
        if cryptoOptions {
            applyOptionsSessionHistory(
                json: json,
                status: status,
                dataSource: dataSource,
                bookId: bookId
            )
            return
        }

        if declareAssetClass == .usdm {
            applyNamedFuturesSessionHistory(
                json: json,
                status: status,
                dataSource: dataSource,
                bookId: bookId,
                expectedBookId: BarDeskTemplate.binanceComUsdmBookId,
                expectedSource: "fapi_klines"
            )
            return
        }
        if declareAssetClass == .coinm {
            applyNamedFuturesSessionHistory(
                json: json,
                status: status,
                dataSource: dataSource,
                bookId: bookId,
                expectedBookId: BarDeskTemplate.binanceComCoinmBookId,
                expectedSource: "dapi_klines"
            )
            return
        }

        deskHistoryStatus = status
        deskHistoryIneligible = stringList(json["ineligible"]).filter {
            $0 != "rights_forbid_canonical"
        }
        deskHistoryProductUse = nil
        deskHistoryBookId = (json["book_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let spotKlines = dataSource == "binance_klines"
            || bookId == BarDeskTemplate.binanceComSpotBookId
        let rawCandles = (json["data"] as? [String: Any])?["candles"]
        if spotKlines {
            let candles = Self.parseSessionCandles(rawCandles)
            if status == "success", !candles.isEmpty {
                setDeskHistoryCandles(candles, interval: historyInterval(in: json))
                return
            }
            if status == "success", rawCandles != nil {
                deskHistoryStatus = "unavailable"
            }
        }
        setDeskHistoryCandles([])
    }

    /// Native Neo cash series. Never COM klines. Empty success is a hole.
    private func applyKotakNativeHistory(json: [String: Any], status: String) {
        deskHistoryProductUse = (json["product_use"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        deskHistoryBookId = (json["book_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let candles = Self.parseSessionCandles((json["data"] as? [String: Any])?["candles"])
        if status == "success", !candles.isEmpty {
            deskHistoryStatus = "success"
            deskHistoryIneligible = []
            setDeskHistoryCandles(candles, interval: historyInterval(in: json))
            return
        }
        deskHistoryStatus = status.isEmpty ? "unavailable" : status
        if deskHistoryStatus == "success" {
            deskHistoryStatus = "unavailable"
        }
        deskHistoryIneligible = stringList(json["ineligible"]).filter {
            $0 != "rights_forbid_canonical"
        }
        setDeskHistoryCandles([])
    }

    /// Declared-gap vendor series on a Kotak desk. Never Kotak last. Never COM klines.
    private func applyKotakVendorHistory(json: [String: Any], status: String) {
        deskHistoryProductUse = (json["product_use"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        deskHistoryBookId = (json["book_id"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let candles = Self.parseSessionCandles((json["data"] as? [String: Any])?["candles"])
        if status == "success", !candles.isEmpty {
            deskHistoryStatus = "success"
            deskHistoryIneligible = []
            setDeskHistoryCandles(candles, interval: historyInterval(in: json))
            return
        }
        deskHistoryStatus = status.isEmpty ? "unavailable" : status
        if deskHistoryStatus == "success" {
            deskHistoryStatus = "unavailable"
        }
        deskHistoryIneligible = stringList(json["ineligible"]).filter {
            $0 != "rights_forbid_canonical"
        }
        setDeskHistoryCandles([])
    }

    private func applyOptionsSessionHistory(
        json: [String: Any],
        status: String,
        dataSource: String,
        bookId: String
    ) {
        if bookId == "binance-com-spot"
            || dataSource == "binance_klines"
        {
            deskHistoryStatus = "unavailable"
            deskHistoryIneligible = []
            setDeskHistoryCandles([])
            return
        }
        let candles = Self.parseSessionCandles((json["data"] as? [String: Any])?["candles"])
        if status == "success", !candles.isEmpty {
            deskHistoryStatus = "success"
            deskHistoryIneligible = []
            setDeskHistoryCandles(candles, interval: historyInterval(in: json))
            return
        }
        deskHistoryStatus = status == "unsupported" ? "unavailable" : (status.isEmpty ? "unavailable" : status)
        if deskHistoryStatus == "success" {
            deskHistoryStatus = "unavailable"
        }
        deskHistoryIneligible = stringList(json["ineligible"]).filter {
            $0 != "rights_forbid_canonical"
        }
        setDeskHistoryCandles([])
    }

    /// USDM `fapi_klines` / Coin-M `dapi_klines` only. Never spot, eapi, Yahoo, or licensed-history.
    private func applyNamedFuturesSessionHistory(
        json: [String: Any],
        status: String,
        dataSource: String,
        bookId: String,
        expectedBookId: String,
        expectedSource: String
    ) {
        let mix = dataSource == "binance_klines"
            || dataSource == "eapi_klines"
            || dataSource == "yahoo"
            || dataSource == "yahoo_chart"
            || bookId == "licensed-history"
            || bookId == BarDeskTemplate.binanceComSpotBookId
            || (dataSource == "fapi_klines" && expectedSource != "fapi_klines")
            || (dataSource == "dapi_klines" && expectedSource != "dapi_klines")
        if mix || bookId != expectedBookId || dataSource != expectedSource {
            deskHistoryStatus = "unavailable"
            deskHistoryIneligible = []
            setDeskHistoryCandles([])
            deskHistoryProductUse = nil
            deskHistoryBookId = bookId.isEmpty ? nil : bookId
            return
        }
        let candles = Self.parseSessionCandles((json["data"] as? [String: Any])?["candles"])
        if status == "success", !candles.isEmpty {
            deskHistoryStatus = "success"
            deskHistoryIneligible = []
            setDeskHistoryCandles(candles, interval: historyInterval(in: json))
            deskHistoryProductUse = nil
            deskHistoryBookId = expectedBookId
            return
        }
        deskHistoryStatus = status == "unsupported" ? "unavailable" : (status.isEmpty ? "unavailable" : status)
        if deskHistoryStatus == "success" {
            deskHistoryStatus = "unavailable"
        }
        deskHistoryIneligible = stringList(json["ineligible"]).filter {
            $0 != "rights_forbid_canonical"
        }
        setDeskHistoryCandles([])
        deskHistoryProductUse = nil
        deskHistoryBookId = expectedBookId
    }

    static func parseSessionCandles(_ raw: Any?) -> [DeskSessionCandle] {
        guard let rows = raw as? [[String: Any]] else { return [] }
        return rows.compactMap { row in
            let openTime = VenuePosture.parseInt64(row["open_time_ms"])
            let open = stringScalar(row["open"])
            let high = stringScalar(row["high"])
            let low = stringScalar(row["low"])
            let close = stringScalar(row["close"])
            let volume = stringScalar(row["volume"])
            guard let openTime, let open, let high, let low, let close, let volume else {
                return nil
            }
            return DeskSessionCandle(
                openTimeMs: openTime,
                open: open,
                high: high,
                low: low,
                close: close,
                volume: volume
            )
        }
    }

    /// Envelope `data.interval`. Empty or missing stays nil — the chart does not invent 1m.
    private func historyInterval(in json: [String: Any]) -> String? {
        guard let raw = (json["data"] as? [String: Any])?["interval"] as? String else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    private func setDeskHistoryCandles(_ candles: [DeskSessionCandle], interval: String? = nil) {
        deskHistoryCandles = candles
        if candles.isEmpty {
            deskHistoryInterval = nil
        } else {
            let trimmed = interval?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            deskHistoryInterval = trimmed.isEmpty ? nil : trimmed
        }
    }

    /// Bound quote last for the Session last-line. History close never substitutes.
    var sessionChartLast: Double? {
        SessionChartQuoteLast.value(deskQuoteLast, status: deskLastStatus)
    }

    private static func stringScalar(_ raw: Any?) -> String? {
        if let s = raw as? String {
            let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
            return t.isEmpty ? nil : t
        }
        if let n = raw as? NSNumber { return n.stringValue }
        return nil
    }

    /// Drop every market extract bound to the previous instrument/book, and move the
    /// generation so any in-flight fetch started before this call cannot apply.
    ///
    /// Chain and OI always go dark: they are book-scoped, and the book changes with the
    /// asset class and the broker slug. Last goes dark only when the new binding is not
    /// allowed to show one (Binance options, a cash token under an options declare) —
    /// otherwise the fetch that follows this call repaints it.
    ///
    /// Call this *before* `refreshDeskExtracts`, never after: the fetch captures the
    /// generation it must still match at apply time.
    @discardableResult
    func invalidateDeskMarketExtracts(reason: String) -> UInt64 {
        deskExtractGeneration &+= 1
        deskExtractInvalidationReason = reason
        deskChainStatus = "unavailable"
        clearDeskChainRows()
        deskOiStatus = "unavailable"
        clearDeskOiNumbers()
        clearDeskDepth()
        clearDeskGreeks()
        clearDeskIndex()
        deskHistoryStatus = "unavailable"
        deskHistoryIneligible = []
        setDeskHistoryCandles([])
        deskTickSize = nil
        deskStepSize = nil
        deskQuoteLast = nil
        if !shouldBindQuoteLast(adapter: nil, instrumentId: deskSelectedInstrumentId) {
            deskLastStatus = "unavailable"
            deskQuoteCapability = "unavailable"
        }
        return deskExtractGeneration
    }

    /// Testable quote path. Dated contracts name `binance-com-options`; USDM / Coin-M name
    /// their books from **class**, not instrument shape (`BTCUSDT` is shared with spot).
    /// Kotak identities stay bookless on quote — no other desk sends `book=` on a quote at all.
    func deskQuoteExtractPath(instrument: String) -> String {
        let encoded = InstrumentTickBookId.queryEncode(instrument)
        if declareAssetClass == .usdm {
            let encodedBook = InstrumentTickBookId.queryEncode(BarDeskTemplate.binanceComUsdmBookId)
            return "/api/station/quote?instrument=\(encoded)&book=\(encodedBook)"
        }
        if declareAssetClass == .coinm {
            let encodedBook = InstrumentTickBookId.queryEncode(BarDeskTemplate.binanceComCoinmBookId)
            return "/api/station/quote?instrument=\(encoded)&book=\(encodedBook)"
        }
        guard let book = Self.marketBook(for: instrument),
              book == BarDeskTemplate.binanceComOptionsBookId
        else {
            return "/api/station/quote?instrument=\(encoded)"
        }
        let encodedBook = InstrumentTickBookId.queryEncode(book)
        return "/api/station/quote?instrument=\(encoded)&book=\(encodedBook)"
    }

    /// Testable search path. Named futures books use obtain `operation=search` — never slug `/instruments/search`.
    func deskSearchExtractPath(query: String) -> String {
        let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? query
        if let book = declareBookId,
           let adapter = BarAccountChrome.obtainAdapterId(forStartSlug: resolvedDeskSlug)
        {
            return BarAccountChrome.obtainPath(adapter: adapter, bookId: book, operation: "search")
                + "&q=\(encoded)"
        }
        return "/instruments/search?q=\(encoded)"
    }

    /// Book-keyed obtain search. A spot catalog payload on the USDM tab is dropped.
    func applyObtainSearchEnvelope(_ json: [String: Any], expectedBook: String) {
        let named = (json["book_id"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard named == expectedBook else {
            symbolSuggestions = []
            showSymbolSuggestions = false
            symbolSearchHint = nil
            return
        }
        let status = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        guard status == "success" else {
            symbolSuggestions = []
            showSymbolSuggestions = false
            return
        }
        let rows = (json["data"] as? [String: Any])?["rows"] as? [[String: Any]] ?? []
        let decoded: [InstrumentResult]
        if let data = try? JSONSerialization.data(withJSONObject: rows),
           let parsed = try? JSONDecoder().decode([InstrumentResult].self, from: data)
        {
            decoded = parsed
        } else {
            decoded = []
        }
        let filtered = DeskCatalogAllowlist.filterSymbols(
            decoded,
            deskSlug: resolvedDeskSlug
        )
        if symbolSuggestions != filtered {
            symbolSuggestions = filtered
        }
        showSymbolSuggestions = !filtered.isEmpty
        symbolSearchHint = filtered.isEmpty
            ? DeskCapabilityChrome.emptySearchHint(
                masterStatus: deskInstrumentsCapability,
                connectedInstrumentDesk: connectedInstrumentCatalogDesk
            )
            : nil
    }

    /// Dated crypto Options session series. Never Kotak history, never spot `/api/v3/klines`.
    func deskOptionsHistoryExtractPath() -> String {
        "/api/station/obtain?adapter=binance_com&book=binance-com-options&operation=history"
    }

    /// USDM public klines. Never `/api/v3/klines`, never eapi, never dapi.
    func deskUsdmHistoryExtractPath() -> String {
        "/api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=history"
    }

    /// Coin-M public klines. Never fapi, never spot, never eapi.
    func deskCoinmHistoryExtractPath() -> String {
        "/api/station/obtain?adapter=binance_com&book=binance-com-coinm&operation=history"
    }

    /// Testable chain glance path. Kotak names `deskBookId` and an underlying ticker.
    /// Crypto Options names `binance-com-options` and the dated contract — bind `bookId`
    /// stays nil. A leftover typed `BTC` must not replace `BTC-200730-9000-C`.
    func deskChainExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.path(
            bookId: glanceBookId(symbol: symbol),
            underlying: glanceInstrument(symbol: symbol)
        )
    }

    /// Same `book=` + instrument as chain. Do not send a TickBook token as the OI instrument.
    func deskOiExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.oiPath(
            bookId: glanceBookId(symbol: symbol),
            underlying: glanceInstrument(symbol: symbol)
        )
    }

    func deskDepthExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.depthPath(
            bookId: depthBookId(symbol: symbol),
            instrument: depthInstrument(symbol: symbol)
        )
    }

    /// Book-specific ladder copy. The envelope physics word is never rewritten to `synced`.
    var deskDepthPhysicsNote: String {
        let symbol = deskSelectedInstrumentId.isEmpty ? barDeclarationSymbol : deskSelectedInstrumentId
        return BarDeskTemplate.depthPhysicsNote(
            bookId: depthBookId(symbol: symbol),
            physics: deskDepthPhysics
        )
    }

    /// Same `book=` + instrument as chain and OI. Greeks are per contract, so a leftover
    /// typed `BTC` must never replace the selected dated contract here either.
    func deskGreeksExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.greeksPath(
            bookId: glanceBookId(symbol: symbol),
            underlying: glanceInstrument(symbol: symbol)
        )
    }

    /// Same `book=` + instrument as chain. Index S is catalog `underlying=`, never OI's asset.
    func deskIndexExtractPath(symbol: String) -> String {
        DeskChainExtractQuery.indexPath(
            bookId: glanceBookId(symbol: symbol),
            underlying: glanceInstrument(symbol: symbol)
        )
    }

    /// NFO glance is keyed on `pSymbolName`. Eapi chain/OI need the mixed-case contract
    /// so `optionSymbols` can match and `openInterest` can take YYMMDD.
    private func glanceInstrument(symbol: String) -> String {
        let selected = deskSelectedInstrumentId.isEmpty ? symbol : deskSelectedInstrumentId
        if BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: selected
        ) {
            return selected
        }
        return DeskChainExtractQuery.underlyingTicker(
            preferred: symbol,
            declarationSymbol: barDeclarationSymbol
        )
    }

    private func glanceBookId(symbol: String) -> String? {
        if declareAssetClass == .usdm {
            return BarDeskTemplate.binanceComUsdmBookId
        }
        if declareAssetClass == .coinm {
            return BarDeskTemplate.binanceComCoinmBookId
        }
        let instrument = deskSelectedInstrumentId.isEmpty ? symbol : deskSelectedInstrumentId
        if BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: instrument
        ) {
            return BarDeskTemplate.binanceComOptionsBookId
        }
        return BarDeskTemplate.deskBookId(slug: resolvedDeskSlug, assetClass: declareAssetClass)
    }

    private func depthInstrument(symbol: String) -> String {
        if declareAssetClass == .usdm || declareAssetClass == .coinm {
            let selected = deskSelectedInstrumentId.isEmpty ? symbol : deskSelectedInstrumentId
            return selected.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        let selected = deskSelectedInstrumentId.isEmpty ? symbol : deskSelectedInstrumentId
        let trimmed = selected.trimmingCharacters(in: .whitespacesAndNewlines)
        if BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: trimmed
        ) {
            return trimmed
        }
        if trimmed.contains("|") {
            return trimmed
        }
        if declareAssetClass == .spot {
            return trimmed
        }
        return ""
    }

    private func depthBookId(symbol: String) -> String? {
        if declareAssetClass == .usdm {
            return BarDeskTemplate.binanceComUsdmBookId
        }
        if declareAssetClass == .coinm {
            return BarDeskTemplate.binanceComCoinmBookId
        }
        let instrument = deskSelectedInstrumentId.isEmpty ? symbol : deskSelectedInstrumentId
        if BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: instrument
        ) {
            return BarDeskTemplate.binanceComOptionsBookId
        }
        if let market = BarDeskTemplate.marketBook(for: instrument) {
            return market
        }
        if declareAssetClass == .spot {
            return BarDeskTemplate.binanceComSpotBookId
        }
        return BarDeskTemplate.deskBookId(slug: resolvedDeskSlug, assetClass: declareAssetClass)
    }

    /// Apply one `/api/station/option_chain` envelope. Pure state, no HTTP —
    /// the parse is the seam under test.
    ///
    /// Success with `rows` keeps mixed-case `instrument_id` / `trading_symbol`,
    /// `strike_raw`, `option_type` (side), `expiry_raw`. Last overlays only when
    /// the venue string is present and not `"0"`. Numeric JSON does not paint.
    func applyStationChainEnvelope(_ json: [String: Any]) {
        let rawStatus = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        clearDeskChainRows()
        deskChainStatus = rawStatus.isEmpty ? "unavailable" : rawStatus

        guard rawStatus.lowercased() == "success" else { return }
        guard let data = json["data"] as? [String: Any],
              let rawRows = data["rows"] as? [Any]
        else { return }
        let parsed = rawRows.compactMap(Self.deskChainRow(from:))
        if !parsed.isEmpty {
            deskChainRows = parsed
            syncDeskSelectedInstrumentTypeFromChainRows(parsed)
        }
    }

    private func syncDeskSelectedInstrumentTypeFromChainRows(_ rows: [DeskChainCatalogRow]) {
        let selected = deskSelectedInstrumentId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !selected.isEmpty else { return }
        guard let row = rows.first(where: { $0.instrumentId == selected }),
              let instType = row.instrumentType?.trimmingCharacters(in: .whitespacesAndNewlines),
              !instType.isEmpty
        else { return }
        deskSelectedInstrumentType = instType
    }

    private static func deskChainRow(from raw: Any) -> DeskChainCatalogRow? {
        guard let obj = raw as? [String: Any] else { return nil }
        let instrumentId = publishedVenueString(obj["instrument_id"])
        let symbol = publishedVenueString(obj["trading_symbol"])
            ?? instrumentId
            ?? publishedVenueString(obj["symbol"])
        guard let symbol else { return nil }
        let last = publishedVenueString(obj["last"])
        return DeskChainCatalogRow(
            symbol: symbol,
            instrumentId: instrumentId,
            instrumentType: publishedVenueString(obj["instrument_type"]),
            strikeRaw: publishedVenueString(obj["strike_raw"]),
            side: publishedVenueString(obj["option_type"]) ?? publishedVenueString(obj["side"]),
            expiryRaw: publishedVenueString(obj["expiry_raw"]),
            last: (last == "0") ? nil : last
        )
    }

    private func clearDeskChainRows() {
        deskChainRows = []
    }

    /// Apply one `/api/station/open_interest` envelope. Pure state, no HTTP —
    /// the parse is the seam under test.
    ///
    /// Exact mixed-case symbol match → copy `sumOpenInterest` / Usd / timestamp /
    /// `symbol` as published strings. Rows-only → keep the list, leave the
    /// single-contract fields nil, never sum. Empty string is missing, not `"0"`.
    /// A venue `"0"` that actually arrived as a string may paint. Numeric JSON
    /// is not the contract — never `as? Double`.
    func applyStationOiEnvelope(_ json: [String: Any]) {
        let rawStatus = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        clearDeskOiNumbers()
        deskOiStatus = rawStatus.isEmpty ? "unavailable" : rawStatus

        guard rawStatus.lowercased() == "success" else { return }
        guard let data = json["data"] as? [String: Any] else { return }

        if let field = Self.publishedVenueString(data["field"]), field == "open_int",
           let openInt = Self.publishedVenueString(data["open_interest"])
        {
            deskOiOpenInt = openInt
            deskOiField = field
            return
        }

        if let rawRows = data["rows"] as? [Any] {
            let parsed = rawRows.compactMap(Self.deskOiRow(from:))
            if !parsed.isEmpty {
                deskOiRows = parsed
            }
            return
        }

        guard let sum = Self.publishedVenueString(data["sumOpenInterest"]) else { return }
        deskOiSumOpenInterest = sum
        deskOiSumOpenInterestUsd = Self.publishedVenueString(data["sumOpenInterestUsd"])
        deskOiTimestamp = Self.publishedVenueString(data["timestamp"])
        deskOiSymbol = Self.publishedVenueString(data["symbol"])
    }

    /// Venue text only. Whitespace-only and non-strings are missing — not `"0"`.
    private static func publishedVenueString(_ raw: Any?) -> String? {
        guard let raw = raw as? String else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    private static func deskOiRow(from raw: Any) -> DeskOiExpiryRow? {
        guard let obj = raw as? [String: Any],
              let symbol = publishedVenueString(obj["symbol"])
        else { return nil }
        return DeskOiExpiryRow(
            symbol: symbol,
            sumOpenInterest: publishedVenueString(obj["sumOpenInterest"]),
            sumOpenInterestUsd: publishedVenueString(obj["sumOpenInterestUsd"]),
            timestamp: publishedVenueString(obj["timestamp"])
        )
    }

    /// Numbers and the expiry list — status is owned by the caller / envelope.
    private func clearDeskOiNumbers() {
        deskOiSumOpenInterest = nil
        deskOiSumOpenInterestUsd = nil
        deskOiTimestamp = nil
        deskOiSymbol = nil
        deskOiRows = []
        deskOiOpenInt = nil
        deskOiField = nil
    }

    /// Apply one `/api/station/depth` envelope. Unusable keeps an empty ladder.
    func applyStationDepthEnvelope(_ json: [String: Any]) {
        let rawStatus = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let incoming = rawStatus.isEmpty ? "unavailable" : rawStatus
        // Same generation: a late unbound glance must not wipe a ladder that
        // already landed. Unusable still replaces Success — that is a COM gap.
        if deskDepthStatus.lowercased() == "success",
           incoming.lowercased() == "unavailable"
        {
            return
        }
        let display = (json["rights"] as? [String: Any])?["display"] as? Bool ?? false
        clearDeskDepthLadder()
        deskDepthStatus = incoming
        deskDepthDisplay = display
        if let identity = json["identity"] as? [String: Any],
           let physics = identity["physics"] as? String
        {
            let trimmed = physics.trimmingCharacters(in: .whitespacesAndNewlines)
            deskDepthPhysics = trimmed.isEmpty ? "bounded_snapshot" : trimmed
        }
        guard display else { return }
        guard rawStatus.lowercased() == "success" else { return }
        guard let data = json["data"] as? [String: Any] else { return }
        deskDepthBids = Self.deskDepthLevels(from: data["bids"], side: "bid")
        deskDepthAsks = Self.deskDepthLevels(from: data["asks"], side: "ask")
    }

    private static func deskDepthLevels(from raw: Any?, side: String) -> [DeskDepthLevel] {
        guard let rows = raw as? [Any] else { return [] }
        return rows.compactMap { row in
            guard let obj = row as? [String: Any],
                  let price = publishedVenueString(obj["price"]),
                  let quantity = publishedVenueString(obj["quantity"])
            else { return nil }
            return DeskDepthLevel(
                side: side,
                price: price,
                quantity: quantity,
                orders: publishedVenueString(obj["orders"])
            )
        }
    }

    private func clearDeskDepthLadder() {
        deskDepthBids = []
        deskDepthAsks = []
        deskDepthPhysics = "bounded_snapshot"
    }

    private func clearDeskDepth() {
        deskDepthStatus = "unavailable"
        deskDepthDisplay = false
        clearDeskDepthLadder()
    }

    /// Apply one `/api/station/greeks` envelope. Pure state, no HTTP — the parse is the
    /// seam under test.
    ///
    /// The four numbers are painted only when the desk said `success`, the rights said
    /// `display`, and the venue published all four. Anything else leaves them nil and the
    /// grid shows a chip: three numbers and a hole would read as a partial pricer.
    ///
    /// Every greek is read as `String` — the venue's own text. `as? Double` would reparse
    /// and reprint a number the venue never published.
    func applyGreeksEnvelope(_ json: [String: Any]) {
        let status = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        let rights = json["rights"] as? [String: Any]
        let display = rights?["display"] as? Bool ?? false
        let data = json["data"] as? [String: Any]

        func published(_ key: String) -> String? {
            guard let raw = data?[key] as? String else { return nil }
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : trimmed
        }

        clearDeskGreeks()
        // The request went out and this is its reply — even an empty one. Set after the
        // clear, which is what resets it.
        deskGreeksAsked = true
        deskGreeksStatus = status.isEmpty ? "unavailable" : status
        deskGreeksDisplay = display
        deskGreeksIneligible = stringList(json["ineligible"])

        guard status == "success", display,
              let delta = published("delta"),
              let gamma = published("gamma"),
              let theta = published("theta"),
              let vega = published("vega")
        else { return }

        deskGreeksDelta = delta
        deskGreeksGamma = gamma
        deskGreeksTheta = theta
        deskGreeksVega = vega

        let provenance = json["provenance"] as? [String: Any]
        let source = json["source"] as? [String: Any]
        let model = (provenance?["model"] as? String) ?? (source?["kind"] as? String) ?? ""
        let path = (provenance?["path"] as? String) ?? (source?["path"] as? String) ?? ""
        deskGreeksProv = [model, path]
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: " · ")
    }

    /// Back to dark: status `unavailable`, no numbers, no display right, no provenance.
    private func clearDeskGreeks() {
        deskGreeksStatus = "unavailable"
        deskGreeksDelta = nil
        deskGreeksGamma = nil
        deskGreeksTheta = nil
        deskGreeksVega = nil
        deskGreeksDisplay = false
        deskGreeksProv = ""
        deskGreeksIneligible = []
        deskGreeksAsked = false
    }

    /// Apply one `/api/station/index` envelope. Pure state, no HTTP — the parse is the
    /// seam under test. Lock-named `indexPrice` only. Never `lastPrice` / `markPrice` / `c`.
    func applyStationIndexEnvelope(_ json: [String: Any]) {
        let status = (json["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        clearDeskIndex()
        deskIndexStatus = status.isEmpty ? "unavailable" : status
        guard status == "success" else { return }
        guard let data = json["data"] as? [String: Any] else { return }
        deskIndexPrice = Self.publishedVenueString(data["indexPrice"])
        deskIndexUnderlying = Self.publishedVenueString(data["underlying"])
    }

    private func clearDeskIndex() {
        deskIndexStatus = "unavailable"
        deskIndexPrice = nil
        deskIndexUnderlying = nil
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
        let plan = DeskExtractPlan.resolve(
            slug: resolvedDeskSlug,
            assetClass: declareAssetClass,
            instrumentId: raw
        )
        // Bookless options desk: nothing to ask for, so nothing is asked. The stale
        // `barDeclarationSymbol` these paths would encode never reaches the wire.
        let chainPath = plan.fetchesGlance ? deskChainExtractPath(symbol: symbol) : nil
        let oiPath = plan.fetchesGlance ? deskOiExtractPath(symbol: symbol) : nil
        let greeksPath = plan.fetchesGlance ? deskGreeksExtractPath(symbol: symbol) : nil
        let indexPath = plan.fetchesGlance ? deskIndexExtractPath(symbol: symbol) : nil
        let namedFuturesDepth = plan.usesUsdmHistoryObtain || plan.usesCoinmHistoryObtain
        let depthPath = (plan.fetchesGlance && !plan.defersComSpotDepth) || namedFuturesDepth
            ? deskDepthExtractPath(symbol: symbol)
            : nil
        let historyPath = deskHistoryExtractPath(instrument: raw, consumeVendorArm: true)
        let generation = deskExtractGeneration
        Task { [weak self] in
            guard let self else { return }
            async let chain = self.getExtractJSON(optional: chainPath)
            async let oi = self.getExtractJSON(optional: oiPath)
            async let greeks = self.getExtractJSON(optional: greeksPath)
            async let index = self.getExtractJSON(optional: indexPath)
            async let depth = self.getExtractJSON(optional: depthPath)
            let licensedJSON = await self.getExtractJSON(optional: historyPath)
            let chainJSON = await chain
            let oiJSON = await oi
            let greeksJSON = await greeks
            let indexJSON = await index
            let depthJSON = await depth
            await MainActor.run {
                // The desk rebound while this was in flight — these rows are for an
                // instrument/book that is no longer selected. Leave the holes dark.
                guard generation == self.deskExtractGeneration else { return }
                self.applyStationHistoryEnvelope(licensedJSON ?? [:])
                self.scheduleSessionHistoryRefresh()
                if namedFuturesDepth {
                    self.applyStationDepthEnvelope(depthJSON ?? [:])
                }
                // A skipped glance writes nothing: the hole keeps what invalidate set.
                guard plan.fetchesGlance else { return }
                self.deskChainStatus = chainJSON?["status"] as? String ?? "unavailable"
                self.applyStationChainEnvelope(chainJSON ?? [:])
                self.deskOiStatus = oiJSON?["status"] as? String ?? "unavailable"
                self.applyStationOiEnvelope(oiJSON ?? [:])
                self.applyGreeksEnvelope(greeksJSON ?? [:])
                self.applyStationIndexEnvelope(indexJSON ?? [:])
                if !plan.defersComSpotDepth && !namedFuturesDepth {
                    self.applyStationDepthEnvelope(depthJSON ?? [:])
                }
            }
        }
    }

    /// Cash native obtain always carries `instrument=`. NFO stays the vendor-gated hole.
    func deskHistoryExtractPath(instrument: String, consumeVendorArm: Bool) -> String? {
        let encoded = InstrumentTickBookId.queryEncode(instrument)
        let plan = DeskExtractPlan.resolve(
            slug: resolvedDeskSlug,
            assetClass: declareAssetClass,
            instrumentId: instrument
        )
        if plan.usesKotakHistoryObtain {
            if InstrumentTickBookId.isCashIdentity(instrument) {
                return VendorHistoryObtain.kotakNativeHistoryPath(instrument: instrument)
            }
            let mode = VendorFetchModeStore.mode(for: "licensed_history")
            let armed = consumeVendorArm
                && mode == .onObtain
                && VendorFetchModeStore.consumeArmed("licensed_history")
            return VendorHistoryObtain.kotakHistoryPath(mode: mode, armed: armed)
        }
        if plan.usesOptionsHistoryObtain {
            return deskOptionsHistoryExtractPath()
        }
        if plan.usesUsdmHistoryObtain {
            return deskUsdmHistoryExtractPath()
        }
        if plan.usesCoinmHistoryObtain {
            return deskCoinmHistoryExtractPath()
        }
        if declareAssetClass == .options || declareAssetClass.isNamedComFutures {
            return nil
        }
        return "/api/station/history?instrument=\(encoded)"
    }

    /// Re-read history so the forming bar can land after klines/quotes seed.
    /// Spot, Kotak cash, USDM, and Coin-M. NFO Session stays unnamed.
    func scheduleSessionHistoryRefresh() {
        let instrument = deskSelectedInstrumentId.isEmpty ? barDeclarationSymbol : deskSelectedInstrumentId
        let cash = InstrumentTickBookId.isCashIdentity(instrument)
        let spot = declareAssetClass == .spot && !BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: declareAssetClass,
            instrumentId: instrument
        )
        let namedFutures = declareAssetClass.isNamedComFutures
        guard cash || spot || namedFutures else { return }
        guard let path = deskHistoryExtractPath(instrument: instrument, consumeVendorArm: false) else {
            return
        }
        let generation = deskExtractGeneration
        Task { [weak self] in
            let deadline = Date().addingTimeInterval(15)
            while Date() < deadline {
                guard let self else { return }
                guard generation == self.deskExtractGeneration else { return }
                let json = await self.getExtractJSON(path)
                guard generation == self.deskExtractGeneration else { return }
                guard let json else {
                    try? await Task.sleep(nanoseconds: 400_000_000)
                    continue
                }
                await MainActor.run {
                    guard generation == self.deskExtractGeneration else { return }
                    self.applyStationHistoryEnvelope(json)
                }
                try? await Task.sleep(nanoseconds: 400_000_000)
            }
        }
    }

    /// A skipped row resolves to nil without a request — never a fetch of a sentinel path.
    private func getExtractJSON(optional pathAndQuery: String?) async -> [String: Any]? {
        guard let pathAndQuery else { return nil }
        return await getExtractJSON(pathAndQuery)
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

    private func stringField(_ raw: Any?) -> String? {
        let trimmed = (raw as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return trimmed.isEmpty ? nil : trimmed
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
        guard canSubmitBarDeclaration else { return }
        guard let url = URL(string: baseURL() + "/api/daemon/bar/declare") else { return }
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
                    barPublishedDeclarationId = BarDeclarationIdReducer.apply(
                        event: .declareSucceeded(declId),
                        state: barPublishedDeclarationId,
                    )
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

    /// T4 — bind tray shots to this ticket before debrief save.
    func attachUnpostedToTicket(_ declarationId: String) async -> [String] {
        let ticket = declarationId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !ticket.isEmpty, UUID(uuidString: ticket) != nil else { return [] }
        var ids: [String] = []
        let snapshot = unpostedCaptures
        for rec in snapshot {
            let tradeId = linkableRecentTrades.first(where: { UUID(uuidString: $0.id) != nil })?.id ?? ticket
            if let pending = await sendUnpostedCapture(rec.id, tradeId: tradeId, declarationId: ticket) {
                ids.append(pending)
            }
        }
        return ids
    }

    @discardableResult
    func sendUnpostedCapture(_ id: UUID, tradeId: String, declarationId: String? = nil) async -> String? {
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
            return nil
        }
        guard let rec = unpostedCaptures.first(where: { $0.id == id }) else { return nil }
        let fileURL = unpostedStore.fileURL(for: rec)
        guard let pngData = try? Data(contentsOf: fileURL) else {
            var next = rec
            next.lastError = "Local file missing."
            unpostedStore.update(next)
            reloadUnpostedCaptures()
            return nil
        }
        unpostedSendBusyId = id
        defer { unpostedSendBusyId = nil }

        do {
            let pendingId = try await uploadScreenshotToTrade(
                imageData: pngData,
                contentType: rec.contentType,
                caption: rec.caption,
                tradeId: tradeId,
                existing: rec,
                declarationId: declarationId
            )
            unpostedStore.delete(id: id)
            reloadUnpostedCaptures()
            tradeIdsWithChart.insert(tradeId)
            persistChartTradeIds()
            journalCaptureLastSuccess = "On the trade note"
            journalCaptureLastError = nil
            return pendingId
        } catch {
            var next = rec
            next.lastError = error.localizedDescription
            unpostedStore.update(next)
            reloadUnpostedCaptures()
            return nil
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
        existing: UnpostedCaptureRecord,
        declarationId: String? = nil
    ) async throws -> String {
        guard isAuthenticated && (sessionState == "active" || sessionState == "expiring_soon") else {
            throw NSError(domain: "Notch", code: 401, userInfo: [NSLocalizedDescriptionKey: "Sign in required to finalize captures."])
        }
        let idem = existing.idempotencyKey?.isEmpty == false ? existing.idempotencyKey! : StationWireClient.makeULID()
        var pendingId = existing.consolePendingId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if pendingId.isEmpty {
            pendingId = try await acceptImageOnlyCapture(
                caption: caption,
                tradeId: tradeId,
                idempotencyKey: idem,
                declarationId: declarationId
            )
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
        return pendingId
    }

    private func acceptImageOnlyCapture(
        caption: String,
        tradeId: String,
        idempotencyKey: String,
        declarationId: String? = nil
    ) async throws -> String {
        guard let url = URL(string: baseURL() + "/api/daemon/journal/toolbar-capture/accept") else {
            throw NSError(domain: "Notch", code: 0, userInfo: [NSLocalizedDescriptionKey: "Bad daemon URL"])
        }
        var body: [String: Any] = [
            "draftText": caption,
            "imageOnly": true,
            "tradeId": tradeId,
            "explicitPending": false,
            "idempotencyKey": idempotencyKey,
        ]
        if let declarationId, UUID(uuidString: declarationId) != nil {
            body["preTradeDeclarationId"] = declarationId
        }
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
                if self.shouldPollMarketReads() {
                    await self.fetchPulseData()
                    await self.fetchPositions()
                }
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
                if self.shouldPollMarketReads() {
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
            await fetchMorningBriefIfNeeded()
            if shouldPollMarketReads() {
                await fetchRecentTrades()
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

    /// NSE cash-style window 09:15–15:30 IST (inclusive end minute). COM must not use this as a poll gate.
    func isIstMarketSession() -> Bool {
        BookSessionClock.isNseCashSession(at: Date(), calendar: istCalendar())
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
                killSwitchExpiresAtMs = KillSwitchCountdown.parseMs(payload["expires_at_ms"])
                let wireCountdown: Int?
                if let c = payload["countdown_secs"] as? Int {
                    wireCountdown = c
                } else if let d = payload["countdown_secs"] as? Double {
                    wireCountdown = Int(d)
                } else {
                    wireCountdown = nil
                }
                killSwitchCountdownSecs = KillSwitchCountdown.remainingSecs(
                    expiresAtMs: killSwitchExpiresAtMs,
                    countdownSecs: wireCountdown,
                    nowMs: KillSwitchCountdown.nowMs()
                )
                startKillSwitchCountdownTimerIfNeeded()
            } else {
                killSwitchActive = false
                killSwitchCountdownSecs = nil
                killSwitchExpiresAtMs = nil
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

        case "venue_egress_state":
            if let parsed = VenuePosture(payload: payload) {
                venuePostureBySlug[parsed.venue] = parsed
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
            applyVendorFence(from: json)
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

    func applyVendorFence(from json: [String: Any]) {
        vendorFenceRows = VendorFence.rows(fromHealthJSON: json)
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
                applyPulseHero([:], degraded: true)
                return
            }
            applyPulseHero(hero, degraded: false)
            let trades = j["trades"] as? [[String: Any]] ?? []
            todayClosedTrips = trades.compactMap { row in
                let sym = (row["symbol"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                let net = row["netPnlUsd"] as? Double ?? (row["net_pnl_usd"] as? Double)
                guard !sym.isEmpty, let net, net.isFinite else { return nil }
                return TodayClosedTripCite(symbol: sym, net: net)
            }
        } catch {
            lastAlert = error.localizedDescription
        }
    }

    /// Daemon Today hero is **spot**. Named futures tabs must not paint `pnlTodayUsd`.
    func applyPulseHero(_ hero: [String: Any], degraded: Bool) {
        if degraded {
            if paintsTodayHero {
                sessionPnL = 0
            }
            winRate = 0
            tradesToday = 0
            return
        }
        if paintsTodayHero {
            if let p = hero["pnlTodayInr"] as? Double {
                sessionPnL = p
            } else if let p = hero["pnlTodayUsd"] as? Double {
                sessionPnL = p
            } else {
                sessionPnL = 0
            }
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
            if declareAssetClass.isNamedComFutures {
                positions = []
            } else {
                notePositionTransitionForBar(previousCount: previousPositionCount, newCount: out.count)
                positions = out
            }
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

    var openStartUnlocked: Bool {
        BarOpenStartGate.unlocked(
            calm: declEmotionalCalm,
            confidence: declEmotionalConfidence,
            rule: openNonNegotiable
        )
    }

    var openHidesIndexChips: Bool {
        BarOpenStartGate.hidesIndexChips(
            bookId: selectedMarketBookId,
            assetClass: declareAssetClass,
            slug: resolvedDeskSlug
        )
    }

    /// Brief tab CTA — jump to PLAN only when D gate is set. Calm+confidence already live on Plan.
    func startTradingFromMorningBrief() {
        guard openStartUnlocked else { return }
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
                let json = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
                let levelInt = (json["level"] as? Int)
                    ?? (json["level"] as? Double).map { Int($0) }
                    ?? (bodyDict["level"] as? Int)
                    ?? 3
                let levelStr = levelInt >= 3 ? "L3" : (levelInt == 2 ? "L2" : "L\(levelInt)")
                var payload: [String: Any] = [
                    "active": true,
                    "level": levelStr,
                    "requires_ack": levelInt >= 3,
                ]
                if let exp = KillSwitchCountdown.parseMs(json["expires_at_ms"]) {
                    payload["expires_at_ms"] = exp
                }
                if let c = json["countdown_secs"] as? Int {
                    payload["countdown_secs"] = c
                } else if let d = json["countdown_secs"] as? Double {
                    payload["countdown_secs"] = Int(d)
                }
                _ = applyDaemonEventPayload(
                    type: "kill_switch_state",
                    payload: payload,
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
            killSwitchExpiresAtMs = nil
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

    /// Session P&L delta vs previous fetch not tracked — omit until Today derives it. Never hardcode 0 as a change.
    var sessionPnLChangeHint: Double? { nil }

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
        guard killSwitchCountdownSecs != nil || killSwitchExpiresAtMs != nil else { return }
        stopKillSwitchCountdownTimer()
        let t = Timer(timeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                let remaining = KillSwitchCountdown.remainingSecs(
                    expiresAtMs: self.killSwitchExpiresAtMs,
                    countdownSecs: self.killSwitchCountdownSecs.map { max(0, $0 - 1) },
                    nowMs: KillSwitchCountdown.nowMs()
                )
                self.killSwitchCountdownSecs = remaining
                if remaining == nil || remaining == 0 {
                    self.stopKillSwitchCountdownTimer()
                }
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
    /// Desk-honest money format: single known quote only. Never default INR. DualNoBlend dashes.
    public func formatDeskMoney(_ v: Double) -> String {
        guard let ccy = formatQuoteCurrency else { return "—" }
        return DeskMoneyFormatting.formatSigned(v, quoteCurrency: ccy)
    }

    /// Payload quote for a single desk, else catalog map. Mixed USD+INR → nil.
    var formatQuoteCurrency: String? {
        let slugs = startedDeskSlugs
        if slugs.count > 1, DeskHonesty.heroQuoteCurrency(activeSlugs: slugs) == nil {
            return nil
        }
        if slugs.count == 1 {
            if let payload = deskQuoteCurrency?.trimmingCharacters(in: .whitespacesAndNewlines), !payload.isEmpty {
                return payload.uppercased()
            }
            return DeskHonesty.heroQuoteCurrency(activeSlugs: slugs)
        }
        if let payload = deskQuoteCurrency?.trimmingCharacters(in: .whitespacesAndNewlines), !payload.isEmpty {
            return payload.uppercased()
        }
        return nil
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
            activeExecutionBrokerSlug = slug
        }
        if let ccy = payload["quoteCurrency"] as? String, !ccy.isEmpty {
            deskQuoteCurrency = ccy.uppercased()
        } else if let ccy = payload["quote_currency"] as? String, !ccy.isEmpty {
            deskQuoteCurrency = ccy.uppercased()
        } else if let mapped = DeskMoneyFormatting.quoteCurrency(forBrokerSlug: activeExecutionBrokerSlug) {
            deskQuoteCurrency = mapped
        }
        if let calc = payload["calcProfileId"] as? String, !calc.isEmpty {
            deskCalcProfileId = calc
        } else if let calc = payload["calc_profile_id"] as? String, !calc.isEmpty {
            deskCalcProfileId = calc
        } else if let mapped = DeskMoneyFormatting.calcProfileId(forBrokerSlug: activeExecutionBrokerSlug) {
            deskCalcProfileId = mapped
        }
        if brokerSyncClass == "not_connected" {
            // Start-off only: founder Stop. Do not clear on `disconnected` (poll-circuit) or COM banned.
            activeExecutionBrokerSlug = nil
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
        if brokerSyncClass == "not_connected" {
            accountChrome = .empty
        }
    }

    /// Quote vs funds/fills stay independent — one broker pill is not enough.
    /// Omitting `capabilities` keeps last-known pills; only an explicit status paints red.
    func applyDeskCapabilities(from payload: [String: Any]) {
        let caps = payload["capabilities"] as? [String: Any]
        if let q = caps?["quote"] as? String, !q.isEmpty {
            setIfChanged(\.deskQuoteCapability, q.lowercased())
        }
        if let funds = caps?["funds"] as? String, !funds.isEmpty {
            setIfChanged(\.deskFundsCapability, funds.lowercased())
        }
        if let fills = caps?["fills"] as? String, !fills.isEmpty {
            setIfChanged(\.deskFillsCapability, fills.lowercased())
        }
        if let instruments = caps?["instruments"] as? String, !instruments.isEmpty {
            setIfChanged(\.deskInstrumentsCapability, instruments.lowercased())
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
            await refreshAccountChrome()
        } catch {
            // Agent down: keep last known; UI already has daemon FSM / live-state strips.
        }
    }

    /// Test seam: plant obtain envelopes without hitting the agent.
    func applyAccountObtainEnvelopes(
        funds: [String: Any]?,
        holdings: [String: Any]?,
        positions: [String: Any]?,
        orders: [String: Any]?
    ) {
        guard let book = BarAccountChrome.pulseBookId(
            forAssetClass: declareAssetClass,
            startSlug: activeExecutionBrokerSlug
        ) else {
            accountChrome = .empty
            return
        }
        let ccy = deskQuoteCurrency
            ?? DeskMoneyFormatting.quoteCurrency(forBrokerSlug: activeExecutionBrokerSlug)
            ?? ""
        accountChrome = BarAccountChrome.compose(
            shippingBookId: book,
            quoteCurrency: ccy,
            funds: funds,
            holdings: holdings,
            positions: positions,
            orders: orders
        )
        if declareAssetClass.isNamedComFutures {
            let fundsBook = (funds?["book_id"] as? String)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if fundsBook == book, let pnl = BarAccountChrome.realizedPnl(from: funds) {
                sessionPnL = pnl
            }
        }
        syncUsdmPositionbookPhaseFromChrome()
    }

    func refreshAccountChrome() async {
        guard brokerSessionActive,
              let slug = activeExecutionBrokerSlug,
              let adapter = BarAccountChrome.obtainAdapterId(forStartSlug: slug),
              let book = BarAccountChrome.pulseBookId(
                forAssetClass: declareAssetClass,
                startSlug: slug
              )
        else {
            accountChrome = .empty
            return
        }
        accountChromeGeneration += 1
        let gen = accountChromeGeneration
        async let funds = getExtractJSON(
            BarAccountChrome.obtainPath(adapter: adapter, bookId: book, operation: "funds")
        )
        let holdingsJSON: [String: Any]?
        let positionsJSON: [String: Any]?
        let ordersJSON: [String: Any]?
        if declareAssetClass.isNamedComFutures {
            // Named futures pulse: funds + positionbook. Force-order is lossy and stays off the Funds pill.
            async let positions = getExtractJSON(
                BarAccountChrome.obtainPath(adapter: adapter, bookId: book, operation: "positionbook")
            )
            holdingsJSON = nil
            positionsJSON = await positions
            ordersJSON = nil
        } else {
            async let holdings = getExtractJSON(
                BarAccountChrome.obtainPath(adapter: adapter, bookId: book, operation: "holdings")
            )
            async let positions = getExtractJSON(
                BarAccountChrome.obtainPath(adapter: adapter, bookId: book, operation: "positionbook")
            )
            async let orders = getExtractJSON(
                BarAccountChrome.obtainPath(adapter: adapter, bookId: book, operation: "orderbook")
            )
            holdingsJSON = await holdings
            positionsJSON = await positions
            ordersJSON = await orders
        }
        let fundsJSON = await funds
        guard gen == accountChromeGeneration else { return }
        applyAccountObtainEnvelopes(
            funds: fundsJSON,
            holdings: holdingsJSON,
            positions: positionsJSON,
            orders: ordersJSON
        )
    }

    /// Named futures live/post follow this book's positionbook, never daemon spot inventory.
    private func syncUsdmPositionbookPhaseFromChrome() {
        guard declareAssetClass.isNamedComFutures else { return }
        guard accountChrome.bookId == declareBookId else { return }
        guard accountChrome.positionsStatus == "success" else {
            recomputeBarSurfacePhase()
            return
        }
        let newCount = accountChrome.positionsCount
        notePositionTransitionForBar(previousCount: usdmPositionbookCount, newCount: newCount)
        usdmPositionbookCount = newCount
        recomputeBarSurfacePhase()
    }

    static func brokerDisplayName(forSlug slug: String?) -> String {
        switch slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "kotak_neo", "kotak": return "Kotak Neo"
        case "binance_com": return "Binance.com"
        case nil, "": return "No broker"
        case let other?: return other.replacingOccurrences(of: "_", with: " ").capitalized
        }
    }

    /// Pure pill chrome mapping (testable). Two-arg form ignores venue posture.
    static func brokerPillChrome(
        brokerSyncClass: String,
        slug: String?
    ) -> (dotName: String, label: String) {
        brokerPillChrome(brokerSyncClass: brokerSyncClass, slug: slug, venuePosture: nil)
    }

    /// One pill + subtitle. Banned/backoff on the active slug wins over sync class.
    static func brokerPillChrome(
        brokerSyncClass: String,
        slug: String?,
        venuePosture: VenuePosture?
    ) -> (dotName: String, label: String) {
        let name = brokerDisplayName(forSlug: slug)
        switch venuePosture?.posture.lowercased() {
        case "banned":
            let clock = relativeUntilSuffix(untilMs: venuePosture?.untilMs)
            return ("red", "\(name) · banned\(clock)")
        case "backoff":
            let clock = relativeUntilSuffix(untilMs: venuePosture?.untilMs)
            return ("amber", "\(name) · paused\(clock)")
        default:
            break
        }
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

    /// Compact remaining-time suffix from `until_ms`, or empty when absent/elapsed.
    static func relativeUntilSuffix(untilMs: Int64?) -> String {
        guard let untilMs else { return "" }
        let remainingMs = untilMs - Int64(Date().timeIntervalSince1970 * 1000)
        guard remainingMs > 0 else { return "" }
        let secs = remainingMs / 1000
        if secs < 60 { return " · \(secs)s" }
        let mins = secs / 60
        if mins < 60 { return " · \(mins)m" }
        return " · \(mins / 60)h"
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
            activeExecutionBrokerSlug = nil
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
