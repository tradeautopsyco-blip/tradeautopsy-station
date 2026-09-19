import Foundation

/// Venue ticket type. Names follow Binance `NewOrderParams` / algo twins, not Notch plan SL/TP.
enum BarVenueOrderType: String, Equatable, CaseIterable, Identifiable {
    case market = "MARKET"
    case limit = "LIMIT"
    case limitMaker = "LIMIT_MAKER"
    case conditional = "CONDITIONAL"

    var id: String { rawValue }

    var stackLabel: String {
        switch self {
        case .market: return "Market"
        case .limit: return "Limit"
        case .limitMaker: return "Limit maker"
        case .conditional: return "Conditional"
        }
    }
}

enum BarTicketSizeMode: String, Equatable {
    case base
    case quote
    case contracts
}

enum BarTicketTif: String, Equatable, CaseIterable, Identifiable {
    case gtc = "GTC"
    case ioc = "IOC"
    case fok = "FOK"

    var id: String { rawValue }
}

/// Live ticket for one named book. DualNoBlend: never copy type/sizeMode across `book_id`.
struct BarDeskTicketIntent: Equatable {
    var bookId: String
    var type: BarVenueOrderType
    var sizeMode: BarTicketSizeMode
    var tif: BarTicketTif
    var reduceOnly: Bool
    var postOnly: Bool
    var quoteOrderQty: Double?
    var triggerPrice: String

    static let blank = BarDeskTicketIntent(
        bookId: "",
        type: .limit,
        sizeMode: .base,
        tif: .gtc,
        reduceOnly: false,
        postOnly: false,
        quoteOrderQty: nil,
        triggerPrice: "",
    )

    static func defaults(bookId: String) -> BarDeskTicketIntent {
        let spec = BarDeskTicketSpec.forBook(bookId)
        return BarDeskTicketIntent(
            bookId: bookId,
            type: spec.defaultType,
            sizeMode: spec.defaultSizeMode,
            tif: .gtc,
            reduceOnly: false,
            postOnly: false,
            quoteOrderQty: nil,
            triggerPrice: "",
        )
    }

    func coerced() -> BarDeskTicketIntent {
        let spec = BarDeskTicketSpec.forBook(bookId)
        var next = self
        if !spec.types.contains(type) {
            next.type = spec.defaultType
        }
        if !spec.sizeModes.contains(sizeMode) {
            next.sizeMode = spec.defaultSizeMode
        }
        if spec.bookId != BarDeskTemplate.binanceComSpotBookId {
            next.quoteOrderQty = nil
        }
        if next.sizeMode != .quote {
            next.quoteOrderQty = nil
        }
        if type != .conditional {
            next.triggerPrice = ""
        }
        if !spec.reduceOnly {
            next.reduceOnly = false
        }
        if !spec.postOnly {
            next.postOnly = false
        }
        return next
    }

    var path: String {
        let spec = BarDeskTicketSpec.forBook(bookId)
        if type == .conditional, let algo = spec.algoPath {
            return algo
        }
        return spec.orderPath
    }

    var showsTif: Bool {
        BarDeskTicketSpec.forBook(bookId).tifOn.contains(type)
    }

    var jsonObject: [String: Any] {
        var o: [String: Any] = [
            "type": type.rawValue,
            "tif": tif.rawValue,
            "reduce_only": reduceOnly,
            "size_mode": sizeMode.rawValue,
            "path": path,
        ]
        if postOnly { o["post_only"] = true }
        if let q = quoteOrderQty { o["quote_order_qty"] = q }
        let trig = triggerPrice.trimmingCharacters(in: .whitespacesAndNewlines)
        if type == .conditional, !trig.isEmpty { o["trigger_price"] = trig }
        return o
    }
}

struct BarDeskTicketSpec: Equatable {
    var bookId: String
    var types: [BarVenueOrderType]
    var defaultType: BarVenueOrderType
    var sizeModes: [BarTicketSizeMode]
    var defaultSizeMode: BarTicketSizeMode
    var tifOn: [BarVenueOrderType]
    var reduceOnly: Bool
    var postOnly: Bool
    var orderPath: String
    var algoPath: String?
    var conditionalLabel: String

    static func forBook(_ bookId: String) -> BarDeskTicketSpec {
        switch bookId {
        case BarDeskTemplate.binanceComSpotBookId:
            return BarDeskTicketSpec(
                bookId: bookId,
                types: [.market, .limit, .limitMaker],
                defaultType: .limit,
                sizeModes: [.base, .quote],
                defaultSizeMode: .base,
                tifOn: [.limit],
                reduceOnly: false,
                postOnly: false,
                orderPath: "/api/v3/order",
                algoPath: nil,
                conditionalLabel: "Conditional",
            )
        case BarDeskTemplate.binanceComUsdmBookId:
            return BarDeskTicketSpec(
                bookId: bookId,
                types: [.limit, .market, .conditional],
                defaultType: .limit,
                sizeModes: [.contracts],
                defaultSizeMode: .contracts,
                tifOn: [.limit, .conditional],
                reduceOnly: true,
                postOnly: false,
                orderPath: "/fapi/v1/order",
                algoPath: "/fapi/v1/algoOrder",
                conditionalLabel: "Conditional · algoOrder",
            )
        case BarDeskTemplate.binanceComCoinmBookId:
            return BarDeskTicketSpec(
                bookId: bookId,
                types: [.limit, .market, .conditional],
                defaultType: .limit,
                sizeModes: [.contracts],
                defaultSizeMode: .contracts,
                tifOn: [.limit, .conditional],
                reduceOnly: true,
                postOnly: false,
                orderPath: "/dapi/v1/order",
                algoPath: "/dapi/v1/algoOrder",
                conditionalLabel: "Conditional · dapi algoOrder",
            )
        case BarDeskTemplate.binanceComOptionsBookId:
            return BarDeskTicketSpec(
                bookId: bookId,
                types: [.limit],
                defaultType: .limit,
                sizeModes: [.contracts],
                defaultSizeMode: .contracts,
                tifOn: [.limit],
                reduceOnly: false,
                postOnly: true,
                orderPath: "/eapi/v1/order",
                algoPath: nil,
                conditionalLabel: "Conditional",
            )
        default:
            return BarDeskTicketSpec(
                bookId: bookId,
                types: [.limit],
                defaultType: .limit,
                sizeModes: [.base],
                defaultSizeMode: .base,
                tifOn: [.limit],
                reduceOnly: false,
                postOnly: false,
                orderPath: "",
                algoPath: nil,
                conditionalLabel: "Conditional",
            )
        }
    }

    func label(for type: BarVenueOrderType) -> String {
        type == .conditional ? conditionalLabel : type.stackLabel
    }
}

enum BarDeskTicketIllegal {
    /// Options leftover pair (CATIUSDT letters on the Options book) refuses bind.
    static func reason(bookId: String, instrumentId: String, type: BarVenueOrderType) -> String? {
        if bookId == BarDeskTemplate.binanceComOptionsBookId {
            if type != .limit { return "Market on Options" }
            if !InstrumentTickBookId.isDatedOptionContract(instrumentId) {
                return "leftover CATIUSDT refuses bind"
            }
        }
        return nil
    }
}
