import Foundation
import Testing
@testable import Notch

struct NotchDeskHonestyTests {
    @Test func binanceComMapsToUsd() {
        let profile = DeskHonesty.profile(forBrokerSlug: "binance_com")
        #expect(profile?.quoteCurrency == "USD")
        #expect(profile?.calcProfileId == "crypto_spot_usd")
    }

    @Test func kotakNeoMapsToInr() {
        let profile = DeskHonesty.profile(forBrokerSlug: "kotak_neo")
        #expect(profile?.quoteCurrency == "INR")
        #expect(profile?.calcProfileId == "equities_inr_cash")
    }

    @Test func dualComPlusKotakDoesNotBlend() {
        let resolution = DeskHonesty.resolve(activeSlugs: ["binance_com", "kotak_neo"])
        guard case let .dualNoBlend(profiles) = resolution else {
            Issue.record("expected dualNoBlend, got \(resolution)")
            return
        }
        #expect(profiles.count == 2)
        #expect(DeskHonesty.heroQuoteCurrency(activeSlugs: ["binance_com", "kotak_neo"]) == nil)
    }

    @Test func singleActiveSlugFormatsHeroCurrency() {
        #expect(DeskHonesty.heroQuoteCurrency(activeSlugs: ["binance_com"]) == "USD")
        #expect(DeskHonesty.heroQuoteCurrency(activeSlugs: ["kotak_neo"]) == "INR")
    }
}
