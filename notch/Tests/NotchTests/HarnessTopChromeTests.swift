import Foundation
import Testing
@testable import Notch

@MainActor
struct HarnessTopChromeTests {
    @Test func screenshotNowOneBookTwoHolesNeverPrintsVendorSlug() {
        let chrome = HarnessTopChrome.compose(
            activeSlug: "binance_com",
            brokerSyncClass: "synced",
            venuePostureBySlug: [:],
            quoteStatus: "unavailable",
            instrumentsStatus: "fresh",
            accountStatus: "fresh",
            vendorFenceRows: [
                VendorFenceRow(
                    adapterId: "licensed_history",
                    status: "unsupported",
                    what: "licensed_history India ohlcv (Kotak has none)",
                    why: "unsupported",
                    obtainNoun: "history"
                ),
            ]
        )

        #expect(chrome.clusterDisplayNames == ["Binance.com"])
        #expect(chrome.statusLabel == "2 holes")
        #expect(chrome.statusKind == .bad)
        #expect(chrome.showsRetryInstruments == false)
        #expect(chrome.dataRows.map(\.nounLabel) == ["History"])
        #expect(chrome.dataRows.map(\.statusLabel) == ["Not eligible"])
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
            vendorFenceRows: [
                VendorFenceRow(
                    adapterId: "licensed_history",
                    status: "up",
                    what: "licensed_history India ohlcv",
                    why: "up",
                    obtainNoun: "history"
                ),
            ]
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
            vendorFenceRows: []
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
            vendorFenceRows: [
                VendorFenceRow(
                    adapterId: "licensed_history",
                    status: "up",
                    what: "history",
                    why: "up",
                    obtainNoun: "history"
                ),
                VendorFenceRow(
                    adapterId: "amfi",
                    status: "up",
                    what: "AMFI NAV (labs)",
                    why: "up",
                    obtainNoun: "amfi_nav"
                ),
            ]
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
            vendorFenceRows: []
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
            vendorFenceRows: [
                VendorFenceRow(
                    adapterId: "licensed_history",
                    status: "up",
                    what: "history",
                    why: "up",
                    obtainNoun: "history"
                ),
            ]
        )

        #expect(chrome.statusLabel == "Quote unavailable")
        #expect(chrome.statusKind == .bad)
        #expect(chrome.dataRows.map(\.statusLabel) == ["Eligible"])
    }
}
