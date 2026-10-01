import Foundation
import Notch
import Testing
@testable import Station

@MainActor
struct ShippingFundsGlanceTests {
    @Test func applyObtainEnvelopeLightsFreeBalance() {
        let session = SessionModel()
        session.activeBrokerSlug = "kotak_neo"
        session.deskQuoteCurrency = "INR"
        let envelope: [String: Any] = [
            "status": "success",
            "book_id": "kotak-nse-bse-cash",
            "data": [
                "holdings": [
                    ["asset": "INR", "free": 19.41, "locked": 0],
                ],
            ],
        ]
        session.applyShippingFundsObtainEnvelope(envelope, startSlug: "kotak_neo")
        #expect(session.shippingFundsGlance.isLit)
        #expect(session.shippingFundsGlance.freeText == "INR 19.41")
    }

    @Test func disconnectedClearsGlanceOnSyncEvent() {
        let session = SessionModel()
        session.applyShippingFundsObtainEnvelope(
            [
                "status": "success",
                "book_id": "binance-com-spot",
                "data": ["holdings": [["asset": "USDT", "free": 1.0, "locked": 0]]],
            ],
            startSlug: "binance_com"
        )
        #expect(session.shippingFundsGlance.isLit)
        _ = session.applyDaemonEventPayload(
            type: "broker_sync_state",
            payload: ["class": "not_connected"]
        )
        #expect(session.shippingFundsGlance == .dark)
    }
}
