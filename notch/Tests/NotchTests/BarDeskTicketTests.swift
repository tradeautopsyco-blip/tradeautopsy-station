import Foundation
import Testing
@testable import Notch

struct BarDeskTicketTests {
    @Test func surfaceRoutesComBooksAndSkipsEquity() {
        #expect(BarDeskTicketSurface.surface(
            for: .usdm, slug: "binance_com", instrumentId: "CATIUSDT"
        ) == .usdm)
        #expect(BarDeskTicketSurface.surface(
            for: .coinm, slug: "binance_com", instrumentId: "BTCUSD_PERP"
        ) == .coinm)
        #expect(BarDeskTicketSurface.surface(
            for: .spot, slug: "binance_com", instrumentId: "BTCUSDT"
        ) == .spot)
        #expect(BarDeskTicketSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTC-260925-90000-C"
        ) == .cryptoOptions)
        #expect(BarDeskTicketSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "CATIUSDT"
        ) == .none)
        #expect(BarDeskTicketSurface.surface(
            for: .equity, slug: "kotak_neo", instrumentId: "RELIANCE"
        ) == .none)
        #expect(BarDeskTicketSurface.bookId(
            for: .usdm, slug: "binance_com", instrumentId: "CATIUSDT"
        ) == "binance-com-usdm")
        #expect(BarDeskTicketSurface.bookId(
            for: .coinm, slug: "binance_com", instrumentId: "BTCUSD_PERP"
        ) == "binance-com-coinm")
        #expect(BarDeskTicketSurface.bookId(
            for: .usdm, slug: "binance_com", instrumentId: "CATIUSDT"
        ) != BarDeskTicketSurface.bookId(
            for: .spot, slug: "binance_com", instrumentId: "CATIUSDT"
        ))
    }

    @Test func usdmConditionalIsAlgoOrderCoinMDoesNotShareSpotPath() {
        let usdm = BarDeskTicketIntent.defaults(bookId: "binance-com-usdm")
        var cond = usdm
        cond.type = .conditional
        cond = cond.coerced()
        #expect(cond.path == "/fapi/v1/algoOrder")
        #expect(cond.path != "/fapi/v1/order")

        var coinm = BarDeskTicketIntent.defaults(bookId: "binance-com-coinm")
        coinm.type = .conditional
        coinm = coinm.coerced()
        #expect(coinm.path == "/dapi/v1/algoOrder")
        #expect(coinm.path != cond.path)
        #expect(coinm.bookId != usdm.bookId)

        let spot = BarDeskTicketIntent.defaults(bookId: "binance-com-spot")
        #expect(spot.path == "/api/v3/order")
        #expect(spot.sizeMode == .base)
        #expect(spot.coerced().quoteOrderQty == nil)
    }

    @Test func optionsLeftoverCatiusdtRefusesBind() {
        #expect(
            BarDeskTicketIllegal.reason(
                bookId: "binance-com-options",
                instrumentId: "CATIUSDT",
                type: .limit
            ) == "leftover CATIUSDT refuses bind"
        )
        #expect(
            BarDeskTicketIllegal.reason(
                bookId: "binance-com-options",
                instrumentId: "BTC-260925-90000-C",
                type: .limit
            ) == nil
        )
        #expect(
            BarDeskTicketIllegal.reason(
                bookId: "binance-com-options",
                instrumentId: "BTC-260925-90000-C",
                type: .market
            ) == "Market on Options"
        )
    }

    @Test func spotQuoteXorDropsQuoteWhenBase() {
        var t = BarDeskTicketIntent.defaults(bookId: "binance-com-spot")
        t.sizeMode = .quote
        t.quoteOrderQty = 50
        t = t.coerced()
        #expect(t.quoteOrderQty == 50)
        t.sizeMode = .base
        t = t.coerced()
        #expect(t.quoteOrderQty == nil)
        #expect(t.sizeMode == .base)
    }
}

@MainActor
struct BarDeskTicketViewModelTests {
    @Test func classSwitchWipesLeftoverTypeByBookId() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.declareAssetClass = .usdm
        vm.deskSelectedInstrumentId = "CATIUSDT"
        vm.adoptDeskTicket()
        #expect(vm.deskTicket.bookId == "binance-com-usdm")
        vm.deskTicket.type = .conditional
        vm.persistDeskTicket()
        #expect(vm.deskTicket.path == "/fapi/v1/algoOrder")

        vm.declareAssetClass = .spot
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.adoptDeskTicket()
        #expect(vm.deskTicket.bookId == "binance-com-spot")
        #expect(vm.deskTicket.type == .limit)
        #expect(vm.deskTicket.type != .conditional)
        #expect(vm.deskTicket.sizeMode == .base)

        vm.declareAssetClass = .usdm
        vm.deskSelectedInstrumentId = "CATIUSDT"
        vm.adoptDeskTicket()
        #expect(vm.deskTicket.bookId == "binance-com-usdm")
        #expect(vm.deskTicket.type == .conditional)
        #expect(vm.deskTicket.path == "/fapi/v1/algoOrder")
    }

    @Test func flatUsdmMarginIsNoneNotInventedFiftyX() {
        let empty = BarAccountChrome.Snapshot.empty
        let readout = BarAccountChrome.futuresMargin(from: empty, symbol: "CATIUSDT")
        #expect(readout.mode == nil)
        #expect(readout.leverage == nil)
        #expect(readout.liq == nil)
        #expect(readout.leverage != "50")
    }
}
