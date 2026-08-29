import Foundation

/// Pre-trade asset class — orthogonal to Intraday / Swing / Scalper style.
enum BarDeclareAssetClass: String, CaseIterable, Identifiable {
    case spot
    case equity
    case options

    var id: String { rawValue }

    var label: String {
        switch self {
        case .spot: return "Spot"
        case .equity: return "Equity"
        case .options: return "Options"
        }
    }
}

/// One Station extract hole on the declare form.
enum BarDeskInstrumentKind: String, Equatable {
    case last
    case history
    case chain
    case openInterest

    var title: String {
        switch self {
        case .last: return "Last"
        case .history: return "History"
        case .chain: return "Chain"
        case .openInterest: return "OI"
        }
    }
}

/// Kotak TickBook identity — `nse_cm|2885` / NFO `nse_fo|token`, never a Binance pair or `"NSE"`.
enum InstrumentTickBookId {
    static let kotakCashSegments: Set<String> = ["nse_cm", "bse_cm"]
    /// NSE F&O segment (SDK `NFO` → `nse_fo`). Not BFO/CDS/MCX unless sourced.
    static let kotakNfoSegment = "nse_fo"

    /// Cash `segment|token` for Kotak REST/TickBook. Nil for Binance (token 0 / SPOT) and FO.
    static func make(segment: String?, instrumentToken: Int64?) -> String? {
        make(segment: segment, instrumentToken: instrumentToken, allowNfo: false)
    }

    /// Options/NFO last strip: `nse_fo|token` only when desk book is `kotak-nse-nfo`.
    /// Cash/equity declare stays cash-only — do not pass `.options` for those classes.
    static func make(
        segment: String?,
        instrumentToken: Int64?,
        forAsset assetClass: BarDeclareAssetClass,
        deskSlug: String?
    ) -> String? {
        let nfo = BarDeskTemplate.isKotakNfoDesk(slug: deskSlug, assetClass: assetClass)
        return make(segment: segment, instrumentToken: instrumentToken, allowNfo: nfo)
    }

    /// True when `raw` is a well-formed `nse_fo|<token>` TickBook id.
    static func isNfoIdentity(_ raw: String) -> Bool {
        segment(of: raw) == kotakNfoSegment
    }

    /// True when `raw` is a well-formed `nse_cm|` / `bse_cm|` cash TickBook id.
    static func isCashIdentity(_ raw: String) -> Bool {
        guard let seg = segment(of: raw) else { return false }
        return kotakCashSegments.contains(seg)
    }

    /// Dated option contract in Binance shape (`BTC-200730-9000-C`): hyphen segments
    /// with a trailing C/P right. Shape only — a new underlying needs no code change.
    /// Never an OpenAlgo `NIFTY28NOV...CE` string parse.
    static func isDatedOptionContract(_ raw: String) -> Bool {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.contains("|") else { return false }
        let parts = trimmed.split(separator: "-", omittingEmptySubsequences: false)
        guard parts.count >= 3, parts.allSatisfy({ !$0.isEmpty }) else { return false }
        let right = parts[parts.count - 1].uppercased()
        return right == "C" || right == "P"
    }

    /// NFO `nse_fo|token` from catalog fields regardless of desk — shape detection only.
    /// Constructing one does not make it bindable; `shouldBindQuoteLast` still gates Last.
    static func nfoIdentity(segment: String?, instrumentToken: Int64?) -> String? {
        make(segment: segment, instrumentToken: instrumentToken, allowNfo: true)
    }

    /// Segment of a well-formed `<segment>|<positive token>` id, else nil.
    private static func segment(of raw: String) -> String? {
        let parts = raw.split(separator: "|", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2, Int64(parts[1]) ?? 0 > 0 else { return nil }
        return parts[0].trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }

    /// Encode `|` so `/api/station/quote?instrument=` survives URL parsing.
    static func queryEncode(_ raw: String) -> String {
        var allowed = CharacterSet.urlQueryAllowed
        allowed.remove(charactersIn: "|&+")
        return raw.addingPercentEncoding(withAllowedCharacters: allowed) ?? raw
    }

    private static func make(segment: String?, instrumentToken: Int64?, allowNfo: Bool) -> String? {
        guard let token = instrumentToken, token > 0 else { return nil }
        let seg = (segment ?? "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        if allowNfo {
            guard seg == kotakNfoSegment else { return nil }
            return "\(seg)|\(token)"
        }
        guard kotakCashSegments.contains(seg) else { return nil }
        return "\(seg)|\(token)"
    }
}

/// Quote vs account health from `GET /api/daemon/broker/sync-state` `capabilities`.
enum DeskCapabilityChrome {
    /// Account proof is funds/fills — never copied from quote.
    static func accountStatus(funds: String, fills: String) -> String {
        let fundsN = funds.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if !fundsN.isEmpty, fundsN != "unavailable" {
            return fundsN
        }
        let fillsN = fills.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if !fillsN.isEmpty {
            return fillsN
        }
        return "unavailable"
    }

    static func dotName(forStatus status: String) -> String {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "fresh":
            return "teal"
        case "stale", "unknown", "loading":
            return "amber"
        default:
            return "red"
        }
    }

    static func pillLabel(kind: String, status: String) -> String {
        "\(kind) · \(status)"
    }

    static func emptySearchHint(masterStatus: String?, connectedInstrumentDesk: Bool) -> String? {
        switch masterStatus?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "loading":
            return "Loading instruments…"
        case "unavailable" where connectedInstrumentDesk:
            return "Catalog failed"
        case "stale" where connectedInstrumentDesk:
            return "Catalog failed"
        default:
            return nil
        }
    }

    static func showsRetryInstruments(status: String) -> Bool {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "unavailable", "stale":
            return true
        default:
            return false
        }
    }
}

enum DeskCatalogAllowlist {
    static func isAllowedExchange(_ exchange: String) -> Bool {
        switch normalized(exchange) {
        case "kotak_neo", "kotak", "binance_com", "binance":
            return true
        default:
            return false
        }
    }

    static func isBinanceDesk(_ slug: String?) -> Bool {
        switch slug.map(normalized) {
        case "binance_com", "binance":
            return true
        default:
            return false
        }
    }

    static func filterSymbols(_ symbols: [InstrumentResult], deskSlug: String?) -> [InstrumentResult] {
        let allowed = symbols.filter { isAllowedExchange($0.exchange) }
        if BarDeskTemplate.isKotakNeoDesk(slug: deskSlug) {
            return allowed.filter { isKotakExchange($0.exchange) }
        }
        if isBinanceDesk(deskSlug) {
            return allowed.filter { isBinanceExchange($0.exchange) }
        }
        return allowed
    }

    static func refusesKotakSelection(_ result: InstrumentResult, deskSlug: String?) -> Bool {
        guard BarDeskTemplate.isKotakNeoDesk(slug: deskSlug) else { return false }
        return !isKotakExchange(result.exchange)
    }

    private static func isKotakExchange(_ exchange: String) -> Bool {
        switch normalized(exchange) {
        case "kotak_neo", "kotak":
            return true
        default:
            return false
        }
    }

    private static func isBinanceExchange(_ exchange: String) -> Bool {
        switch normalized(exchange) {
        case "binance_com", "binance":
            return true
        default:
            return false
        }
    }

    private static func normalized(_ value: String) -> String {
        value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }
}

enum BarDeskTemplate {
    /// Widgets for this class. Chain/OI stay on the options template even while extracts are unavailable.
    static func glanceKinds(for asset: BarDeclareAssetClass) -> [BarDeskInstrumentKind] {
        switch asset {
        case .spot, .equity:
            return [.last, .history]
        case .options:
            return [.last, .history, .chain, .openInterest]
        }
    }

    static func historyDetail(status: String, ineligible: [String]) -> String {
        if ineligible.contains("rights_forbid_canonical") {
            return "yahoo-shaped · rights_forbid_canonical"
        }
        if status == "unsupported" {
            return "unsupported"
        }
        if status == "unavailable" {
            return "no licensed series"
        }
        return status
    }

    /// One licensed History row. Yahoo is never the product line (COM or Kotak).
    static func historyGlanceLine(
        licensedStatus: String,
        licensedIneligible: [String],
        yahooStatus: String,
        yahooIneligible: [String],
        stitchYahoo: Bool
    ) -> String {
        let licensed = historyDetail(status: licensedStatus, ineligible: licensedIneligible)
        guard stitchYahoo else { return licensed }
        let yahoo = historyDetail(status: yahooStatus, ineligible: yahooIneligible)
        return "\(licensed) · \(yahoo)"
    }

    static func isKotakNeoDesk(slug: String?) -> Bool {
        switch slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "kotak_neo", "kotak":
            return true
        default:
            return false
        }
    }

    /// Catalog v1 `bookId` for Kotak cash. Do not add a second catalog row for NFO.
    static let kotakCashBookId = "kotak-nse-bse-cash"
    /// Named NFO book — same `kotak_neo` login slug; not a `BrokerCatalog.v1` row.
    static let kotakNfoBookId = "kotak-nse-nfo"

    /// Desk book without a catalog row: Kotak slug + options class → `kotak-nse-nfo`.
    static func deskBookId(slug: String?, assetClass: BarDeclareAssetClass) -> String? {
        guard isKotakNeoDesk(slug: slug) else { return nil }
        return assetClass == .options ? kotakNfoBookId : kotakCashBookId
    }

    /// NFO last strip is allowed only on the named book (Kotak desk + options declare).
    static func isKotakNfoDesk(slug: String?, assetClass: BarDeclareAssetClass) -> Bool {
        isKotakNeoDesk(slug: slug) && assetClass == .options
    }

    /// Crypto options desk: Binance slug + options declare + a dated contract binding.
    /// A leftover `BTCUSDT` on the Options tab is a pair, not a contract — the instrument
    /// shape is the discriminator, never the desk alone.
    static func isBinanceOptionsDesk(
        slug: String?,
        assetClass: BarDeclareAssetClass,
        instrumentId: String
    ) -> Bool {
        DeskCatalogAllowlist.isBinanceDesk(slug)
            && assetClass == .options
            && InstrumentTickBookId.isDatedOptionContract(instrumentId)
    }
}

/// Entry-price seed from a wire last. An eapi premium of `0.001` through `%.2f` prints
/// `0.00` — a lie — so the crypto options path keeps the wire string verbatim.
/// NFO / cash keep the two-decimal format off the parsed Double.
enum BarDeskLastFormatting {
    static func entryPrice(rawLast: String?, value: Double, preservesPrecision: Bool) -> String {
        if preservesPrecision {
            let trimmed = (rawLast ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty { return trimmed }
        }
        return String(format: "%.2f", value)
    }
}

/// What `refreshDeskExtracts` is allowed to ask the agent for on this desk. Pure, so the
/// decision is testable without an HTTP seam — a request that is never issued is the
/// only honest way to prove a hole stays dark.
struct DeskExtractPlan: Equatable {
    /// Chain and OI are book-scoped. An options declare with no named book has nothing
    /// to ask for — `chain_handler` can only answer dark — so it issues no glance at all
    /// rather than sending a bookless request and painting the reply.
    var fetchesGlance: Bool
    /// The Kotak history obtain is licensed to the Kotak desk. No other desk borrows it.
    var usesKotakHistoryObtain: Bool

    static func resolve(slug: String?, assetClass: BarDeclareAssetClass) -> DeskExtractPlan {
        let book = BarDeskTemplate.deskBookId(slug: slug, assetClass: assetClass)
        return DeskExtractPlan(
            fetchesGlance: !(assetClass == .options && book == nil),
            usesKotakHistoryObtain: BarDeskTemplate.isKotakNeoDesk(slug: slug)
        )
    }
}

/// What an instrument id says it is, independent of the declare tab. The tab is a
/// user preference; this is the instrument's own shape, and the shape wins.
enum DeskInstrumentShape: Equatable {
    case kotakNfo
    case kotakCash
    /// Binance dated contract (`BTC-200730-9000-C`).
    case binanceOption
    /// Binance pair (`BTCUSDT`).
    case binanceSpot
    /// Nothing recognisable — keep the caller's class and let the desk refuse it.
    case unknown

    var impliedAssetClass: BarDeclareAssetClass? {
        switch self {
        case .kotakNfo, .binanceOption: return .options
        case .kotakCash: return .equity
        case .binanceSpot: return .spot
        case .unknown: return nil
        }
    }
}

/// One resolved desk binding: which book to glance, which class to declare under,
/// which id to quote, and which underlying the chain is keyed on. Resolve once at
/// selection, then act — so no branch can rebind against a stale field.
struct DeskInstrumentBind: Equatable {
    /// Catalog book for chain/OI. **Nil on every Binance desk**: Binance never sends
    /// `book=` and its glance is skipped outright, so there is no `binance-com-*`
    /// book id to name here. Mirrors `BarDeskTemplate.deskBookId` exactly.
    var bookId: String?
    /// Class implied by the instrument, or the caller's class when nothing is implied.
    var assetClass: BarDeclareAssetClass
    /// `nse_fo|token`, `nse_cm|2885`, `BTCUSDT`, `BTC-200730-9000-C` — verbatim casing.
    var tickBookId: String
    /// Chain key: NFO wants `pSymbolName` (BANKNIFTY), not the contract ticker.
    /// Empty when only a `segment|token` is known — callers fall back to the typed symbol.
    var chainUnderlying: String
    var shape: DeskInstrumentShape

    /// Kotak-only identity picked up on a desk that cannot serve it (paste, stale row).
    var isKotakIdentity: Bool {
        shape == .kotakNfo || shape == .kotakCash
    }

    static func resolve(
        _ result: InstrumentResult,
        slug: String?,
        currentClass: BarDeclareAssetClass
    ) -> DeskInstrumentBind {
        let ticker = BarBrokerTicker.normalize(raw: result.trading_symbol)
            ?? result.trading_symbol.trimmingCharacters(in: .whitespacesAndNewlines)
        let kotakDesk = BarDeskTemplate.isKotakNeoDesk(slug: slug)
        let shape: DeskInstrumentShape
        let tickBookId: String
        if let nfo = InstrumentTickBookId.nfoIdentity(
            segment: result.segment,
            instrumentToken: result.instrument_token
        ) {
            shape = .kotakNfo
            tickBookId = nfo
        } else if let cash = InstrumentTickBookId.make(
            segment: result.segment,
            instrumentToken: result.instrument_token
        ) {
            shape = .kotakCash
            tickBookId = cash
        } else {
            let exchange = result.exchange.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            shape = binanceShape(
                ticker,
                // A catalog row can name the venue even when the desk slug does not.
                binanceDesk: !kotakDesk
                    && (DeskCatalogAllowlist.isBinanceDesk(slug)
                        || exchange == "binance_com"
                        || exchange == "binance")
            )
            tickBookId = shape == .unknown ? (result.tickBookInstrumentId ?? ticker) : ticker
        }
        // NFO chain is keyed on `pSymbolName`, never the strike-bearing contract ticker.
        let underlying = shape == .kotakNfo
            ? nonEmpty(result.name) ?? ticker
            : ticker
        return make(
            shape: shape,
            tickBookId: tickBookId,
            chainUnderlying: underlying,
            slug: slug,
            currentClass: currentClass
        )
    }

    /// Paste / type path: the id is all we have, so classify the string itself.
    static func resolve(
        rawId: String,
        slug: String?,
        currentClass: BarDeclareAssetClass
    ) -> DeskInstrumentBind {
        let trimmed = rawId.trimmingCharacters(in: .whitespacesAndNewlines)
        let shape: DeskInstrumentShape
        if InstrumentTickBookId.isNfoIdentity(trimmed) {
            shape = .kotakNfo
        } else if InstrumentTickBookId.isCashIdentity(trimmed) {
            shape = .kotakCash
        } else {
            shape = binanceShape(trimmed, binanceDesk: DeskCatalogAllowlist.isBinanceDesk(slug))
        }
        return make(
            shape: shape,
            tickBookId: trimmed,
            // A `segment|token` is not an underlying — leave it empty rather than invent one.
            chainUnderlying: trimmed.contains("|") ? "" : trimmed,
            slug: slug,
            currentClass: currentClass
        )
    }

    /// Dated contract vs pair — only on a Binance desk, so `M-M`-style cash tickers can
    /// never be read as options and an unknown desk still falls through to `.unknown`.
    private static func binanceShape(_ ticker: String, binanceDesk: Bool) -> DeskInstrumentShape {
        guard binanceDesk, !ticker.isEmpty else { return .unknown }
        return InstrumentTickBookId.isDatedOptionContract(ticker) ? .binanceOption : .binanceSpot
    }

    private static func make(
        shape: DeskInstrumentShape,
        tickBookId: String,
        chainUnderlying: String,
        slug: String?,
        currentClass: BarDeclareAssetClass
    ) -> DeskInstrumentBind {
        let assetClass = shape.impliedAssetClass ?? currentClass
        return DeskInstrumentBind(
            // Single source of truth for `book=` — nil for Binance falls out of it.
            bookId: BarDeskTemplate.deskBookId(slug: slug, assetClass: assetClass),
            assetClass: assetClass,
            tickBookId: tickBookId,
            chainUnderlying: chainUnderlying,
            shape: shape
        )
    }

    private static func nonEmpty(_ raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}

/// `GET /api/station/chain` and `/api/station/oi` query. Named `book=` only when
/// `deskBookId` is set. Instrument is the typed underlying ticker — never a cash
/// token, never `s1_desk_symbol`.
enum DeskChainExtractQuery {
    static func path(bookId: String?, underlying: String) -> String {
        glancePath(operation: "chain", bookId: bookId, underlying: underlying)
    }

    static func oiPath(bookId: String?, underlying: String) -> String {
        glancePath(operation: "oi", bookId: bookId, underlying: underlying)
    }

    private static func glancePath(operation: String, bookId: String?, underlying: String) -> String {
        let instrument = InstrumentTickBookId.queryEncode(
            underlying.trimmingCharacters(in: .whitespacesAndNewlines)
        )
        let book = bookId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if book.isEmpty {
            return "/api/station/\(operation)?instrument=\(instrument)"
        }
        let encodedBook = InstrumentTickBookId.queryEncode(book)
        return "/api/station/\(operation)?book=\(encodedBook)&instrument=\(instrument)"
    }

    /// Prefer a declaration / typed ticker over a TickBook `segment|token`. Empty stays empty.
    static func underlyingTicker(preferred: String, declarationSymbol: String) -> String {
        func ticker(_ raw: String) -> String? {
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty, !trimmed.contains("|") else { return nil }
            return trimmed
        }
        return ticker(preferred) ?? ticker(declarationSymbol) ?? ""
    }
}
