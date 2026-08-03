import Foundation
import Testing
@testable import Station

struct DeskHonestyTests {
    @Test func binanceComMapsToUsdCryptoSpotProfile() {
        let profile = DeskHonesty.profile(forBrokerSlug: "binance_com")
        #expect(profile?.quoteCurrency == "USD")
        #expect(profile?.calcProfileId == "crypto_spot_usd")
    }

    @Test func kotakNeoMapsToInrEquitiesProfile() {
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

    @Test func todayPresentationUsesInrWhenKotakActive() {
        let payload = TodayAgentPayload(
            localDate: "2026-07-25",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 1_250, tradesToday: 2, winRate: 0.5),
            topSignals: [],
            trades: [],
            openPositionCount: 0,
            brokerSlug: "kotak_neo",
            quoteCurrency: "INR",
            calcProfileId: "equities_inr_cash"
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        let pnl = presentation.heroTiles.first { $0.id == "pnl" }?.value ?? ""
        #expect(pnl.contains("₹") || pnl.contains("INR") || pnl.contains("1,250") || pnl.contains("1250"))
        #expect(!pnl.contains("$"))
    }

    @Test func todayPresentationUsesUsdWhenComActive() {
        let payload = TodayAgentPayload(
            localDate: "2026-07-25",
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(pnlTodayUsd: 42.5, tradesToday: 1, winRate: 1.0),
            topSignals: [],
            trades: [],
            openPositionCount: 0,
            brokerSlug: "binance_com",
            quoteCurrency: "USD",
            calcProfileId: "crypto_spot_usd"
        )
        let presentation = TodayScreenPresentation.build(payload: payload, agentHealthy: true)
        let pnl = presentation.heroTiles.first { $0.id == "pnl" }?.value ?? ""
        #expect(pnl.contains("42"))
        #expect(pnl.contains("$") || pnl.contains("USD"))
    }

    @Test func deskMoneyFormattingFirstPairDogfood() {
        let usd = DeskMoneyFormatting.formatSigned(100, quoteCurrency: "USD")
        let inr = DeskMoneyFormatting.formatSigned(100, quoteCurrency: "INR")
        #expect(usd.contains("100"))
        #expect(inr.contains("100"))
        #expect(usd != inr)
        #expect(DeskMoneyFormatting.quoteCurrency(forBrokerSlug: "binance_com") == "USD")
        #expect(DeskMoneyFormatting.quoteCurrency(forBrokerSlug: "kotak_neo") == "INR")
    }
}
