import Foundation
import Testing
@testable import Notch

struct BarDeskInstrumentsTests {
    @Test func spotAndEquityDoNotShowChain() {
        #expect(BarDeskTemplate.glanceKinds(for: .spot) == [.last, .history])
        #expect(BarDeskTemplate.glanceKinds(for: .equity) == [.last, .history])
    }

    @Test func optionsShellIncludesRedChainAndOi() {
        #expect(BarDeskTemplate.glanceKinds(for: .options) == [
            .last, .history, .chain, .openInterest,
        ])
    }

    @Test func yahooHistoryDetailIsRightsForbid() {
        let detail = BarDeskTemplate.historyDetail(
            status: "research_segment",
            ineligible: ["rights_forbid_canonical"]
        )
        #expect(detail.contains("rights_forbid_canonical"))
    }

    @Test func licensedHistoryHoleIsUnavailable() {
        #expect(
            BarDeskTemplate.historyDetail(status: "unavailable", ineligible: [])
                == "no licensed series"
        )
    }

    @Test func kotakHistoryIsUnsupportedNotYahoo() {
        #expect(
            BarDeskTemplate.historyDetail(status: "unsupported", ineligible: [])
                == "unsupported"
        )
        let line = BarDeskTemplate.historyGlanceLine(
            licensedStatus: "unsupported",
            licensedIneligible: [],
            yahooStatus: "research_segment",
            yahooIneligible: ["rights_forbid_canonical"],
            stitchYahoo: false
        )
        #expect(line == "unsupported")
        #expect(!line.contains("yahoo"))
    }

    @Test func binanceHistoryGlanceIsLicensedOnly() {
        let hole = BarDeskTemplate.historyGlanceLine(
            licensedStatus: "unavailable",
            licensedIneligible: [],
            yahooStatus: "research_segment",
            yahooIneligible: ["rights_forbid_canonical"],
            stitchYahoo: false
        )
        #expect(hole == "no licensed series")
        #expect(!hole.contains("yahoo"))
        let row = BarDeskTemplate.historyGlanceLine(
            licensedStatus: "success",
            licensedIneligible: [],
            yahooStatus: "research_segment",
            yahooIneligible: ["rights_forbid_canonical"],
            stitchYahoo: false
        )
        #expect(row == "success")
        #expect(!row.contains("yahoo"))
        #expect(!row.contains("yahoo-shaped"))
    }

    @Test func kotakTickBookIdIsSegmentTokenNotBinancePair() {
        #expect(InstrumentTickBookId.make(segment: "nse_cm", instrumentToken: 2885) == "nse_cm|2885")
        #expect(InstrumentTickBookId.make(segment: "bse_cm", instrumentToken: 1400) == "bse_cm|1400")
        #expect(InstrumentTickBookId.make(segment: "SPOT", instrumentToken: 0) == nil)
        #expect(InstrumentTickBookId.make(segment: "NSE", instrumentToken: 2885) == nil)
        #expect(InstrumentTickBookId.make(segment: "nse_fo", instrumentToken: 12345) == nil)
        #expect(InstrumentTickBookId.queryEncode("nse_cm|2885").contains("%7C"))
        #expect(!InstrumentTickBookId.queryEncode("nse_cm|2885").contains("|"))
        #expect(InstrumentTickBookId.queryEncode("nse_fo|12345").contains("%7C"))
    }

    @Test func nfoTickBookIdIsNonNilOnlyOnOptionsKotakPath() {
        #expect(
            InstrumentTickBookId.make(
                segment: "nse_fo",
                instrumentToken: 12345,
                forAsset: .options,
                deskSlug: "kotak_neo"
            ) == "nse_fo|12345"
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "nse_fo",
                instrumentToken: 12345,
                forAsset: .equity,
                deskSlug: "kotak_neo"
            ) == nil
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "nse_fo",
                instrumentToken: 12345,
                forAsset: .spot,
                deskSlug: "kotak_neo"
            ) == nil
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "nse_fo",
                instrumentToken: 12345,
                forAsset: .options,
                deskSlug: "binance_com"
            ) == nil
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "nse_cm",
                instrumentToken: 2885,
                forAsset: .options,
                deskSlug: "kotak_neo"
            ) == nil
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "bse_fo",
                instrumentToken: 12345,
                forAsset: .options,
                deskSlug: "kotak_neo"
            ) == nil
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "cde_fo",
                instrumentToken: 12345,
                forAsset: .options,
                deskSlug: "kotak_neo"
            ) == nil
        )
        #expect(
            InstrumentTickBookId.make(
                segment: "mcx_fo",
                instrumentToken: 12345,
                forAsset: .options,
                deskSlug: "kotak_neo"
            ) == nil
        )
        #expect(InstrumentTickBookId.isNfoIdentity("nse_fo|12345"))
        #expect(!InstrumentTickBookId.isNfoIdentity("nse_cm|2885"))
        #expect(!InstrumentTickBookId.isNfoIdentity("BTCUSDT"))
        #expect(!InstrumentTickBookId.isNfoIdentity("nse_fo|"))
        #expect(!InstrumentTickBookId.isNfoIdentity("nse_fo|0"))
    }

    @Test func kotakDeskBookIsNfoOnOptionsWithoutCatalogRow() {
        #expect(
            BarDeskTemplate.deskBookId(slug: "kotak_neo", assetClass: .options) == "kotak-nse-nfo"
        )
        #expect(
            BarDeskTemplate.deskBookId(slug: "kotak_neo", assetClass: .equity) == "kotak-nse-bse-cash"
        )
        #expect(
            BarDeskTemplate.deskBookId(slug: "kotak_neo", assetClass: .spot) == "kotak-nse-bse-cash"
        )
        #expect(BarDeskTemplate.deskBookId(slug: "binance_com", assetClass: .options) == nil)
        #expect(BarDeskTemplate.isKotakNfoDesk(slug: "kotak_neo", assetClass: .options))
        #expect(!BarDeskTemplate.isKotakNfoDesk(slug: "kotak_neo", assetClass: .equity))
        #expect(!BarDeskTemplate.isKotakNfoDesk(slug: "binance_com", assetClass: .options))

        let nfoPath = DeskChainExtractQuery.path(
            bookId: BarDeskTemplate.deskBookId(slug: "kotak_neo", assetClass: .options),
            underlying: "BANKNIFTY"
        )
        #expect(nfoPath.contains("book=kotak-nse-nfo"))
        #expect(nfoPath.contains("instrument=BANKNIFTY"))
        let nfoOi = DeskChainExtractQuery.oiPath(
            bookId: BarDeskTemplate.deskBookId(slug: "kotak_neo", assetClass: .options),
            underlying: "BANKNIFTY"
        )
        #expect(nfoOi.contains("/api/station/oi?"))
        #expect(nfoOi.contains("book=kotak-nse-nfo"))
        #expect(nfoOi.contains("instrument=BANKNIFTY"))
        let binancePath = DeskChainExtractQuery.path(
            bookId: BarDeskTemplate.deskBookId(slug: "binance_com", assetClass: .options),
            underlying: "BTCUSDT"
        )
        #expect(!binancePath.contains("book="))
    }

    @Test func kotakSearchResultIdentityIsTickBookNotTickerOnly() {
        let nse = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "kotak_neo",
            segment: "nse_cm",
            instrument_token: 2885,
            last_price: 0
        )
        let bse = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "kotak_neo",
            segment: "bse_cm",
            instrument_token: 1400,
            last_price: 0
        )
        #expect(nse.id == "nse_cm|2885")
        #expect(bse.id == "bse_cm|1400")
        #expect(nse.id != bse.id)
        #expect(nse.venueLabel == "nse_cm")
        #expect(nse.exchange == "kotak_neo")
        #expect(nse.exchange != "NSE")
        #expect(nse.exchange != "binance_com")
        #expect(nse.tickBookInstrumentId(for: .equity, deskSlug: "kotak_neo") == "nse_cm|2885")
        #expect(nse.tickBookInstrumentId(for: .options, deskSlug: "kotak_neo") == nil)
    }

    @Test func nfoSearchResultIdentityIsSegmentTokenNotCash() {
        let nfo = InstrumentResult(
            trading_symbol: "BANKNIFTY25SEP57500CE",
            name: "BANKNIFTY",
            exchange: "kotak_neo",
            segment: "nse_fo",
            instrument_token: 12345,
            last_price: 0
        )
        #expect(nfo.tickBookInstrumentId == nil)
        #expect(nfo.tickBookInstrumentId(for: .options, deskSlug: "kotak_neo") == "nse_fo|12345")
        #expect(nfo.tickBookInstrumentId(for: .equity, deskSlug: "kotak_neo") == nil)
        #expect(nfo.id == "nse_fo|12345")
        #expect(nfo.venueLabel == "nse_fo")
    }

    @Test func binanceSearchResultDoesNotInventKotakToken() {
        let btc = InstrumentResult(
            trading_symbol: "BTCUSDT",
            name: "BTC",
            exchange: "binance_com",
            segment: "SPOT",
            instrument_token: 0,
            last_price: 0
        )
        #expect(btc.tickBookInstrumentId == nil)
        #expect(btc.id == "binance_com:BTCUSDT")
        #expect(btc.venueLabel == "binance_com")
    }

    @Test func quoteCapabilityDotIsIndependentOfAccount() {
        #expect(DeskCapabilityChrome.dotName(forStatus: "fresh") == "teal")
        #expect(DeskCapabilityChrome.dotName(forStatus: "stale") == "amber")
        #expect(DeskCapabilityChrome.dotName(forStatus: "unavailable") == "red")
        #expect(DeskCapabilityChrome.dotName(forStatus: "quotes_http") == "red")
        #expect(DeskCapabilityChrome.dotName(forStatus: "session") == "red")
        #expect(DeskCapabilityChrome.dotName(forStatus: "quotes_unusable") == "red")
        #expect(DeskCapabilityChrome.accountStatus(funds: "unavailable", fills: "fresh") == "fresh")
        #expect(DeskCapabilityChrome.accountStatus(funds: "fresh", fills: "unavailable") == "fresh")
        #expect(DeskCapabilityChrome.pillLabel(kind: "Quote", status: "fresh") == "Quote · fresh")
        #expect(BarDeskTemplate.isKotakNeoDesk(slug: "kotak_neo"))
        #expect(!BarDeskTemplate.isKotakNeoDesk(slug: "binance_com"))
    }

    @Test func loadingInstrumentsUsesAmberDot() {
        #expect(DeskCapabilityChrome.dotName(forStatus: "loading") == "amber")
        #expect(DeskCapabilityChrome.dotName(forStatus: "loading") == DeskCapabilityChrome.dotName(forStatus: "stale"))
        #expect(DeskCapabilityChrome.pillLabel(kind: "Instruments", status: "loading") == "Instruments · loading")
    }

    @Test func emptyInstrumentSearchHint() {
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: "loading", connectedInstrumentDesk: false)
                == "Loading instruments…"
        )
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: "unavailable", connectedInstrumentDesk: true)
                == "Catalog failed"
        )
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: "unavailable", connectedInstrumentDesk: false)
                == nil
        )
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: "fresh", connectedInstrumentDesk: true)
                == nil
        )
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: nil, connectedInstrumentDesk: true)
                == nil
        )
    }

    @Test func catalogAllowlistDropsNseKeepsKotakAndBinance() {
        let nse = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "NSE",
            segment: "NSE",
            instrument_token: 2885,
            last_price: 0
        )
        let kotak = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "kotak_neo",
            segment: "nse_cm",
            instrument_token: 2885,
            last_price: 0
        )
        let binance = InstrumentResult(
            trading_symbol: "BTCUSDT",
            name: "BTC",
            exchange: "binance_com",
            segment: "SPOT",
            instrument_token: 0,
            last_price: 0
        )
        let filtered = DeskCatalogAllowlist.filterSymbols(
            [nse, kotak, binance],
            deskSlug: nil
        )
        #expect(filtered.map(\.exchange) == ["kotak_neo", "binance_com"])
        #expect(!DeskCatalogAllowlist.isAllowedExchange("NSE"))
        #expect(DeskCatalogAllowlist.isAllowedExchange("kotak"))
        #expect(DeskCatalogAllowlist.isAllowedExchange("BINANCE"))
    }

    @Test func kotakDeskFilterDropsBinanceCom() {
        let kotak = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "kotak_neo",
            segment: "nse_cm",
            instrument_token: 2885,
            last_price: 0
        )
        let binance = InstrumentResult(
            trading_symbol: "BTCUSDT",
            name: "BTC",
            exchange: "binance_com",
            segment: "SPOT",
            instrument_token: 0,
            last_price: 0
        )
        let filtered = DeskCatalogAllowlist.filterSymbols(
            [kotak, binance],
            deskSlug: "kotak_neo"
        )
        #expect(filtered.map(\.exchange) == ["kotak_neo"])
    }

    @Test func refusesKotakSelectionOnNseNotCashRow() {
        let nse = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "NSE",
            segment: "NSE",
            instrument_token: 2885,
            last_price: 0
        )
        let cash = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "kotak_neo",
            segment: "nse_cm",
            instrument_token: 2885,
            last_price: 0
        )
        #expect(DeskCatalogAllowlist.refusesKotakSelection(nse, deskSlug: "kotak_neo"))
        #expect(!DeskCatalogAllowlist.refusesKotakSelection(cash, deskSlug: "kotak_neo"))
        #expect(!DeskCatalogAllowlist.refusesKotakSelection(nse, deskSlug: "binance_com"))
    }

    @Test func emptyInstrumentSearchHintStaleConnectedIsCatalogFailed() {
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: "stale", connectedInstrumentDesk: true)
                == "Catalog failed"
        )
        #expect(
            DeskCapabilityChrome.emptySearchHint(masterStatus: "stale", connectedInstrumentDesk: false)
                == nil
        )
    }

    @Test func showsRetryInstrumentsForUnavailableAndStale() {
        #expect(DeskCapabilityChrome.showsRetryInstruments(status: "unavailable"))
        #expect(DeskCapabilityChrome.showsRetryInstruments(status: " STALE "))
        #expect(!DeskCapabilityChrome.showsRetryInstruments(status: "fresh"))
        #expect(!DeskCapabilityChrome.showsRetryInstruments(status: "loading"))
        #expect(!DeskCapabilityChrome.showsRetryInstruments(status: ""))
    }

    @Test func instrumentSearchResponseDecodesOptionalMasterStatus() throws {
        let legacy = try JSONDecoder().decode(
            InstrumentSearchResponse.self,
            from: Data(#"{"symbols":[]}"#.utf8)
        )
        #expect(legacy.symbols.isEmpty)
        #expect(legacy.master_status == nil)

        let loading = try JSONDecoder().decode(
            InstrumentSearchResponse.self,
            from: Data(#"{"symbols":[],"master_status":"loading"}"#.utf8)
        )
        #expect(loading.master_status == "loading")
    }
}
