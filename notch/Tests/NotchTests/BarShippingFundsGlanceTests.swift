import Foundation
import Testing
@testable import Notch

struct BarShippingFundsGlanceTests {
    @Test func kotakCashFundsLightsOnShippingBook() {
        let envelope: [String: Any] = [
            "status": "success",
            "book_id": "kotak-nse-bse-cash",
            "data": [
                "holdings": [
                    ["asset": "INR", "free": 19.41, "locked": 0],
                ],
            ],
        ]
        let glance = BarShippingFundsGlance.present(
            fundsEnvelope: envelope,
            shippingBookId: "kotak-nse-bse-cash",
            quoteCurrency: "INR"
        )
        #expect(glance.isLit)
        #expect(glance.freeText == "INR 19.41")
    }

    @Test func wrongBookIdStaysDark() {
        let envelope: [String: Any] = [
            "status": "success",
            "book_id": "binance-com-usdm",
            "data": [
                "holdings": [
                    ["asset": "USDT", "free": 1.04, "locked": 0],
                ],
            ],
        ]
        let glance = BarShippingFundsGlance.present(
            fundsEnvelope: envelope,
            shippingBookId: "binance-com-spot",
            quoteCurrency: "USDT"
        )
        #expect(!glance.isLit)
        #expect(glance.freeText == "—")
    }

    @Test func shippingContextNilForUnknownSlug() {
        #expect(BarShippingFundsGlance.shippingContext(startSlug: "bybit") == nil)
        #expect(BarShippingFundsGlance.shippingContext(startSlug: "binance_com")?.bookId == "binance-com-spot")
    }
}
