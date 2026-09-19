import Foundation
import Testing
@testable import Notch

/// Phase 5 STOP facts. Lighting Session klines, at-expiry polyline, Greek numbers,
/// or a strike grid on NFO requires a new lock line in the PR body before product
/// code. These goldens must stay green until that lock exists.
@MainActor
struct BarNfoLockGatesTests {
    @Test func sessionStaysKotakHistoryUnsupported() {
        #expect(BarNfoHistoryCopy.sessionHoleBody.contains("kotak_history_unsupported"))
        #expect(!BarNfoHistoryCopy.sdkNamesNseFoOnHistoricalDetails)
        #expect(!BarNfoHistoryCopy.sessionHoleBody.contains("eapi"))
        #expect(!BarNfoHistoryCopy.sessionHoleBody.lowercased().contains("yahoo"))
    }

    @Test func payoffStaysChainInheritedHole() {
        #expect(BarNfoPayoffCopy.holeBody == "derived/payoff inherits market/option_chain")
        #expect(!BarNfoPayoffCopy.holeBody.contains("GET /eapi/v1/index"))
        #expect(!BarNfoPayoffCopy.holeBody.contains("Black-76"))
        #expect(!BarNfoPayoffCopy.holeBody.contains("settlement identity"))
    }

    @Test func greeksStayPricingModelUnspecifiedWithNoNumbers() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|56526"
        vm.applyGreeksEnvelope([
            "status": "unavailable",
            "data": NSNull(),
            "rights": ["research_fetch": true, "display": false],
            "ineligible": ["pricing_model_unspecified"],
        ])
        #expect(vm.deskGreeksDelta == nil)
        #expect(vm.deskGreeksGamma == nil)
        #expect(vm.deskGreeksTheta == nil)
        #expect(vm.deskGreeksDisplay == false)
        #expect(vm.deskGreeksIneligible.contains("pricing_model_unspecified"))
        let line = BarNfoGreeksCopy.provenance(
            asked: vm.deskGreeksAsked,
            status: vm.deskGreeksStatus,
            ineligible: vm.deskGreeksIneligible
        )
        #expect(line.contains("pricing_model_unspecified"))
        #expect(!line.contains("Black-76"))
        #expect(!line.contains("SPAN"))
    }

    @Test func strikeGridStaysOffAndCatalogIsTheChainProduct() {
        #expect(BarNfoCockpitSeed.catalog.contains(.chain))
        #expect(!BarNfoCockpitSeed.catalog.contains(where: { $0.rawValue == "strike" }))
        let titles = BarNfoCockpitSeed.catalog.map(BarNfoCockpitSeed.title)
        #expect(!titles.contains(where: { $0.lowercased().contains("strike grid") }))
        for status in ["unavailable", "empty", "unusable", "success"] {
            #expect(
                BarOptionsChainPresentation.from(underlying: "NIFTY", chainStatus: status)
                    .showsStrikeGrid == false
            )
        }
        #expect(
            BarOptionsChainPresentation.from(underlying: "", chainStatus: "unavailable")
                .showsStrikeGrid == false
        )
    }

    @Test func lockFixtureStrikeStaysRawScientific() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "nse_fo|56526"
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
        #expect(vm.deskChainRows[0].strikeRaw == "2.1e+06")
        #expect(vm.deskChainRows[0].strikeRaw != "21000")
        #expect(vm.deskChainRows[0].strikeRaw != "57500")
    }
}
