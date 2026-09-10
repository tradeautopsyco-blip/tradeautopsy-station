import Foundation
import Testing
@testable import Notch

@MainActor
struct BarOptionsDeclareTests {
    @Test func optionsSurfaceSplitsByDeskNotByAssetClassAlone() {
        // Kotak NFO keeps the three-zone surface.
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "kotak_neo", instrumentId: "nse_fo|12345"
        ) == .nfoThreeZone)
        #expect(BarOptionsDeclareSurface.usesThreeZone(
            for: .options, slug: "kotak_neo", instrumentId: "nse_fo|12345"
        ))

        // Binance + a dated contract is the crypto surface — never three-zone.
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTC-200730-9000-C"
        ) == .cryptoOptions)
        #expect(BarOptionsDeclareSurface.usesCryptoOptions(
            for: .options, slug: "binance_com", instrumentId: "BTC-200730-9000-C"
        ))
        #expect(!BarOptionsDeclareSurface.usesThreeZone(
            for: .options, slug: "binance_com", instrumentId: "BTC-200730-9000-C"
        ))

        // A pair left sitting on the Options tab is not crypto options.
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTCUSDT"
        ) == .standardForm)

        // Spot / equity never leave the standard form on either desk.
        for slug in ["kotak_neo", "binance_com"] {
            for klass in [BarDeclareAssetClass.spot, .equity] {
                #expect(BarOptionsDeclareSurface.surface(
                    for: klass, slug: slug, instrumentId: "BTC-200730-9000-C"
                ) == .standardForm)
            }
        }

        // Dated contracts split by instrument shape — execution desk does not gate the surface.
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: nil, instrumentId: "BTC-200730-9000-C"
        ) == .cryptoOptions)
        #expect(!BarOptionsDeclareSurface.usesThreeZone(
            for: .options, slug: nil, instrumentId: "nse_fo|12345"
        ))
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

    @Test func datedBinanceContractGlancesTheNamedOptionsBook() {
        let plan = DeskExtractPlan.resolve(
            slug: "binance_com",
            assetClass: .options,
            instrumentId: "BTC-200730-9000-C"
        )
        #expect(plan.fetchesGlance)
        #expect(!plan.usesKotakHistoryObtain)

        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        #expect(vm.deskChainExtractPath(symbol: "BTC-200730-9000-C").contains("book=binance-com-options"))
        #expect(vm.deskOiExtractPath(symbol: "BTC-200730-9000-C").contains("book=binance-com-options"))
        #expect(vm.deskChainExtractPath(symbol: "BTC-200730-9000-C").contains("BTC-200730-9000-C"))
        #expect(!vm.deskChainExtractPath(symbol: "BTC-200730-9000-C").contains("kotak-nse-nfo"))

        // Typed leftover `BTC` must not replace the selected dated contract on glance.
        vm.barDeclarationSymbol = "BTC"
        #expect(vm.deskChainExtractPath(symbol: "BTC").contains("instrument=BTC-200730-9000-C"))
        #expect(vm.deskOiExtractPath(symbol: "BTC").contains("instrument=BTC-200730-9000-C"))
        #expect(!vm.deskOiExtractPath(symbol: "BTC").contains("instrument=BTC&"))
    }

    @Test func leftoverSpotPairOnOptionsTabStillIssuesNoGlance() {
        let plan = DeskExtractPlan.resolve(
            slug: "binance_com",
            assetClass: .options,
            instrumentId: "BTCUSDT"
        )
        #expect(!plan.fetchesGlance)
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

    // MARK: - Crypto options desk (Binance + dated contract)

    @Test func binanceDatedContractBindsPremiumVerbatim() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTC-200730-9000-C",
            "data": ["last": "0.001"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.deskQuoteCapability == "fresh")
        // %.2f would print 0.00 — the wire string is the only honest seed.
        #expect(vm.declEntryPrice == "0.001")
        #expect(vm.declEntryPrice != "0.00")
    }

    /// The eapi ticker is REST with no exchange timestamp, so every options tick carries
    /// `age_unknown` and the envelope says `unknown` — never `fresh`. Last must still bind:
    /// `unknown` is a freshness claim the desk declines to make, not a hole.
    @Test func binanceOptionsEnvelopeIsUnknownFreshnessAndStillBinds() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "unknown",
            "instrument_id": "BTC-200730-9000-C",
            "data": ["last": "0.001", "as_of": "2026-08-29T00:00:00.000Z"],
            "provenance": ["adapter_id": "binance_com", "transport": "rest"],
        ])
        #expect(vm.deskLastStatus == "unknown")
        #expect(vm.deskQuoteCapability == "unknown")
        #expect(vm.declEntryPrice == "0.001")
        // `unknown` is not honesty dialect, so the strip prints the wire word, not a chip.
        #expect(HonestyStatus.fromWire(vm.deskLastStatus) == nil)
    }

    @Test func kotakDeskAcceptsTheCryptoOptionsEnvelope() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTC-200730-9000-C",
            "data": ["last": "0.001"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.declEntryPrice == "0.001")
        #expect(!vm.canExecuteSelectedInstrument())
    }

    @Test func optionsBindTruthTableIsTwoKeyed() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.declareAssetClass = .options

        vm.activeExecutionBrokerSlug = "kotak_neo"
        #expect(vm.shouldBindQuoteLast(adapter: "kotak_neo", instrumentId: "nse_fo|12345"))
        #expect(!vm.shouldBindQuoteLast(adapter: "kotak_neo", instrumentId: "nse_cm|2885"))
        #expect(vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "BTC-200730-9000-C"))

        vm.activeExecutionBrokerSlug = "binance_com"
        #expect(vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "BTC-200730-9000-C"))
        #expect(!vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "BTCUSDT"))
        #expect(!vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "nse_fo|12345"))
        #expect(!vm.shouldBindQuoteLast(adapter: nil, instrumentId: ""))
    }

    @Test func optionsQuotePathNamesTheEapiBookOnlyForDatedBinance() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        let crypto = vm.deskQuoteExtractPath(instrument: "BTC-200730-9000-C")
        #expect(crypto.contains("instrument=BTC-200730-9000-C"))
        #expect(crypto.contains("&book=binance-com-options"))
        // A pair on the same desk is not the options book.
        #expect(!vm.deskQuoteExtractPath(instrument: "BTCUSDT").contains("book="))

        vm.activeBrokerSlug = "kotak_neo"
        #expect(!vm.deskQuoteExtractPath(instrument: "nse_fo|12345").contains("book="))
        #expect(vm.deskQuoteExtractPath(instrument: "nse_fo|12345").contains("instrument=nse_fo%7C12345"))
        vm.declareAssetClass = .equity
        #expect(!vm.deskQuoteExtractPath(instrument: "nse_cm|2885").contains("book="))
    }

    @Test func cryptoOptionsLastGoesDarkWhenTheTabRebindsOntoAPair() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTC-200730-9000-C",
            "data": ["last": "0.001"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "fresh")

        // Rebind onto a pair: the premium standing on screen belongs to a contract that
        // is no longer selected, and a spot last may not stand in for it.
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.invalidateDeskMarketExtracts(reason: "select-symbol")
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(!vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "BTCUSDT"))

        // And the same transition through the class switch stays dark.
        vm.declareAssetClass = .spot
        vm.declareAssetClass = .options
        #expect(vm.deskLastStatus == "unavailable")
    }

    // MARK: - commitDeskSymbol (paste-bind)

    @Test func pastedDatedContractArmsTheCryptoSurfaceOnCommit() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        // A pair left over from the last selection — the catalog is spot-only, so this is
        // the only thing a Binance desk can have been bound to.
        vm.deskSelectedInstrumentId = "BTCUSDT"
        #expect(BarOptionsDeclareSurface.surface(
            for: vm.declareAssetClass,
            slug: vm.resolvedDeskSlug,
            instrumentId: vm.deskSelectedInstrumentId
        ) == .standardForm)

        vm.barDeclarationSymbol = "BTC-250926-90000-C"
        vm.commitDeskSymbol()

        #expect(vm.deskSelectedInstrumentId == "BTC-250926-90000-C")
        #expect(BarOptionsDeclareSurface.surface(
            for: vm.declareAssetClass,
            slug: vm.resolvedDeskSlug,
            instrumentId: vm.deskSelectedInstrumentId
        ) == .cryptoOptions)
        // The contract splits into the fields that surface renders.
        #expect(vm.barDeclarationSymbol == "BTC")
        #expect(vm.declOptionExpiry == "250926")
        #expect(vm.declOptionStrike == "90000")
        #expect(vm.declOptionRight == "CE")
        #expect(vm.barDeclarationLastError == nil)
        // The glance now names the eapi book for the contract, not the leftover pair.
        #expect(vm.deskChainExtractPath(symbol: "BTC").contains("book=binance-com-options"))
        #expect(vm.deskChainExtractPath(symbol: "BTC").contains("instrument=BTC-250926-90000-C"))
    }

    @Test func leftoverPairCommitsNothingAndKeepsTheStandardForm() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-250926-90000-C"
        vm.declOptionExpiry = "250926"
        vm.declOptionStrike = "90000"

        vm.barDeclarationSymbol = "BTCUSDT"
        vm.commitDeskSymbol()

        // A pair is not a contract, so nothing binds — the typed text stays exactly as
        // typed and the previously bound contract is not rewritten by it.
        #expect(vm.barDeclarationSymbol == "BTCUSDT")
        #expect(vm.deskSelectedInstrumentId == "BTC-250926-90000-C")
        #expect(vm.declOptionExpiry == "250926")
        #expect(vm.declOptionStrike == "90000")
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTCUSDT"
        ) == .standardForm)
    }

    @Test func datedContractOnAKotakDeskBindsToBinanceComOptions() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"

        vm.barDeclarationSymbol = "BTC-250926-90000-C"
        vm.commitDeskSymbol()

        #expect(vm.deskSelectedInstrumentId == "BTC-250926-90000-C")
        #expect(vm.barDeclarationLastError == nil)
        #expect(vm.declOptionExpiry == "250926")
        #expect(vm.selectedMarketBookId == BarDeskTemplate.binanceComOptionsBookId)
        #expect(BarOptionsDeclareSurface.surface(
            for: vm.declareAssetClass,
            slug: vm.resolvedDeskSlug,
            instrumentId: vm.deskSelectedInstrumentId
        ) == .cryptoOptions)
        #expect(!vm.canExecuteSelectedInstrument())
    }

    @Test func recommittingTheSameContractDoesNotReinvalidate() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-250926-90000-C"
        vm.barDeclarationSymbol = "BTC-250926-90000-C"
        let generation = vm.deskExtractGeneration

        // Blur fires on every focus toggle; a no-op commit must not wipe live extracts.
        vm.commitDeskSymbol()
        vm.commitDeskSymbol()

        #expect(vm.deskExtractGeneration == generation)
    }

    @Test func pastedContractDropsTheLeftoverSpotEntry() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSyncClass = "synced"
        vm.declareAssetClass = .spot
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.declEntryPrice = "65000.00"

        vm.barDeclarationSymbol = "BTC-260925-145000-C"
        vm.commitDeskSymbol()

        // A spot-shaped Last is not a premium: it must not stand in Entry while the eapi
        // book answers for the contract now bound.
        #expect(vm.deskSelectedInstrumentId == "BTC-260925-145000-C")
        #expect(vm.declEntryPrice.isEmpty)
    }

    @Test func pastingAContractOnTheSpotTabDoesNotArmOptions() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .spot

        vm.barDeclarationSymbol = "BTC-250926-90000-C"
        vm.commitDeskSymbol()

        // The id binds, but the tab is the user's — picking a contract never silently
        // arms the Options surface.
        #expect(vm.deskSelectedInstrumentId == "BTC-250926-90000-C")
        #expect(vm.declareAssetClass == .spot)
        #expect(BarOptionsDeclareSurface.surface(
            for: vm.declareAssetClass,
            slug: vm.resolvedDeskSlug,
            instrumentId: vm.deskSelectedInstrumentId
        ) == .standardForm)
    }

    // MARK: - Greeks (venue_published mark pass-through, Binance options desk)

    /// The one that matters. `rights.display == false` means Station may not print the
    /// venue's number, even though `data.delta` is right there in the envelope.
    @Test func greeksRightsDisplayFalseNeverPaintsANumber() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyGreeksEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "delta": "0.55937056",
                "gamma": "0.00010969",
                "theta": "3739.82509871",
                "vega": "978.58874732",
            ],
            "provenance": ["model": "venue_published", "path": "/eapi/v1/mark"],
            "rights": ["research_fetch": true, "display": false, "derived": true],
            "ineligible": [],
        ])
        #expect(vm.deskGreeksDisplay == false)
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksGamma == nil)
        #expect(vm.deskGreeksTheta == nil)
        #expect(vm.deskGreeksVega == nil)
        #expect(vm.deskGreeksProv.isEmpty)
    }

    @Test func greeksLitEnvelopePaintsTheVenueStringsVerbatim() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyGreeksEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "mark_price": "1195.5",
                "delta": "0.55937056",
                "gamma": "0.00010969",
                "theta": "3739.82509871",
                "vega": "978.58874732",
                "mark_iv": "78.86",
                "ask_iv": "80.10",
            ],
            "provenance": [
                "model": "venue_published",
                "input_at": "2026-08-31T09:00:00Z",
                "adapter_id": "binance_com",
                "path": "/eapi/v1/mark",
            ],
            "source": ["kind": "venue_published", "path": "/eapi/v1/mark", "adapter": "binance_com"],
            "rights": ["research_fetch": true, "store": false, "display": true, "derived": true],
            "ineligible": [],
        ])
        #expect(vm.deskGreeksStatus == "success")
        #expect(vm.deskGreeksDisplay)
        // The venue's own text, digit for digit — no Double round-trip invents precision.
        #expect(vm.deskGreeksDelta == "0.55937056")
        #expect(vm.deskGreeksGamma == "0.00010969")
        #expect(vm.deskGreeksTheta == "3739.82509871")
        #expect(vm.deskGreeksVega == "978.58874732")
        #expect(vm.deskGreeksProv == "venue_published · /eapi/v1/mark")
        // Lit is not an honesty state — there is no fifth chip.
        #expect(HonestyStatus.fromWire(vm.deskGreeksStatus) == nil)
    }

    @Test func greeksDarkEnvelopeLeavesEveryNumberNil() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyGreeksEnvelope([
            "status": "unavailable",
            "data": NSNull(),
            "rights": ["research_fetch": true, "display": true, "derived": true],
            "ineligible": ["mark_snapshot_unavailable"],
        ])
        #expect(vm.deskGreeksStatus == "unavailable")
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksVega == nil)
        #expect(vm.deskGreeksProv.isEmpty)
        // A dark greeks row is a chip in the shared dialect, never a fifth state.
        #expect(HonestyStatus.fromWire(vm.deskGreeksStatus) == .unavailable)
    }

    @Test func greeksPartialEnvelopeIsNotHalfLit() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        // Venue sent three of four; the grid must not print three numbers and one chip.
        vm.applyGreeksEnvelope([
            "status": "success",
            "data": ["delta": "0.55937056", "gamma": "0.00010969", "theta": "3739.82509871", "vega": ""],
            "provenance": ["model": "venue_published", "path": "/eapi/v1/mark"],
            "rights": ["display": true],
        ])
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksGamma == nil)
        #expect(vm.deskGreeksTheta == nil)
        #expect(vm.deskGreeksVega == nil)
    }

    /// Never `as? Double`. A venue string that Swift could parse as a number must still
    /// arrive as text, and a numeric JSON value is not the contract — it does not paint.
    @Test func greeksNumericJsonValueIsNotTheStringContract() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyGreeksEnvelope([
            "status": "success",
            "data": ["delta": 0.55937056, "gamma": 0.00010969, "theta": 3739.82509871, "vega": 978.58874732],
            "provenance": ["model": "venue_published", "path": "/eapi/v1/mark"],
            "rights": ["display": true],
        ])
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksVega == nil)
    }

    @Test func greeksExtractPathNamesTheOptionsBookAndTheDatedContract() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        let path = vm.deskGreeksExtractPath(symbol: "BTC-200730-9000-C")
        #expect(path.contains("/api/station/greeks?"))
        #expect(path.contains("book=binance-com-options"))
        #expect(path.contains("instrument=BTC-200730-9000-C"))
        #expect(!path.contains("kotak-nse-nfo"))
    }

    /// Mirrors the OI leftover-ticker test: a stale typed `BTC` must not replace the
    /// selected dated contract on the greeks path either.
    @Test func greeksPathKeepsTheSelectedContractOverALeftoverTypedTicker() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.barDeclarationSymbol = "BTC"
        #expect(vm.deskGreeksExtractPath(symbol: "BTC").contains("instrument=BTC-200730-9000-C"))
        #expect(!vm.deskGreeksExtractPath(symbol: "BTC").contains("instrument=BTC&"))
        #expect(vm.deskGreeksExtractPath(symbol: "BTC").hasSuffix("BTC-200730-9000-C"))
    }

    /// A bookless options desk issues no glance at all — so it issues no greeks request.
    @Test func booklessOptionsDeskIssuesNoGreeksRequest() {
        #expect(!DeskExtractPlan.resolve(slug: "binance_com", assetClass: .options).fetchesGlance)
        #expect(!DeskExtractPlan.resolve(slug: "binance_com", assetClass: .options, instrumentId: "BTCUSDT").fetchesGlance)
        #expect(!DeskExtractPlan.resolve(slug: nil, assetClass: .options).fetchesGlance)
        #expect(DeskExtractPlan.resolve(slug: "binance_com", assetClass: .options, instrumentId: "BTC-200730-9000-C").fetchesGlance)
    }

    /// NFO greeks are an OPTIONS-PRICING blocker and stay dark. No number, ever.
    @Test func nfoGreeksStayDarkWithNoNumber() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        #expect(vm.deskGreeksStatus == "unavailable")
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksDisplay == false)
        // The NFO surface only ever renders a chip, and the dialect has no lit case.
        #expect(HonestyStatus.fromWire("success") == nil)
        #expect(!HonestyStatus.allCases.contains { $0.rawValue == "success" })
        #expect(HonestyStatus.fromWire("inherited_dark") == .inheritedDark)
    }

    @Test func invalidateWipesEveryGreeksFieldUnderTheSameGeneration() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyGreeksEnvelope([
            "status": "success",
            "data": ["delta": "0.55937056", "gamma": "0.00010969", "theta": "3739.82509871", "vega": "978.58874732"],
            "provenance": ["model": "venue_published", "path": "/eapi/v1/mark"],
            "rights": ["display": true],
        ])
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "sumOpenInterest": "12.5",
                "sumOpenInterestUsd": "1000.00",
                "timestamp": "1597026383085",
            ],
        ])
        vm.applyStationChainEnvelope([
            "status": "success",
            "data": [
                "row_count": 1,
                "rows": [[
                    "instrument_id": "BTC-200730-9000-C",
                    "trading_symbol": "BTC-200730-9000-C",
                    "option_type": "CALL",
                    "strike_raw": "9000.000",
                    "expiry_raw": "1596067200000",
                ]],
            ],
        ])
        #expect(vm.deskGreeksDelta != nil)
        #expect(vm.deskOiSumOpenInterest == "12.5")
        #expect(vm.deskChainRows.count == 1)
        let before = vm.deskExtractGeneration

        let after = vm.invalidateDeskMarketExtracts(reason: "select-symbol")

        #expect(after != before)
        #expect(vm.deskGreeksStatus == "unavailable")
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksGamma == nil)
        #expect(vm.deskGreeksTheta == nil)
        #expect(vm.deskGreeksVega == nil)
        #expect(vm.deskGreeksDisplay == false)
        #expect(vm.deskGreeksProv.isEmpty)
        // Same generation bump as chain/OI — one rebind, one wipe.
        #expect(vm.deskChainStatus == "unavailable")
        #expect(vm.deskOiStatus == "unavailable")
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiRows.isEmpty)
        #expect(vm.deskChainRows.isEmpty)
    }

    @Test func contractSwitchWipesGreeksSynchronously() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyGreeksEnvelope([
            "status": "success",
            "data": ["delta": "0.55937056", "gamma": "0.00010969", "theta": "3739.82509871", "vega": "978.58874732"],
            "provenance": ["model": "venue_published", "path": "/eapi/v1/mark"],
            "rights": ["display": true],
        ])
        #expect(vm.deskGreeksVega == "978.58874732")

        // An in-flight reply for the OLD contract must never paint the new one.
        vm.declareAssetClass = .spot

        #expect(vm.deskGreeksStatus == "unavailable")
        #expect(vm.deskGreeksVega == nil)
        #expect(vm.deskExtractInvalidationReason == "asset-class")
    }

    // MARK: - OI LatestState paint (eapi sumOpenInterest, crypto Options desk)

    @Test func oiExactMatchPaintsVenueStringsVerbatim() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "sumOpenInterest": "12.5",
                "sumOpenInterestUsd": "1000.00",
                "timestamp": "1597026383085",
            ],
        ])
        #expect(vm.deskOiStatus == "success")
        // Venue text, digit for digit — not 12.50, not a Double round-trip.
        #expect(vm.deskOiSumOpenInterest == "12.5")
        #expect(vm.deskOiSumOpenInterestUsd == "1000.00")
        #expect(vm.deskOiTimestamp == "1597026383085")
        #expect(vm.deskOiSymbol == "BTC-200730-9000-C")
        #expect(vm.deskOiRows.isEmpty)
        #expect(HonestyStatus.fromWire(vm.deskOiStatus) == nil)
    }

    @Test func oiEmptyStringIsMissingNotZero() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "sumOpenInterest": "",
                "sumOpenInterestUsd": "1000.00",
                "timestamp": "1597026383085",
            ],
        ])
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiSumOpenInterest != "0")
        #expect(vm.deskOiRows.isEmpty)
    }

    @Test func oiVenueZeroStringMayPaint() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "sumOpenInterest": "0",
            ],
        ])
        #expect(vm.deskOiSumOpenInterest == "0")
    }

    @Test func oiNumericJsonValueIsNotTheStringContract() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "symbol": "BTC-200730-9000-C",
                "sumOpenInterest": 12.5,
                "sumOpenInterestUsd": 1000.00,
                "timestamp": 1597026383085,
            ],
        ])
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiSumOpenInterestUsd == nil)
        #expect(vm.deskOiTimestamp == nil)
    }

    @Test func oiRowsListDoesNotInventATotal() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "row_count": 2,
                "rows": [
                    [
                        "symbol": "BTC-200730-9000-P",
                        "sumOpenInterest": "3.0",
                        "sumOpenInterestUsd": "200.00",
                        "timestamp": "1596067200000",
                    ],
                    [
                        "symbol": "ETH-200730-400-C",
                        "sumOpenInterest": "12.5",
                        "sumOpenInterestUsd": "1000.00",
                        "timestamp": "1596067200000",
                    ],
                ],
            ],
        ])
        #expect(vm.deskOiStatus == "success")
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiSumOpenInterestUsd == nil)
        #expect(vm.deskOiSymbol == nil)
        #expect(vm.deskOiRows.count == 2)
        #expect(vm.deskOiRows[0].symbol == "BTC-200730-9000-P")
        #expect(vm.deskOiRows[0].sumOpenInterest == "3.0")
        #expect(vm.deskOiRows[1].symbol == "ETH-200730-400-C")
        #expect(vm.deskOiRows[1].sumOpenInterest == "12.5")
        // Never 3.0+12.5.
        #expect(vm.deskOiSumOpenInterest != "15.5")
    }

    @Test func oiEmptyRowsStayUnavailableHonesty() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": ["row_count": 0, "rows": [] as [Any]],
        ])
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiRows.isEmpty)
        #expect(vm.deskOiSumOpenInterest != "0")
    }

    @Test func oiDarkEnvelopeLeavesEveryNumberNil() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationOiEnvelope([
            "status": "unavailable",
            "data": NSNull(),
        ])
        #expect(vm.deskOiStatus == "unavailable")
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiRows.isEmpty)
        #expect(HonestyStatus.fromWire(vm.deskOiStatus) == .unavailable)
    }

    @Test func nfoOiEnvelopeDoesNotPaintEapiFields() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": [
                "open_interest": "480750",
                "field": "open_int",
                "instrument_id": "nse_fo|12345",
            ],
        ])
        // Wire status stays the NFO glance word — the crypto number fields stay empty.
        #expect(vm.deskOiStatus == "success")
        #expect(vm.deskOiSumOpenInterest == nil)
        #expect(vm.deskOiRows.isEmpty)
        #expect(BarOptionsDeclareSurface.usesThreeZone(
            for: .options, slug: "kotak_neo", instrumentId: "nse_fo|12345"
        ))
        #expect(!BarOptionsDeclareSurface.usesCryptoOptions(
            for: .options, slug: "kotak_neo", instrumentId: "nse_fo|12345"
        ))
    }

    @Test func spotOptionsTabStillNotCryptoOi() {
        #expect(BarOptionsDeclareSurface.surface(
            for: .options, slug: "binance_com", instrumentId: "BTCUSDT"
        ) == .standardForm)
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.applyStationOiEnvelope([
            "status": "success",
            "data": ["sumOpenInterest": "12.5", "symbol": "BTCUSDT"],
        ])
        // Apply is book-agnostic; the Options tab still is not the crypto surface.
        #expect(!BarOptionsDeclareSurface.usesCryptoOptions(
            for: .options, slug: "binance_com", instrumentId: "BTCUSDT"
        ))
    }

    // MARK: - Chain catalog paint (optionSymbols list, crypto Options desk)

    @Test func chainCatalogPaintsVenueRowsVerbatim() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.applyStationChainEnvelope([
            "status": "success",
            "data": [
                "row_count": 2,
                "rows": [
                    [
                        "instrument_id": "BTC-200730-9000-C",
                        "trading_symbol": "BTC-200730-9000-C",
                        "option_type": "CALL",
                        "strike_raw": "9000.000",
                        "expiry_raw": "1596067200000",
                    ],
                    [
                        "instrument_id": "BTC-200730-9000-P",
                        "trading_symbol": "BTC-200730-9000-P",
                        "option_type": "PUT",
                        "strike_raw": "9000.000",
                        "expiry_raw": "1596067200000",
                        "last": "12.50",
                    ],
                ],
            ],
        ])
        #expect(vm.deskChainStatus == "success")
        #expect(vm.deskChainRows.count == 2)
        #expect(vm.deskChainRows[0].symbol == "BTC-200730-9000-C")
        #expect(vm.deskChainRows[0].strikeRaw == "9000.000")
        #expect(vm.deskChainRows[0].side == "CALL")
        #expect(vm.deskChainRows[0].expiryRaw == "1596067200000")
        #expect(vm.deskChainRows[0].last == nil)
        #expect(vm.deskChainRows[1].symbol == "BTC-200730-9000-P")
        #expect(vm.deskChainRows[1].last == "12.50")
        // Catalog is not a Sensibull strike grid.
        #expect(BarOptionsChainPresentation.from(
            underlying: "BTC", chainStatus: vm.deskChainStatus
        ).showsStrikeGrid == false)
        #expect(vm.deskChainRows[0].strikeRaw != "57500")
    }

    @Test func chainLastOverlayAbsentWhenEmptyNeverZero() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationChainEnvelope([
            "status": "success",
            "data": [
                "rows": [[
                    "instrument_id": "BTC-200730-9000-C",
                    "trading_symbol": "BTC-200730-9000-C",
                    "option_type": "CALL",
                    "strike_raw": "9000.000",
                    "expiry_raw": "1596067200000",
                    "last": "0",
                ]],
            ],
        ])
        #expect(vm.deskChainRows.count == 1)
        #expect(vm.deskChainRows[0].last == nil)
        #expect(vm.deskChainRows[0].last != "0")
    }

    @Test func chainNumericLastIsNotTheStringContract() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyStationChainEnvelope([
            "status": "success",
            "data": [
                "rows": [[
                    "instrument_id": "BTC-200730-9000-C",
                    "trading_symbol": "BTC-200730-9000-C",
                    "option_type": "CALL",
                    "strike_raw": "9000.000",
                    "expiry_raw": "1596067200000",
                    "last": 12.5,
                ]],
            ],
        ])
        #expect(vm.deskChainRows[0].last == nil)
    }

    @Test func chainEmptyStatusStaysNoContracts() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.barDeclarationSymbol = "BTC"
        vm.applyStationChainEnvelope([
            "status": "empty",
            "data": NSNull(),
        ])
        #expect(vm.deskChainRows.isEmpty)
        #expect(BarOptionsChainPresentation.from(
            underlying: "BTC", chainStatus: vm.deskChainStatus
        ) == .empty)
        #expect(BarOptionsChainPresentation.from(
            underlying: "BTC", chainStatus: "empty"
        ).showsStrikeGrid == false)
    }

    @Test func chainCatalogClickBindsMixedCaseId() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.barDeclarationSymbol = "BTC"
        vm.declOptionExpiry = "200730"
        vm.declOptionStrike = "9000"

        vm.bindChainCatalogSymbol("BTC-200730-9000-P")

        #expect(vm.deskSelectedInstrumentId == "BTC-200730-9000-P")
        #expect(vm.barDeclarationSymbol == "BTC")
        #expect(vm.declOptionExpiry == "200730")
        #expect(vm.declOptionStrike == "9000")
        #expect(vm.declOptionRight == "PE")
        #expect(vm.deskExtractInvalidationReason == "commit-symbol")
        #expect(BarOptionsDeclareSurface.usesCryptoOptions(
            for: .options, slug: "binance_com", instrumentId: vm.deskSelectedInstrumentId
        ))
    }

    @Test func chainCatalogIgnoresNfoTokenClick() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTC-200730-9000-C"
        vm.bindChainCatalogSymbol("nse_fo|56526")
        #expect(vm.deskSelectedInstrumentId == "BTC-200730-9000-C")
    }

    @Test func nfoChainEnvelopeDoesNotInventAStrikeGrid() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|12345"
        vm.applyStationChainEnvelope([
            "status": "success",
            "data": [
                "row_count": 1,
                "rows": [[
                    "instrument_id": "nse_fo|56526",
                    "trading_symbol": "NIFTY2692221000PE",
                    "lot": 65,
                    "option_type": "PE",
                    "strike_raw": "2.1e+06",
                    "expiry_raw": "1474554600",
                ]],
            ],
        ])
        #expect(vm.deskChainStatus == "success")
        #expect(vm.deskChainRows.count == 1)
        #expect(vm.deskChainRows[0].strikeRaw == "2.1e+06")
        #expect(vm.deskChainRows[0].strikeRaw != "57500")
        #expect(BarOptionsDeclareSurface.usesThreeZone(
            for: .options, slug: "kotak_neo", instrumentId: "nse_fo|12345"
        ))
        #expect(BarOptionsChainPresentation.from(
            underlying: "BANKNIFTY", chainStatus: "success"
        ).showsStrikeGrid == false)
    }
}

// MARK: - Greeks provenance has three states, not two

@MainActor
@Test func greeksNotAskedIsWaitingButADarkReplyIsNot() {
    let vm = NotchViewModel()

    // Nothing asked yet: "waiting" is true.
    #expect(vm.deskGreeksAsked == false)

    // Asked and got a hole back. The desk is not waiting — it has its answer.
    vm.applyGreeksEnvelope(["status": "unavailable", "data": NSNull()])
    #expect(vm.deskGreeksAsked)
    #expect(vm.deskGreeksStatus == "unavailable")
    #expect(vm.deskGreeksDelta == nil)

    // A rebind puts it back to genuinely-waiting.
    _ = vm.invalidateDeskMarketExtracts(reason: "select-symbol")
    #expect(vm.deskGreeksAsked == false)
}
