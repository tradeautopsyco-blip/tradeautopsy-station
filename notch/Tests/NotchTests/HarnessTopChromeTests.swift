import Foundation
import Testing
@testable import Notch

@MainActor
struct HarnessTopChromeTests {
    private func licensedHistory(status: String) -> VendorFenceRow {
        VendorFenceRow(
            adapterId: "licensed_history",
            status: status,
            what: "licensed_history India ohlcv (Kotak has none)",
            why: status,
            obtainNoun: "history"
        )
    }

    private func amfi(status: String = "up") -> VendorFenceRow {
        VendorFenceRow(
            adapterId: "amfi",
            status: status,
            what: "AMFI NAV (labs)",
            why: status,
            obtainNoun: "amfi_nav"
        )
    }

    @Test func screenshotNowOneBookQuoteUnavailableIgnoresLicensedVendor() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "unavailable",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "unsupported")],
            deskHistoryStatus: "success",
            boundInstrumentId: "BTCUSDT"
        )

        #expect(chrome.clusterDisplayNames == ["Binance.com"])
        #expect(chrome.statusLabel == "Quote unavailable")
        #expect(chrome.statusKind == .bad)
        #expect(chrome.showsRetryInstruments == false)
        #expect(chrome.holes.contains(where: { $0.key == "History" }) == false)
        #expect(chrome.dataRows.map(\.nounLabel) == ["History"])
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible"])
        #expect(chrome.visibleChromeText.contains("licensed_history") == false)
        #expect(chrome.visibleChromeText.contains("Binance.com · live") == false)
        #expect(chrome.books.map(\.stateLabel) == ["Live"])
    }

    @Test func dualLiveBooksStayTwoNames() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [
                "binance_com": VenuePosture(venue: "binance_com", posture: "live"),
                "kotak_neo": VenuePosture(venue: "kotak_neo", posture: "live"),
            ],
            quoteStatus: "fresh",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "up")],
            deskHistoryStatus: "success",
            boundInstrumentId: "BTCUSDT"
        )

        #expect(chrome.clusterDisplayNames == ["Binance.com", "Kotak Neo"])
        #expect(Set(chrome.clusterDisplayNames).count == 2)
        #expect(chrome.statusLabel == "All clear")
        #expect(chrome.statusKind == .quiet)
        #expect(chrome.visibleChromeText.contains("licensed_history") == false)
        #expect(chrome.clusterDisplayNames.joined(separator: " / ").contains("Binance.com / Kotak Neo"))
    }

    @Test func bannedComLeavesKotakInClusterAndPopoverListsCom() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "kotak_neo",
            brokerSyncClass: "synced",
            venuePostureBySlug: [
                "binance_com": VenuePosture(venue: "binance_com", posture: "banned"),
                "kotak_neo": VenuePosture(venue: "kotak_neo", posture: "live"),
            ],
            quoteStatus: "fresh",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [],
            deskHistoryStatus: "",
            boundInstrumentId: ""
        )

        #expect(chrome.clusterDisplayNames == ["Kotak Neo"])
        #expect(chrome.books.map(\.displayName) == ["Kotak Neo", "Binance.com"])
        #expect(chrome.books.map(\.stateLabel) == ["Live", "Banned"])
        #expect(chrome.statusLabel == "All clear")
    }

    @Test func allHealthyIsQuietAndEligible() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [
                "binance_com": VenuePosture(venue: "binance_com", posture: "live"),
                "kotak_neo": VenuePosture(venue: "kotak_neo", posture: "live"),
            ],
            quoteStatus: "fresh",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "up"), amfi()],
            deskHistoryStatus: "success",
            boundInstrumentId: "BTC-260925-145000-C"
        )

        #expect(chrome.statusLabel == "All clear")
        #expect(chrome.holes.isEmpty)
        #expect(chrome.dataRows.map(\.nounLabel) == ["History", "NAV"])
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible", "Eligible"])
        #expect(chrome.visibleChromeText.contains("amfi") == false)
        #expect(chrome.visibleChromeText.contains("licensed_history") == false)
    }

    @Test func instrumentsUnavailableKeepsRetry() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "kotak_neo",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "fresh",
            instrumentsStatus: "unavailable",
            accountStatus: "fresh",
            vendorFenceRows: [],
            deskHistoryStatus: "",
            boundInstrumentId: ""
        )

        #expect(chrome.showsRetryInstruments == true)
        #expect(chrome.statusLabel == "Instruments unavailable")
        #expect(chrome.holes.contains(where: { $0.key == "Instruments" && $0.retryInstruments }))
    }

    @Test func singleQuoteHoleUsesQuoteLabel() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "unavailable",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "up")],
            deskHistoryStatus: "success",
            boundInstrumentId: "BTCUSDT"
        )

        #expect(chrome.statusLabel == "Quote unavailable")
        #expect(chrome.statusKind == .bad)
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible"])
    }

    @Test func quoteUnknownIsFreshnessNotAHole() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "unknown",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "unsupported"), amfi()],
            deskHistoryStatus: "success",
            boundInstrumentId: "BTC-260925-145000-C"
        )

        #expect(chrome.holes.contains(where: { $0.key == "Quote" }) == false)
        #expect(chrome.holes.contains(where: { $0.key == "History" }) == false)
        #expect(chrome.statusLabel == "All clear")
        #expect(chrome.dataRows.map(\.nounLabel) == ["History", "NAV"])
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible", "Eligible"])
        #expect(chrome.visibleChromeText.contains("licensed_history") == false)
    }

    @Test func indiaCashUsesVendorFillWhenBrokerHistoryIsDark() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "kotak_neo",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "fresh",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "unsupported")],
            deskHistoryStatus: "unsupported",
            boundInstrumentId: "nse_cm|2885"
        )

        #expect(HarnessTopChrome.usesIndiaVendorHistoryFill(
            boundInstrumentId: "nse_cm|2885",
            deskHistoryStatus: "unsupported"
        ))
        #expect(chrome.dataRows.map(\.nounLabel) == ["History"])
        #expect(chrome.dataRows.map(\.statusLabel) == ["Not eligible"])
        #expect(chrome.statusLabel == "History Not eligible")
        #expect(chrome.visibleChromeText.contains("licensed_history") == false)
    }

    @Test func indiaCashBrokerHistoryWinsOverDarkVendor() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "kotak_neo",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "fresh",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [licensedHistory(status: "unsupported")],
            deskHistoryStatus: "success",
            boundInstrumentId: "nse_cm|2885"
        )

        #expect(HarnessTopChrome.usesIndiaVendorHistoryFill(
            boundInstrumentId: "nse_cm|2885",
            deskHistoryStatus: "success"
        ) == false)
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible"])
        #expect(chrome.holes.isEmpty)
        #expect(chrome.statusLabel == "All clear")
    }

    @Test func unboundInstrumentOmitsLicensedHistoryRow() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "unavailable",
            instrumentsStatus: "fresh",
            accountStatus: "unavailable",
            vendorFenceRows: [licensedHistory(status: "unsupported"), amfi()],
            deskHistoryStatus: "",
            boundInstrumentId: ""
        )

        #expect(chrome.dataRows.map(\.nounLabel) == ["NAV"])
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible"])
        #expect(chrome.holes.map(\.key).sorted() == ["Account", "Quote"])
        #expect(chrome.statusLabel == "2 holes")
    }
}
