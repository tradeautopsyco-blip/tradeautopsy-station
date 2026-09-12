import Foundation
import Testing
@testable import Notch

struct CollapsedNotchPresentationTests {
    @Test func interventionWinsOverImpact() {
        let notch = BarLiveStateResponse(
            planState: "RED",
            primarySentence: nil,
            triggerType: nil,
            isRedTerminal: true,
            composite: nil,
            activeInterventions: [
                ActiveIntervention(interventionType: "daily_loss", primaryMessage: "stop", expiresAt: nil),
            ],
            syncState: "GREEN",
            lastSyncAt: nil,
            declarationSubmitBlocked: true,
            pendingDeclaration: nil,
        )
        let p = CollapsedNotchPresentation.build(
            notch: notch,
            impact: .known(percent: 32, sign: .down),
            pnlText: "+₹1,250",
        )
        guard case .intervention(let keyword, _, _) = p.layout else {
            Issue.record("expected intervention")
            return
        }
        #expect(keyword == "LIMIT")
        #expect(p.pnlText == "+₹1,250")
    }

    @Test func noInterventionShowsImpactAndPnLForAnyArchetype() {
        let impact = AccountImpact.known(percent: 32, sign: .up)
        let p = CollapsedNotchPresentation.build(
            notch: nil,
            impact: impact,
            pnlText: "+₹1,250",
        )
        #expect(p.layout == .impact(impact))
        #expect(p.pillPulseAmber == false)
        #expect(p.pnlText == "+₹1,250")
    }
}
