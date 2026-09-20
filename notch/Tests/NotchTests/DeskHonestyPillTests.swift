import Foundation
import Testing
@testable import Notch

@MainActor
struct DeskHonestyPillTests {
    @Test func comSlugPaintsUsdNotInr() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USD"
        vm.sessionPnL = 1_250
        let text = vm.formattedSessionPnL
        #expect(text.contains("1,250") || text.contains("1250"))
        #expect(!text.contains("₹"))
        #expect(text.contains("$") || text.contains("USD") || text.hasPrefix("+"))
    }

    @Test func unknownQuoteDashesNeverInr() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.deskQuoteCurrency = nil
        vm.activeExecutionBrokerSlug = nil
        vm.brokerSessionActive = false
        vm.sessionPnL = 1_250
        #expect(vm.formattedSessionPnL == "—")
        #expect(vm.formatDeskMoney(1_250) == "—")
    }

    @Test func mixedComAndKotakDashes() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USD"
        vm.sessionPnL = 1_250
        vm.venuePostureBySlug = [
            "binance_com": VenuePosture(venue: "binance_com", posture: "live"),
            "kotak_neo": VenuePosture(venue: "kotak_neo", posture: "live"),
        ]
        #expect(vm.formattedSessionPnL == "—")
        #expect(vm.formatDeskMoney(1_250) == "—")
    }

    @Test func optionsAndNfoDashWithoutStealingSpot() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USD"
        vm.sessionPnL = 11
        vm.declareAssetClass = .options
        #expect(vm.sessionPnLOwnerMissing)
        #expect(vm.formattedSessionPnL == "—")
        vm.declareAssetClass = .spot
        vm.selectedMarketBookId = BarDeskTemplate.kotakNfoBookId
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.deskQuoteCurrency = "INR"
        #expect(vm.formattedSessionPnL == "—")
    }

    @Test func exposureStaysDarkNotQtyTimes1000() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.positions = [
            NotchPosition(symbol: "RELIANCE", qty: 5, unrealizedPnL: 10, direction: "LONG"),
        ]
        #expect(vm.totalExposureText == "—")
        #expect(vm.totalExposureText != "5,000")
        #expect(!vm.totalExposureText.contains("5000"))
    }

    @Test func comStartedPollsOnSunday() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.activeExecutionBrokerSlug = "binance_com"
        let cal = BookSessionClock.istCalendar()
        var c = DateComponents()
        c.year = 2026
        c.month = 9
        c.day = 20
        c.hour = 12
        let sunday = cal.date(from: c)!
        #expect(!BookSessionClock.isNseCashSession(at: sunday, calendar: cal))
        #expect(vm.shouldPollMarketReads(now: sunday))
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.venuePostureBySlug = [:]
        #expect(!vm.shouldPollMarketReads(now: sunday))
    }
}
