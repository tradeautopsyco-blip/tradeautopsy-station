import Foundation
import Testing
@testable import Notch

struct DeskMoneyFormattingTests {
    @Test func binanceComSlugIsUsd() {
        #expect(DeskMoneyFormatting.quoteCurrency(forBrokerSlug: "binance_com") == "USD")
        #expect(DeskMoneyFormatting.calcProfileId(forBrokerSlug: "binance_com") == "crypto_spot_usd")
    }

    @Test func kotakNeoSlugIsInr() {
        #expect(DeskMoneyFormatting.quoteCurrency(forBrokerSlug: "kotak_neo") == "INR")
        #expect(DeskMoneyFormatting.calcProfileId(forBrokerSlug: "kotak_neo") == "equities_inr_cash")
    }

    @Test func formattersDoNotBlendCurrencies() {
        let usd = DeskMoneyFormatting.formatSigned(1_250, quoteCurrency: "USD")
        let inr = DeskMoneyFormatting.formatSigned(1_250, quoteCurrency: "INR")
        #expect(usd != inr)
        #expect(usd.contains("1,250") || usd.contains("1250"))
        #expect(inr.contains("1,250") || inr.contains("1250") || inr.contains("₹"))
    }
}
