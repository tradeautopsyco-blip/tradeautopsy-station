import Foundation
import SwiftUI

/// Envelope for `GET /api/daemon/bar/live-state` (agent-forwarded hosted JSON).
/// Top-level keys match Next.js **`NextResponse.json`** (camelCase); **`notch`** subtree is snake_case per `NotchBarLiveStateV1`.
struct BarLiveStateAPIResponse: Decodable {
    let schemaVersion: Int?
    let barFeaturesActive: Bool?
    let notch: BarLiveStateResponse?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schemaVersion"
        case barFeaturesActive = "barFeaturesActive"
        case notch = "notch"
    }

    init(schemaVersion: Int? = nil, barFeaturesActive: Bool? = true, notch: BarLiveStateResponse?) {
        self.schemaVersion = schemaVersion
        self.barFeaturesActive = barFeaturesActive
        self.notch = notch
    }
}

// MARK: - Notch subtree (NotchBarLiveStateV1)

struct BarUndeclaredPosition: Decodable, Equatable {
    let symbol: String
    let side: String
    let quantity: Int
    let filledAtMs: Int64?
    let broker: String?

    enum CodingKeys: String, CodingKey {
        case symbol, side, quantity, broker
        case filledAtMs = "filledAtMs"
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        symbol = try c.decode(String.self, forKey: .symbol)
        side = try c.decode(String.self, forKey: .side)
        if let q = try c.decodeIfPresent(Int.self, forKey: .quantity) {
            quantity = q
        } else if let qd = try c.decodeIfPresent(Double.self, forKey: .quantity) {
            quantity = Int(qd)
        } else {
            quantity = 0
        }
        if let ms = try c.decodeIfPresent(Int64.self, forKey: .filledAtMs) {
            filledAtMs = ms
        } else if let msd = try c.decodeIfPresent(Double.self, forKey: .filledAtMs) {
            filledAtMs = Int64(msd)
        } else {
            filledAtMs = nil
        }
        broker = try c.decodeIfPresent(String.self, forKey: .broker)
    }

    init(symbol: String, side: String, quantity: Int, filledAtMs: Int64?, broker: String?) {
        self.symbol = symbol
        self.side = side
        self.quantity = quantity
        self.filledAtMs = filledAtMs
        self.broker = broker
    }
}

struct BarLiveStateResponse: Decodable {
    let planState: String?
    let primarySentence: String?
    let triggerType: String?
    let isRedTerminal: Bool
    let composite: CompositeRisk?
    let activeInterventions: [ActiveIntervention]
    let syncState: String
    let lastSyncAt: String?
    let declarationSubmitBlocked: Bool?
    let pendingDeclaration: BarPendingDeclaration?
    let archetype: String?
    let slStatus: String?
    let slFailureReason: String?
    let undeclaredPosition: BarUndeclaredPosition?
    let slPrice: Double?
    let entryTimeISO: String?
    let isSessionLevel: Bool?
    let sessionTradeCount: Int?
    let sessionMaxTrades: Int?
    let sessionLossAmount: Double?
    let sessionLossLimit: Double?
    let sessionWindowEndsISO: String?
    let tiltSignal: Bool?
    let dailyCheckInRequired: Bool?
    let weeklyPnL: Double?
    let daysInTrade: Int?
    let unrealizedPnL: Double?
    /// ₹ risk envelope from declared qty × |entry − stop| when hosted merged payload carries entry (#113 slice 2).
    let declaredMaxLossINR: Double?
    let swingThesisQuote: String?
    let protectiveExistingSl: BarProtectiveExistingSlSnapshot?
    /// Latest **matched** pre-trade declaration id when server has reconciled a fill (protective `place_sl`).
    let matchedDeclarationId: String?
    /// Declared vs actual escrow lane (#125); omitted or empty until server forwards.
    let escrowMatchReport: BarEscrowMatchReport?
    /// Connected broker for protective `place_sl` (e.g. `kotak_neo`).
    let protectiveBrokerSlug: String?
    /// 0–1 validated composite from hosted brain (#180).
    let behavioralScore: Double?
    /// FLOW | CALM | CAUTION | SOFT_BLOCK | DANGER (#180).
    let behavioralVerdict: String?
    let behaviorSignals: [BarBehaviorSignalRow]
    /// Applied score multipliers from hosted brain (#184).
    let behavioralMultipliers: [BarBehavioralMultiplierRow]

    enum CodingKeys: String, CodingKey {
        case planState = "plan_state"
        case primarySentence = "primary_sentence"
        case triggerType = "trigger_type"
        case isRedTerminal = "is_red_terminal"
        case composite
        case activeInterventions = "active_interventions"
        case syncState = "sync_state"
        case lastSyncAt = "last_sync_at"
        case declarationSubmitBlocked = "declaration_submit_blocked"
        case pendingDeclaration = "pending_declaration"
        case archetype
        case slStatus = "sl_status"
        case slFailureReason = "sl_failure_reason"
        case undeclaredPosition = "undeclared_position"
        case slPrice = "sl_price"
        case entryTimeISO = "entry_time_iso"
        case isSessionLevel = "is_session_level"
        case sessionTradeCount = "session_trade_count"
        case sessionMaxTrades = "session_max_trades"
        case sessionLossAmount = "session_loss_amount"
        case sessionLossLimit = "session_loss_limit"
        case sessionWindowEndsISO = "session_window_ends_iso"
        case tiltSignal = "tilt_signal"
        case dailyCheckInRequired = "daily_check_in_required"
        case weeklyPnL = "weekly_pnl"
        case daysInTrade = "days_in_trade"
        case unrealizedPnL = "unrealized_pnl"
        case declaredMaxLossINR = "declared_max_loss_inr"
        case swingThesisQuote = "swing_thesis_quote"
        case protectiveExistingSl = "protective_existing_sl"
        case matchedDeclarationId = "matched_declaration_id"
        case escrowMatchReport = "escrow_match_report"
        case protectiveBrokerSlug = "protective_broker_slug"
        case behavioralScore = "behavioral_score"
        case behavioralVerdict = "behavioral_verdict"
        case behaviorSignals = "behavior_signals"
        case behavioralMultipliers = "behavioral_multipliers"
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        planState = try c.decodeIfPresent(String.self, forKey: .planState)
        primarySentence = try c.decodeIfPresent(String.self, forKey: .primarySentence)
        triggerType = try c.decodeIfPresent(String.self, forKey: .triggerType)
        isRedTerminal = try c.decodeIfPresent(Bool.self, forKey: .isRedTerminal) ?? false
        composite = try c.decodeIfPresent(CompositeRisk.self, forKey: .composite)
        activeInterventions = try c.decodeIfPresent([ActiveIntervention].self, forKey: .activeInterventions) ?? []
        let rawSync = try c.decodeIfPresent(String.self, forKey: .syncState) ?? "GREEN"
        syncState = rawSync.uppercased()
        lastSyncAt = try c.decodeIfPresent(String.self, forKey: .lastSyncAt)
        declarationSubmitBlocked = try c.decodeIfPresent(Bool.self, forKey: .declarationSubmitBlocked)
        pendingDeclaration = try c.decodeIfPresent(BarPendingDeclaration.self, forKey: .pendingDeclaration)
        archetype = try c.decodeIfPresent(String.self, forKey: .archetype)
        slStatus = try c.decodeIfPresent(String.self, forKey: .slStatus)
        slFailureReason = try c.decodeIfPresent(String.self, forKey: .slFailureReason)
        undeclaredPosition = try c.decodeIfPresent(BarUndeclaredPosition.self, forKey: .undeclaredPosition)
        slPrice = try c.decodeIfPresent(Double.self, forKey: .slPrice)
        entryTimeISO = try c.decodeIfPresent(String.self, forKey: .entryTimeISO)
        isSessionLevel = try c.decodeIfPresent(Bool.self, forKey: .isSessionLevel)
        sessionTradeCount = try c.decodeIfPresent(Int.self, forKey: .sessionTradeCount)
        sessionMaxTrades = try c.decodeIfPresent(Int.self, forKey: .sessionMaxTrades)
        sessionLossAmount = try c.decodeIfPresent(Double.self, forKey: .sessionLossAmount)
        sessionLossLimit = try c.decodeIfPresent(Double.self, forKey: .sessionLossLimit)
        sessionWindowEndsISO = try c.decodeIfPresent(String.self, forKey: .sessionWindowEndsISO)
        tiltSignal = try c.decodeIfPresent(Bool.self, forKey: .tiltSignal)
        dailyCheckInRequired = try c.decodeIfPresent(Bool.self, forKey: .dailyCheckInRequired)
        weeklyPnL = try c.decodeIfPresent(Double.self, forKey: .weeklyPnL)
        daysInTrade = try c.decodeIfPresent(Int.self, forKey: .daysInTrade)
        unrealizedPnL = try c.decodeIfPresent(Double.self, forKey: .unrealizedPnL)
        declaredMaxLossINR = try c.decodeIfPresent(Double.self, forKey: .declaredMaxLossINR)
        swingThesisQuote = try c.decodeIfPresent(String.self, forKey: .swingThesisQuote)
        protectiveExistingSl = try c.decodeIfPresent(BarProtectiveExistingSlSnapshot.self, forKey: .protectiveExistingSl)
        matchedDeclarationId = try c.decodeIfPresent(String.self, forKey: .matchedDeclarationId)
        escrowMatchReport = try c.decodeIfPresent(BarEscrowMatchReport.self, forKey: .escrowMatchReport)
        protectiveBrokerSlug = try c.decodeIfPresent(String.self, forKey: .protectiveBrokerSlug)
        behavioralScore = try c.decodeIfPresent(Double.self, forKey: .behavioralScore)
        behavioralVerdict = try c.decodeIfPresent(String.self, forKey: .behavioralVerdict)
        behaviorSignals = try c.decodeIfPresent([BarBehaviorSignalRow].self, forKey: .behaviorSignals) ?? []
        behavioralMultipliers =
            try c.decodeIfPresent([BarBehavioralMultiplierRow].self, forKey: .behavioralMultipliers) ?? []
    }

    init(
        planState: String?,
        primarySentence: String?,
        triggerType: String?,
        isRedTerminal: Bool,
        composite: CompositeRisk?,
        activeInterventions: [ActiveIntervention],
        syncState: String,
        lastSyncAt: String?,
        declarationSubmitBlocked: Bool?,
        pendingDeclaration: BarPendingDeclaration?,
        archetype: String? = nil,
        slStatus: String? = nil,
        slFailureReason: String? = nil,
        undeclaredPosition: BarUndeclaredPosition? = nil,
        slPrice: Double? = nil,
        entryTimeISO: String? = nil,
        isSessionLevel: Bool? = nil,
        sessionTradeCount: Int? = nil,
        sessionMaxTrades: Int? = nil,
        sessionLossAmount: Double? = nil,
        sessionLossLimit: Double? = nil,
        sessionWindowEndsISO: String? = nil,
        tiltSignal: Bool? = nil,
        dailyCheckInRequired: Bool? = nil,
        weeklyPnL: Double? = nil,
        daysInTrade: Int? = nil,
        unrealizedPnL: Double? = nil,
        declaredMaxLossINR: Double? = nil,
        swingThesisQuote: String? = nil,
        protectiveExistingSl: BarProtectiveExistingSlSnapshot? = nil,
        matchedDeclarationId: String? = nil,
        escrowMatchReport: BarEscrowMatchReport? = nil,
        protectiveBrokerSlug: String? = nil,
        behavioralScore: Double? = nil,
        behavioralVerdict: String? = nil,
        behaviorSignals: [BarBehaviorSignalRow] = [],
        behavioralMultipliers: [BarBehavioralMultiplierRow] = [],
    ) {
        self.planState = planState
        self.primarySentence = primarySentence
        self.triggerType = triggerType
        self.isRedTerminal = isRedTerminal
        self.composite = composite
        self.activeInterventions = activeInterventions
        self.syncState = syncState.uppercased()
        self.lastSyncAt = lastSyncAt
        self.declarationSubmitBlocked = declarationSubmitBlocked
        self.pendingDeclaration = pendingDeclaration
        self.archetype = archetype
        self.slStatus = slStatus
        self.slFailureReason = slFailureReason
        self.undeclaredPosition = undeclaredPosition
        self.slPrice = slPrice
        self.entryTimeISO = entryTimeISO
        self.isSessionLevel = isSessionLevel
        self.sessionTradeCount = sessionTradeCount
        self.sessionMaxTrades = sessionMaxTrades
        self.sessionLossAmount = sessionLossAmount
        self.sessionLossLimit = sessionLossLimit
        self.sessionWindowEndsISO = sessionWindowEndsISO
        self.tiltSignal = tiltSignal
        self.dailyCheckInRequired = dailyCheckInRequired
        self.weeklyPnL = weeklyPnL
        self.daysInTrade = daysInTrade
        self.unrealizedPnL = unrealizedPnL
        self.declaredMaxLossINR = declaredMaxLossINR
        self.swingThesisQuote = swingThesisQuote
        self.protectiveExistingSl = protectiveExistingSl
        self.matchedDeclarationId = matchedDeclarationId
        self.escrowMatchReport = escrowMatchReport
        self.protectiveBrokerSlug = protectiveBrokerSlug
        self.behavioralScore = behavioralScore
        self.behavioralVerdict = behavioralVerdict
        self.behaviorSignals = behaviorSignals
        self.behavioralMultipliers = behavioralMultipliers
    }

    /// Mirrors web Bar `intervention.blocksProceed` — disables declaration submit in Notch (`BarDeclarationFlowView`).
    var blocksDeclarationSubmit: Bool { declarationSubmitBlocked == true }
}

/// Orange banner when latest fill was impulsive / undeclared (#137).
struct BarUndeclaredPositionBanner: View {
    let position: BarUndeclaredPosition?

    var body: some View {
        if let pos = position {
            HStack(spacing: 6) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.orange)
                    .font(.system(size: 13))
                VStack(alignment: .leading, spacing: 2) {
                    Text("Undeclared position")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(.orange)
                    Text("\(pos.symbol) \(pos.side) \(pos.quantity)")
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(Color.orange.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: 6))
            .padding(.horizontal, 8)
        }
    }
}

extension BarLiveStateResponse: Equatable {
    static func == (lhs: BarLiveStateResponse, rhs: BarLiveStateResponse) -> Bool {
        lhs.planState == rhs.planState &&
            lhs.syncState == rhs.syncState &&
            lhs.matchedDeclarationId == rhs.matchedDeclarationId &&
            lhs.composite?.state == rhs.composite?.state &&
            lhs.activeInterventions.count == rhs.activeInterventions.count &&
            lhs.declarationSubmitBlocked == rhs.declarationSubmitBlocked &&
            lhs.behavioralVerdict == rhs.behavioralVerdict
    }
}

struct CompositeRisk: Decodable {
    let worstCase: Double
    let budgetPct: Double
    let state: String
    let breakdown: [PositionRiskBreakdown]

    enum CodingKeys: String, CodingKey {
        case worstCase = "worst_case"
        case budgetPct = "budget_pct"
        case state
        case breakdown
    }

    init(worstCase: Double, budgetPct: Double, state: String, breakdown: [PositionRiskBreakdown]) {
        self.worstCase = worstCase
        self.budgetPct = budgetPct
        self.state = state
        self.breakdown = breakdown
    }
}

struct PositionRiskBreakdown: Decodable {
    let symbol: String
    let worstCase: Double

    enum CodingKeys: String, CodingKey {
        case symbol
        case worstCase = "worst_case"
    }
}

struct ActiveIntervention: Decodable, Identifiable {
    let interventionType: String
    let primaryMessage: String
    let expiresAt: String?

    /// Stable identity for **`ForEach`**: prefers uniqueness from type + expiry + body; collisions only if server duplicates an entire row verbatim.
    var id: String { "\(interventionType)|\(expiresAt ?? "")|\(primaryMessage)" }

    enum CodingKeys: String, CodingKey {
        case interventionType = "intervention_type"
        case primaryMessage = "primary_message"
        case expiresAt = "expires_at"
    }

    init(interventionType: String, primaryMessage: String, expiresAt: String?) {
        self.interventionType = interventionType
        self.primaryMessage = primaryMessage
        self.expiresAt = expiresAt
    }
}

// MARK: - Escrow match (#125)

struct BarEscrowNode: Decodable, Equatable, Identifiable {
    let label: String
    let declared: String
    let actual: String
    /// Server literals: `green` | `amber` | `red`
    let match: String
    let breakReason: String?

    var id: String { "\(label)|\(declared)|\(actual)" }

    enum CodingKeys: String, CodingKey {
        case label, declared, actual, match
        case breakReason = "break_reason"
    }

    init(label: String, declared: String, actual: String, match: String, breakReason: String?) {
        self.label = label
        self.declared = declared
        self.actual = actual
        self.match = match
        self.breakReason = breakReason
    }
}

struct BarEscrowMatchReport: Decodable, Equatable {
    /// 0–100 when present.
    let fidelityPct: Double?
    let preTradeSeconds: Int?
    let nodes: [BarEscrowNode]

    enum CodingKeys: String, CodingKey {
        case fidelityPct = "fidelity_pct"
        case preTradeSeconds = "pre_trade_seconds"
        case nodes
    }

    init(fidelityPct: Double?, preTradeSeconds: Int?, nodes: [BarEscrowNode]) {
        self.fidelityPct = fidelityPct
        self.preTradeSeconds = preTradeSeconds
        self.nodes = nodes
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        fidelityPct = try c.decodeIfPresent(Double.self, forKey: .fidelityPct)
        preTradeSeconds = try c.decodeIfPresent(Int.self, forKey: .preTradeSeconds)
        nodes = try c.decodeIfPresent([BarEscrowNode].self, forKey: .nodes) ?? []
    }
}

struct BarPendingDeclaration: Codable {
    let id: String
    let status: String
    let createdAt: String?
    let symbol: String
    let side: String
    let quantity: Double
    let declarationKind: String
    let protectiveSlConsent: Bool
    let stopLoss: Double?
    let target: Double?
    let planSnapshot: BarPlanSnapshotSummary?
    let filledQty: Double?
    let avgFill: Double?
    let fillSymbol: String?
    let fillSide: String?

    enum CodingKeys: String, CodingKey {
        case id, status, symbol, side, quantity, target
        case createdAt = "created_at"
        case declarationKind = "declaration_kind"
        case protectiveSlConsent = "protective_sl_consent"
        case stopLoss = "stop_loss"
        case planSnapshot = "plan_snapshot"
        case filledQty = "filled_qty"
        case avgFill = "avg_fill"
        case fillSymbol = "fill_symbol"
        case fillSide = "fill_side"
    }
}

// MARK: - Trader archetype (Notch routing, #114)

/// Collapsed Bar archetypes for Swift UI routing — aligned with `dbDeclarationKindToArchetype` on the server.
enum TraderArchetype: String, Equatable, Codable, CaseIterable {
    case intraday
    case scalper
    case swing

    /// Active declaration kind wins; else persisted last known; else `intraday`.
    /// `liveArchetype` is included so callers pass server notch `archetype` verbatim — pending declaration always wins when it maps.
    static func resolve(
        pendingDeclarationKind: String?,
        liveArchetype _: String?,
        lastKnown: TraderArchetype?,
    ) -> TraderArchetype {
        if let pending = pendingDeclarationKind, let fromPending = Self.fromDeclarationKind(pending) {
            return fromPending
        }
        if let lastKnown {
            return lastKnown
        }
        return .intraday
    }

    /// Maps DB `declaration_kind` / pending `declaration_kind` string → notch archetype.
    static func fromDeclarationKind(_ raw: String) -> TraderArchetype? {
        let k = raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !k.isEmpty else { return nil }
        switch k {
        case "scalper_session":
            return .scalper
        case "swing":
            return .swing
        case "intraday", "pre_market", "positional":
            return .intraday
        default:
            return nil
        }
    }
}

// MARK: - Declaration UI surface (#115)

/// Single resolver for which declaration root to mount — keyed by `TraderArchetype` plus an optional
/// wire `declaration_kind` (pending declaration or future picker binding). When `declarationKind`
/// maps via `TraderArchetype.fromDeclarationKind`, it wins over `archetype` (same precedence as #114).
enum BarDeclarationSurface: String, Equatable, CaseIterable {
    case intradayForm
    case scalperSessionForm
    case swingForm

    static func resolve(archetype: TraderArchetype, declarationKind: String?) -> BarDeclarationSurface {
        let fromKind = declarationKind.flatMap { TraderArchetype.fromDeclarationKind($0) }
        let effective = fromKind ?? archetype
        switch effective {
        case .intraday:
            return .intradayForm
        case .scalper:
            return .scalperSessionForm
        case .swing:
            return .swingForm
        }
    }
}

// MARK: - Last-known archetype persistence

protocol BarArchetypeStore: AnyObject {
    func loadLastKnownArchetype() -> TraderArchetype?
    func saveLastKnownArchetype(_ archetype: TraderArchetype)
}

final class UserDefaultsBarArchetypeStore: BarArchetypeStore {
    static let defaultStorageKey = "tradeautopsy.notch.bar.lastKnownArchetype"

    private let defaults: UserDefaults
    private let storageKey: String

    convenience init(defaults: UserDefaults = .standard) {
        self.init(defaults: defaults, storageKey: Self.defaultStorageKey)
    }

    init(defaults: UserDefaults, storageKey: String) {
        self.defaults = defaults
        self.storageKey = storageKey
    }

    func loadLastKnownArchetype() -> TraderArchetype? {
        guard let raw = defaults.string(forKey: storageKey), !raw.isEmpty else { return nil }
        return TraderArchetype(rawValue: raw)
    }

    func saveLastKnownArchetype(_ archetype: TraderArchetype) {
        defaults.set(archetype.rawValue, forKey: storageKey)
    }
}

// MARK: - Instrument search (#148)

struct InstrumentResult: Codable, Identifiable, Equatable {
    /// Kotak dual-list uses `segment|token`; Binance falls back to exchange:ticker.
    var id: String {
        tickBookInstrumentId
            ?? nfoListIdentity
            ?? "\(exchange):\(trading_symbol)"
    }
    let trading_symbol: String
    let name: String
    let exchange: String
    let segment: String?
    let instrument_token: Int64?
    let last_price: Double
    var instrument_type: String? = nil

    /// Cash TickBook id. Equity/spot declare — never `nse_fo`.
    var tickBookInstrumentId: String? {
        InstrumentTickBookId.make(segment: segment, instrumentToken: instrument_token)
    }

    /// Book-aware id: NFO last strip may use `nse_fo|token` on Kotak options declare.
    func tickBookInstrumentId(for assetClass: BarDeclareAssetClass, deskSlug: String?) -> String? {
        InstrumentTickBookId.make(
            segment: segment,
            instrumentToken: instrument_token,
            forAsset: assetClass,
            deskSlug: deskSlug
        )
    }

    /// Venue chip: `nse_cm` / `bse_cm` / `nse_fo` on Kotak, else catalog exchange (never invented `"NSE"`).
    var venueLabel: String {
        let seg = (segment ?? "").trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if InstrumentTickBookId.kotakCashSegments.contains(seg)
            || seg == InstrumentTickBookId.kotakNfoSegment
        {
            return seg
        }
        return exchange
    }

    /// List identity for FO rows without treating FO as cash last. Shape only — the
    /// desk-aware `tickBookInstrumentId(for:deskSlug:)` still gates what may bind Last.
    private var nfoListIdentity: String? {
        InstrumentTickBookId.nfoIdentity(segment: segment, instrumentToken: instrument_token)
    }
}

struct InstrumentSearchResponse: Codable {
    let symbols: [InstrumentResult]
    var master_status: String? = nil
}

struct LTPResponse: Codable {
    let ltp: Double?
    let source: String?
    let quote_status: String?
    /// `session_expired` | `broker_error` when quote failed (#149).
    let error: String?
}
