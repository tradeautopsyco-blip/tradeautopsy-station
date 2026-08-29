import Foundation
import Testing
@testable import Notch

@MainActor
struct BarOptionsDeclareTests {
    @Test func optionsTabUsesThreeZoneSpotDoesNot() {
        #expect(BarOptionsDeclareSurface.usesThreeZone(for: .options))
        #expect(!BarOptionsDeclareSurface.usesThreeZone(for: .spot))
        #expect(!BarOptionsDeclareSurface.usesThreeZone(for: .equity))
    }

    @Test func darkChainNeverShowsStrikeGrid() {
        let none = BarOptionsChainPresentation.from(underlying: "", chainStatus: "unavailable")
        #expect(none == .nothingDeclared)
        #expect(none.showsStrikeGrid == false)

        let hole = BarOptionsChainPresentation.from(underlying: "BANKNIFTY", chainStatus: "unavailable")
        #expect(hole == .unavailable)
        #expect(hole.showsStrikeGrid == false)

        let empty = BarOptionsChainPresentation.from(underlying: "BANKNIFTY", chainStatus: "empty")
        #expect(empty == .empty)
        #expect(empty.showsStrikeGrid == false)

        let unusable = BarOptionsChainPresentation.from(underlying: "BANKNIFTY", chainStatus: "unusable")
        #expect(unusable == .unavailable)
        #expect(unusable.showsStrikeGrid == false)
    }

    @Test func successWireIsNotHonestyAndShowsNoStrikeGrid() {
        #expect(HonestyStatus.fromWire("success") == nil)
        #expect(HonestyStatus(rawValue: "success") == nil)
        #expect(HonestyStatus.allCases.count == 4)

        let lit = BarOptionsChainPresentation.from(underlying: "BANKNIFTY", chainStatus: "success")
        #expect(lit == .lit)
        #expect(lit.showsStrikeGrid == false)
        #expect(lit != .empty)
        #expect(lit != .unavailable)
    }

    @Test func kotakOptionsChainQueryIncludesNfoBook() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.barDeclarationSymbol = "BANKNIFTY25SEP57500CE"
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        let fromName = vm.deskChainExtractPath(symbol: "BANKNIFTY")
        #expect(fromName.contains("book=kotak-nse-nfo"))
        #expect(fromName.contains("instrument=BANKNIFTY"))
        #expect(!fromName.contains("nse_fo"))
        #expect(!fromName.contains("57500"))

        vm.declareAssetClass = .equity
        let cash = vm.deskChainExtractPath(symbol: "RELIANCE")
        #expect(cash.contains("book=kotak-nse-bse-cash"))
        #expect(cash.contains("instrument=RELIANCE"))

        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        let binance = vm.deskChainExtractPath(symbol: "BTCUSDT")
        #expect(!binance.contains("book="))
        #expect(binance.contains("instrument=BTCUSDT"))
        let binanceOi = vm.deskOiExtractPath(symbol: "BTCUSDT")
        #expect(!binanceOi.contains("book="))
        #expect(binanceOi.contains("/api/station/oi?"))
    }

    @Test func kotakOptionsOiQueryIncludesNfoBookNotTickBookToken() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.barDeclarationSymbol = "BANKNIFTY"
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        let oi = vm.deskOiExtractPath(symbol: "BANKNIFTY")
        #expect(oi.contains("/api/station/oi?"))
        #expect(oi.contains("book=kotak-nse-nfo"))
        #expect(oi.contains("instrument=BANKNIFTY"))
        #expect(!oi.contains("nse_fo"))
    }

    @Test func chainInstrumentPrefersTypedUnderlyingOverCashToken() {
        #expect(
            DeskChainExtractQuery.underlyingTicker(
                preferred: "BANKNIFTY",
                declarationSymbol: "nse_cm|2885"
            ) == "BANKNIFTY"
        )
        #expect(
            DeskChainExtractQuery.underlyingTicker(
                preferred: "nse_fo|12345",
                declarationSymbol: "NIFTY"
            ) == "NIFTY"
        )
        #expect(
            DeskChainExtractQuery.underlyingTicker(
                preferred: "nse_cm|2885",
                declarationSymbol: "nse_fo|12345"
            ).isEmpty
        )
        let path = DeskChainExtractQuery.path(bookId: nil, underlying: "BANKNIFTY")
        #expect(!path.contains("book="))
        #expect(path == "/api/station/chain?instrument=BANKNIFTY")
    }

    @Test func premortemUsesDeclaredLimitWhenSigmaDark() {
        let q = BarOptionsPremortem.question(hasLegs: true, declaredMaxINR: 15000)
        #expect(q.contains("15000"))
        #expect(!q.contains("2σ"))
        let hint = BarOptionsPremortem.hint(hasLegs: true, declaredMaxINR: 15000)
        #expect(hint.contains("declared limit"))
    }

    @Test func stopRungFallsBackToDeclaredMaxWithoutEntry() {
        let rung = BarOptionsLadderModel.stopRung(
            units: 1,
            entry: nil,
            stop: 899,
            sideBuy: false,
            declaredMaxINR: 15000,
        )
        #expect(rung == -15000)
    }

    @Test func sigmaRungsStayUnavailable() {
        let rungs = BarOptionsLadderModel.rungs(
            hasLegs: true,
            stopText: "899",
            stopRung: -15000,
            declaredMaxINR: 15000,
            horizonDays: 1,
            expiryDTE: nil,
        )
        #expect(rungs.count == 5)
        #expect(rungs[0].amountINR == -15000)
        #expect(rungs[1].honesty == .unavailable)
        #expect(rungs[2].honesty == .unavailable)
        #expect(rungs[3].honesty == .unavailable)
        #expect(rungs[4].honesty == .unavailable)
        #expect(rungs[1].amountINR == nil)
        #expect(rungs[2].amountINR == nil)
        #expect(rungs[3].amountINR == nil)
        #expect(rungs[4].amountINR == nil)
    }

    @Test func expiryHorizonDoesNotInventDTE() {
        #expect(BarPlanHorizon.days(for: .expiry, dte: nil) == nil)
        #expect(BarPlanHorizon.days(for: .today, dte: nil) == 1)
        #expect(BarPlanHorizon.days(for: .threeSessions, dte: nil) == 3)
    }

    @Test func optionsConfirmSkipsSetupAndRequiresLeg() {
        var input = BarIntradayDeclarationSubmitInput(
            blocksDeclarationSubmit: false,
            protectiveSlConsent: false,
            calm: 2,
            confidence: 4,
            stopLossText: "899",
            symbolRaw: "BANKNIFTY",
            quantityText: "",
            setupType: "",
            invalidationTypeRaw: "",
            invalidationCondition: "It failed because IV crushed.",
            declarationKindWire: "intraday",
            scalperSessionId: "",
            isOptions: true,
            optionLegCount: 0,
            maxPlannedLossText: "15000",
        )
        #expect(BarIntradayDeclareValidator.submitReadiness(input).ready == false)

        input.optionLegCount = 1
        let ready = BarIntradayDeclareValidator.submitReadiness(input)
        #expect(ready.ready == true)
        #expect(ready.hint == nil)
    }

    @Test func twoLegsRoundTripOnPayload() {
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: "BANKNIFTY",
            sideBuy: false,
            quantity: 1,
            stopLoss: 899,
            declarationKind: "intraday",
            moodStress: 2,
            moodImpulse: 4,
            invalidationNote: "limit hit",
            protectiveSlConsent: true,
            entryPrice: nil,
            targetPrice: 320,
            scalperSessionId: nil,
            optionLegs: [
                .init(underlying: "BANKNIFTY", expiry: "2026-09-29", strike: "57500", right: "PE", sideBuy: false, lots: 1),
                .init(underlying: "BANKNIFTY", expiry: "2026-09-29", strike: "57000", right: "PE", sideBuy: true, lots: 1),
            ],
            horizonDays: 1,
            maxPlannedLossINR: 15000,
        )
        let legs = obj["legs"] as? [[String: Any]]
        #expect(legs?.count == 2)
        #expect(legs?[1]["strike"] as? String == "57000")
        #expect(obj["max_planned_loss_inr"] as? Double == 15000)
    }

    @Test func optionsDeclareDoesNotTreatBinanceSpotLastAsOptionsLast() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.declEntryPrice = ""
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.declareAssetClass = .options
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTCUSDT",
            "data": ["last": "65000"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
        #expect(!vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "BTCUSDT"))
    }

    @Test func optionsDeclareClearsBinanceSpotLastWhenSwitchingClass() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.declEntryPrice = ""
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTCUSDT",
            "data": ["last": "65000"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.declEntryPrice == "65000.00")
        vm.declareAssetClass = .options
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
    }

    @Test func optionsDeclareBindsKotakNfoLastNotCash() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.declEntryPrice = ""
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "nse_fo|12345",
            "data": ["last": "245.50"],
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.declEntryPrice == "245.50")

        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "nse_cm|2885",
            "data": ["last": "2910.50"],
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
    }

    // MARK: - Rebind invalidation (chain/OI must not survive a desk change)

    @Test func assetClassSwitchWipesChainAndOiSynchronously() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.barDeclarationSymbol = "BANKNIFTY"
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.deskChainStatus = "success"
        vm.deskOiStatus = "success"

        vm.declareAssetClass = .spot

        // No await: the wipe lands on the class change, not on the HTTP reply.
        #expect(vm.deskChainStatus == "unavailable")
        #expect(vm.deskOiStatus == "unavailable")
        #expect(vm.deskExtractInvalidationReason == "asset-class")
    }

    @Test func brokerSlugSwitchWipesChainAndOiSynchronously() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.barDeclarationSymbol = "BANKNIFTY"
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.deskChainStatus = "success"
        vm.deskOiStatus = "success"

        vm.activeBrokerSlug = "binance_com"

        #expect(vm.deskChainStatus == "unavailable")
        #expect(vm.deskOiStatus == "unavailable")
        #expect(vm.deskExtractInvalidationReason == "broker-slug")
        // The NFO chain is gone before any Binance path is built.
        #expect(!vm.deskChainExtractPath(symbol: "BTCUSDT").contains("kotak-nse-nfo"))
    }

    @Test func repeatedSlugAssignmentDoesNotInvalidate() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        let generation = vm.deskExtractGeneration
        vm.deskChainStatus = "success"

        // Sync-state polls re-assign the same slug every tick — that is not a rebind.
        vm.activeBrokerSlug = "kotak_neo"

        #expect(vm.deskExtractGeneration == generation)
        #expect(vm.deskChainStatus == "success")
    }

    @Test func staleExtractGenerationDoesNotPaintCurrentDesk() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"

        // Generation an in-flight BANKNIFTY chain fetch would be carrying.
        let inFlight = vm.deskExtractGeneration
        vm.activeBrokerSlug = "binance_com"
        #expect(vm.deskExtractGeneration != inFlight)
        #expect(vm.invalidateDeskMarketExtracts(reason: "select-symbol") != inFlight)
    }

    @Test func optionsClassSwitchWipesBinanceLastAndChainTogether() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTCUSDT",
            "data": ["last": "65000"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        vm.deskChainStatus = "success"
        #expect(vm.deskLastStatus == "fresh")

        vm.declareAssetClass = .options

        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(vm.deskChainStatus == "unavailable")
    }

    @Test func invalidationKeepsLastThatTheNewBindingIsAllowedToShow() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "nse_fo|12345",
            "data": ["last": "245.50"],
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "fresh")

        vm.invalidateDeskMarketExtracts(reason: "select-symbol")

        // NFO last is still bindable here — only the book-scoped rows go dark.
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.deskChainStatus == "unavailable")
        #expect(vm.deskOiStatus == "unavailable")
    }

    // MARK: - Bookless options desk asks for nothing (Binance never glances the NFO book)

    @Test func binanceOptionsIssuesNoGlanceAtAll() {
        let plan = DeskExtractPlan.resolve(slug: "binance_com", assetClass: .options)
        #expect(plan.fetchesGlance == false)
        #expect(plan.usesKotakHistoryObtain == false)
        // Same for the bare alias and for a desk with no slug at all — no book, no ask.
        #expect(!DeskExtractPlan.resolve(slug: "binance", assetClass: .options).fetchesGlance)
        #expect(!DeskExtractPlan.resolve(slug: nil, assetClass: .options).fetchesGlance)
    }

    @Test func kotakOptionsStillGlancesTheNamedNfoBook() {
        let plan = DeskExtractPlan.resolve(slug: "kotak_neo", assetClass: .options)
        #expect(plan.fetchesGlance)
        #expect(plan.usesKotakHistoryObtain)

        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.barDeclarationSymbol = "BANKNIFTY"
        #expect(vm.deskChainExtractPath(symbol: "BANKNIFTY").contains("book=kotak-nse-nfo"))
        #expect(vm.deskOiExtractPath(symbol: "BANKNIFTY").contains("book=kotak-nse-nfo"))
    }

    @Test func spotAndEquityGlanceIsUnchangedByTheOptionsSkip() {
        // Only the bookless *options* case was narrowed — nothing else changed shape.
        #expect(DeskExtractPlan.resolve(slug: "binance_com", assetClass: .spot).fetchesGlance)
        #expect(DeskExtractPlan.resolve(slug: "kotak_neo", assetClass: .equity).fetchesGlance)
    }

    @Test func binanceSlugSwitchStopsTheNfoGlanceEntirely() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.barDeclarationSymbol = "BANKNIFTY"
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.deskChainStatus = "success"
        vm.deskOiStatus = "success"

        vm.activeBrokerSlug = "binance_com"

        #expect(vm.deskChainStatus == "unavailable")
        #expect(vm.deskOiStatus == "unavailable")
        // No request follows the wipe, so the stale BANKNIFTY underlying still sitting in
        // barDeclarationSymbol never reaches the wire — and nothing names the NFO book.
        #expect(!DeskExtractPlan.resolve(slug: "binance_com", assetClass: .options).fetchesGlance)
        #expect(!vm.deskChainExtractPath(symbol: "").contains("kotak-nse-nfo"))
        #expect(!vm.deskOiExtractPath(symbol: "").contains("kotak-nse-nfo"))
    }

    @Test func binanceOptionsHistoryDoesNotBorrowTheKotakObtain() {
        #expect(!DeskExtractPlan.resolve(slug: "binance_com", assetClass: .options).usesKotakHistoryObtain)
        #expect(!DeskExtractPlan.resolve(slug: "binance_com", assetClass: .spot).usesKotakHistoryObtain)
        #expect(DeskExtractPlan.resolve(slug: "kotak_neo", assetClass: .options).usesKotakHistoryObtain)

        // A dark reply on the Binance options desk is "unavailable", never Kotak's
        // "unsupported" — the two desks do not share a history verdict.
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.applyStationHistoryEnvelope([:])
        #expect(vm.deskHistoryStatus == "unavailable")
        #expect(vm.deskYahooHistoryStatus == "unavailable")
    }

    @Test func optionsDeclareDoesNotBindUnderlyingTickerWithoutToken() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        #expect(!vm.shouldBindQuoteLast(adapter: "kotak_neo", instrumentId: "BANKNIFTY"))
        #expect(!InstrumentTickBookId.isNfoIdentity("BANKNIFTY"))
        vm.deskSelectedInstrumentId = "BANKNIFTY"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BANKNIFTY",
            "data": ["last": "57500"],
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
    }
}
