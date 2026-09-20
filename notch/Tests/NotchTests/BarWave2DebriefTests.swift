import Foundation
import Testing
@testable import Notch

struct BarWave2DebriefTests {
    @Test func nfoAndOptionsCitedNetStayDash() {
        #expect(BarDebriefCitedNet.display(optionsOrNfo: true, net: 1100, currency: "INR") == "—")
    }

    @Test func cashCitedNetUsesTodayTrip() {
        let net = BarDebriefCitedNet.net(
            symbol: "RELIANCE",
            trips: [("HDFCBANK", 50), ("RELIANCE", 1100)]
        )
        #expect(net == 1100)
        let text = BarDebriefCitedNet.display(optionsOrNfo: false, net: net, currency: "INR")
        #expect(text.contains("1,100") || text.contains("1100"))
        #expect(text.contains("₹") || text.contains("INR"))
    }

    @Test func debriefPayloadLandsOnDeclarationIdWithEmotionOut() {
        let o = BarPostTradeDebriefPayload.buildJSONObject(
            momentANote: "Cited net",
            adherence: BarPostTradeAdherenceAnswers(
                stopAsDeclared: false,
                sizeAsDeclared: true,
                invalidationRespected: true,
                exitPerPlan: false,
                noImpulsiveAdd: true
            ),
            momentCContext: "cooling",
            momentCNote: "Do not chase.",
            declarationId: "22222222-3333-4444-8555-bbbbbbbbbbbb",
            completedAtMs: 1,
            liveNote: "Avg fill 1420",
            emotionOut: 3,
            captureIds: ["aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"]
        )
        #expect(o["declaration_id"] as? String == "22222222-3333-4444-8555-bbbbbbbbbbbb")
        #expect(o["emotion_out"] as? Int == 3)
        #expect(o["live_note"] as? String == "Avg fill 1420")
        #expect(o["moment_c_note"] as? String == "Do not chase.")
        #expect((o["capture_ids"] as? [String])?.count == 1)
    }

    @Test func impulsivePayloadIsReactiveWithoutDeclarationId() {
        let o = BarPostTradeDebriefPayload.buildJSONObject(
            momentANote: "",
            adherence: BarPostTradeAdherenceAnswers(
                stopAsDeclared: false,
                sizeAsDeclared: false,
                invalidationRespected: false,
                exitPerPlan: false,
                noImpulsiveAdd: false
            ),
            momentCContext: "cooling",
            momentCNote: "Filled without a plan.",
            declarationId: nil,
            completedAtMs: 1,
            impulsive: true,
            symbol: "RELIANCE",
            side: "BUY",
            quantity: 10,
            stance: "reactive"
        )
        #expect(o["declaration_id"] == nil)
        #expect(o["impulsive"] as? Bool == true)
        #expect(o["stance"] as? String == "reactive")
        #expect(o["symbol"] as? String == "RELIANCE")
    }
}
