import Foundation

/// Pre-trade Options is not one surface. Kotak NFO futures (and unknown kind) keep the
/// three-zone HTML surface; Kotak NFO options (OPT*) get the cockpit mosaic + Plan rail;
/// a dated Binance contract gets the crypto surface; everything else — spot, equity, and
/// a bookless pair left sitting on the Options tab — keeps the standard form.
/// The split lives here and in the flow routing, never inside a declare view.
enum BarOptionsDeclareSurface {
    enum Surface: Equatable {
        /// Kotak NFO futures, or NFO with unknown `instrument_type`.
        case nfoThreeZone
        /// Kotak NFO options (OPTIDX / OPTSTK / OPTCUR / OPTCOM).
        case nfoCockpit
        /// Binance desk + `.options` + dated contract.
        case cryptoOptions
        /// Spot / equity / USDM / Coin-M / leftover Options — prototype cockpit (strip + mosaic + Plan rail).
        case standardForm
    }

    /// Same allowlist as the NFO master: option rows, never FUTIDX/FUTSTK/SPREAD.
    static func isNfoOptionInstType(_ raw: String?) -> Bool {
        let trimmed = (raw ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return false }
        return ["OPTIDX", "OPTSTK", "OPTCUR", "OPTCOM"].contains {
            $0.caseInsensitiveCompare(trimmed) == .orderedSame
        }
    }

    static func surface(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String,
        instrumentType: String? = nil
    ) -> Surface {
        if BarDeskTemplate.isBinanceOptionsSelection(
            assetClass: asset,
            instrumentId: instrumentId
        ) {
            return .cryptoOptions
        }
        if BarDeskTemplate.isKotakNfoDesk(slug: slug, assetClass: asset) {
            if isNfoOptionInstType(instrumentType) {
                return .nfoCockpit
            }
            return .nfoThreeZone
        }
        return .standardForm
    }

    static func usesThreeZone(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String,
        instrumentType: String? = nil
    ) -> Bool {
        surface(for: asset, slug: slug, instrumentId: instrumentId, instrumentType: instrumentType) == .nfoThreeZone
    }

    static func usesCockpit(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String,
        instrumentType: String? = nil
    ) -> Bool {
        surface(for: asset, slug: slug, instrumentId: instrumentId, instrumentType: instrumentType) == .nfoCockpit
    }

    static func usesCryptoOptions(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String,
        instrumentType: String? = nil
    ) -> Bool {
        surface(for: asset, slug: slug, instrumentId: instrumentId, instrumentType: instrumentType) == .cryptoOptions
    }
}

/// Prototype board id (`cockpit` / `hero` / `focus`). Custom boards use other raw ids.
enum BarCockpitBoardId: String, Equatable, Hashable, CaseIterable {
    case cockpit
    case hero
    case focus
}

/// Plan dock. Floor is chrome, not a second Confirm path.
enum BarCockpitDock: String, Equatable, Hashable {
    case rail
    case floor
}

/// Prototype cash / last-only cockpit (`CASH_SEEDS` / `LAST_SEEDS`).
/// USDM / Coin-M light Depth after that book's lock names fapi/dapi depth.
enum BarCashCockpitSeed {
    typealias Dock = BarCockpitDock

    enum Kind: String, Equatable, Hashable {
        case session, depth, ticket
    }

    enum StripKind: Equatable, Hashable {
        case funds, last, history, depth, margin
    }

    struct Tile: Equatable, Hashable {
        var kind: Kind
        var x: Int
        var y: Int
        var w: Int
        var h: Int
    }

    /// Cockpit seed dock (prototype `board=cockpit`).
    static let planDock: Dock = .rail

    static func planDock(for board: BarCockpitBoardId) -> Dock {
        switch board {
        case .cockpit, .hero: return .rail
        case .focus: return .floor
        }
    }

    static func tiles(for asset: BarDeclareAssetClass) -> [Tile] {
        tiles(for: asset, board: .cockpit)
    }

    static func tiles(for asset: BarDeclareAssetClass, board: BarCockpitBoardId) -> [Tile] {
        let depth = showsDepth(for: asset)
        switch board {
        case .cockpit:
            if depth {
                // Depth + Ticket-C share one band (2×2 each) so vertical weight matches and Plan
                // keeps five mosaic rows instead of six (taller row cells → more ladder rows).
                return [
                    Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
                    Tile(kind: .depth, x: 0, y: 3, w: 2, h: 2),
                ]
            }
            return [Tile(kind: .session, x: 0, y: 0, w: 4, h: 3)]
        case .hero:
            if depth {
                return [
                    Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
                    Tile(kind: .depth, x: 0, y: 3, w: 2, h: 1),
                ]
            }
            return [Tile(kind: .session, x: 0, y: 0, w: 4, h: 4)]
        case .focus:
            if depth {
                return [
                    Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
                    Tile(kind: .depth, x: 0, y: 3, w: 4, h: 1),
                ]
            }
            return [Tile(kind: .session, x: 0, y: 0, w: 4, h: 4)]
        }
    }

    static func showsDepth(for asset: BarDeclareAssetClass) -> Bool {
        BarDeskTemplate.glanceKinds(for: asset).contains(.depth)
    }

    /// Catalog kinds this book may add in Edit board. Ticket is overlay, not catalog.
    static func catalog(for asset: BarDeclareAssetClass) -> [Kind] {
        if showsDepth(for: asset) {
            return [.session, .depth]
        }
        return [.session]
    }

    static func addSize(for kind: Kind, asset: BarDeclareAssetClass) -> (w: Int, h: Int) {
        switch kind {
        case .session:
            return showsDepth(for: asset) ? (4, 2) : (4, 2)
        case .depth:
            return (2, 1)
        case .ticket:
            return ticketOverlaySize(for: asset)
        }
    }

    /// USDM / Coin-M ticket is taller (TIF + Contracts + reduce-only). Spot stays 2×1.
    static func ticketOverlaySize(for asset: BarDeclareAssetClass) -> (w: Int, h: Int) {
        switch asset {
        case .usdm, .coinm:
            return (BarCockpitTicketOverlay.ticketW, 2)
        default:
            // Spot / equity need two mosaic rows so qty + TIF are not clipped (Hero places beside Depth).
            return (BarCockpitTicketOverlay.ticketW, 2)
        }
    }

    static func title(for kind: Kind) -> String {
        switch kind {
        case .session: return "Session"
        case .depth: return "Depth"
        case .ticket: return "Ticket"
        }
    }

    static func note(for kind: Kind) -> String {
        switch kind {
        case .session: return "this pair · named book"
        case .depth: return "market/order_book · not placed"
        case .ticket: return "type · size · TIF"
        }
    }

    static func stripKinds(for asset: BarDeclareAssetClass) -> [StripKind] {
        switch asset {
        case .usdm, .coinm, .spot, .equity, .options:
            // Depth summary removed from top strip — detailed Depth tile stays on the mosaic.
            return [.funds, .last, .history]
        }
    }
}

/// Prototype options/NFO seeds (`OPTIONS_SEEDS`). Cockpit is the v1 default.
enum BarNfoCockpitSeed {
    typealias Dock = BarCockpitDock

    enum Kind: String, Equatable, Hashable {
        case session, oi, payoff, depth, chain
        case greeks, legs, ladder, ticket
    }

    struct Tile: Equatable, Hashable {
        var kind: Kind
        var x: Int
        var y: Int
        var w: Int
        var h: Int
    }

    /// Cockpit seed dock (prototype `board=cockpit`).
    static let planDock: Dock = .rail

    static func planDock(for board: BarCockpitBoardId) -> Dock {
        switch board {
        case .cockpit, .hero: return .rail
        case .focus: return .floor
        }
    }

    /// Cockpit seed tiles — public layout spec used by ticket overlay tests.
    static let tiles: [Tile] = [
        Tile(kind: .session, x: 0, y: 0, w: 2, h: 2),
        Tile(kind: .oi, x: 2, y: 0, w: 2, h: 2),
        Tile(kind: .payoff, x: 0, y: 2, w: 2, h: 2),
        Tile(kind: .depth, x: 2, y: 2, w: 2, h: 2),
        Tile(kind: .chain, x: 0, y: 4, w: 4, h: 1),
    ]

    static func tiles(for board: BarCockpitBoardId) -> [Tile] {
        switch board {
        case .cockpit:
            return tiles
        case .hero:
            return [
                Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
                Tile(kind: .oi, x: 0, y: 3, w: 2, h: 1),
                Tile(kind: .payoff, x: 2, y: 3, w: 2, h: 1),
                Tile(kind: .chain, x: 0, y: 4, w: 4, h: 1),
            ]
        case .focus:
            return [
                Tile(kind: .session, x: 0, y: 0, w: 4, h: 3),
                Tile(kind: .oi, x: 0, y: 3, w: 1, h: 1),
                Tile(kind: .payoff, x: 1, y: 3, w: 1, h: 1),
                Tile(kind: .depth, x: 2, y: 3, w: 1, h: 1),
                Tile(kind: .chain, x: 3, y: 3, w: 1, h: 1),
            ]
        }
    }

    static let catalog: [Kind] = [
        .session, .oi, .payoff, .depth, .chain, .greeks, .legs, .ladder,
    ]

    static func addSize(for kind: Kind) -> (w: Int, h: Int) {
        switch kind {
        case .session, .oi, .payoff, .depth, .greeks, .legs, .ladder: return (2, 1)
        case .chain: return (4, 1)
        case .ticket: return (2, 1)
        }
    }

    static func title(for kind: Kind) -> String {
        switch kind {
        case .session: return "Session"
        case .oi: return "Open interest"
        case .payoff: return "At expiry"
        case .depth: return "Depth"
        case .chain: return "Chain · catalog"
        case .greeks: return "Greeks"
        case .legs: return "Legs"
        case .ladder: return "Ladder"
        case .ticket: return "Ticket"
        }
    }

    static func note(for kind: Kind) -> String {
        switch kind {
        case .session: return BarNfoHistoryCopy.sessionHoleTitle
        case .oi: return "market/open_interest · quote field open_int"
        case .payoff: return BarNfoPayoffCopy.holeTitle
        case .depth: return "market/order_book"
        case .chain: return "showsStrikeGrid = false"
        case .greeks: return "not Black-76"
        case .legs: return "local plan"
        case .ladder: return "rung 1 typed · σ 2–5 dark"
        case .ticket: return "NFO · no COM ticket"
        }
    }
}

/// Chain table is forbidden while the extract is dark. Ghost strike grids are cheating.
/// Wire `"success"` is a glance status, not `HonestyStatus`.
enum BarOptionsChainPresentation: Equatable {
    /// No underlying typed yet.
    case nothingDeclared
    /// Capability hole or refused snapshot — no rows.
    case unavailable
    /// Result set legitimately zero.
    case empty
    /// Glance `status: success` — master rows may exist; still no invented strike grid.
    case lit

    static func from(underlying: String, chainStatus: String) -> BarOptionsChainPresentation {
        let und = underlying.trimmingCharacters(in: .whitespacesAndNewlines)
        if und.isEmpty { return .nothingDeclared }
        let wire = chainStatus.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch wire {
        case "success": return .lit
        case "empty": return .empty
        default: return .unavailable
        }
    }

    /// Strike scale / expiry conversion stay unspecified — never paint 57500.
    var showsStrikeGrid: Bool { false }
}

enum BarOptionsPremortem {
    static func question(hasLegs: Bool, declaredMaxINR: Double?) -> String {
        if hasLegs, let max = declaredMaxINR {
            return "This trade hit your declared limit of ₹\(Self.inr(max)). What happened?"
        }
        return "Write what would prove this trade wrong."
    }

    static func hint(hasLegs: Bool, declaredMaxINR: Double?) -> String {
        if hasLegs, declaredMaxINR != nil {
            return "σ rungs are dark, so this is seeded from your declared limit instead of the chain."
        }
        return "Fill the contract and size, and this question gets sharper."
    }

    private static func inr(_ n: Double) -> String {
        String(format: "%.0f", n)
    }
}

struct BarOptionsLadderRung: Equatable {
    var label: String
    var sub: String
    var amountINR: Double?
    var honesty: HonestyStatus?
}

enum BarOptionsLadderModel {
    /// Rung 1 from typed stop/entry/lots, else −declared max (HTML fallback when chain is a hole).
    static func stopRung(
        units: Double?,
        entry: Double?,
        stop: Double?,
        sideBuy: Bool,
        declaredMaxINR: Double?,
    ) -> Double? {
        if let rung = BarPlanLadder.rung1(units: units, entry: entry, stop: stop, sideBuy: sideBuy) {
            return rung
        }
        if let declaredMaxINR {
            return -abs(declaredMaxINR)
        }
        return nil
    }

    static func rungs(
        hasLegs _: Bool,
        stopText: String,
        stopRung: Double?,
        declaredMaxINR: Double?,
        horizonDays: Int,
        expiryDTE: Int?,
    ) -> [BarOptionsLadderRung] {
        let hLabel = horizonCaption(days: horizonDays, dte: expiryDTE)
        let stopSub: String
        let stopTrim = stopText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !stopTrim.isEmpty {
            stopSub = "option at \(stopTrim) · typed, no market data needed"
        } else {
            stopSub = "set a stop price to light this rung"
        }
        let stopHonesty: HonestyStatus? = stopRung == nil ? .empty : nil
        return [
            BarOptionsLadderRung(
                label: "At your stop",
                sub: stopSub,
                amountINR: stopRung,
                honesty: stopHonesty,
            ),
            BarOptionsLadderRung(
                label: "1σ adverse \(hLabel)",
                sub: "σ scaled from chain IV",
                amountINR: nil,
                honesty: .unavailable,
            ),
            BarOptionsLadderRung(
                label: "2σ / gap \(hLabel)",
                sub: "σ scaled from chain IV",
                amountINR: nil,
                honesty: .unavailable,
            ),
            BarOptionsLadderRung(
                label: "Terminal at expiry",
                sub: "payoff needs premiums from the chain",
                amountINR: nil,
                honesty: .unavailable,
            ),
            BarOptionsLadderRung(
                label: "Distance to breach your limit",
                sub: declaredMaxINR == nil ? "set a max planned loss" : "needs Greeks to solve for the move",
                amountINR: nil,
                honesty: declaredMaxINR == nil ? .empty : .unavailable,
            ),
        ]
    }

    static func horizonCaption(days: Int, dte: Int?) -> String {
        if days == 1 { return "by close today" }
        if let dte, days == dte { return "by expiry" }
        return "over \(days) sessions"
    }
}
