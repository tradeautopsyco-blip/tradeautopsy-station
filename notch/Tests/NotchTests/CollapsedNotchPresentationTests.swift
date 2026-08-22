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
        )
        guard case .intervention(let keyword, _, _) = p.layout else {
            Issue.record("expected intervention")
            return
        }
        #expect(keyword == "LIMIT")
    }

    @Test func noInterventionShowsImpactForAnyArchetype() {
        let impact = AccountImpact.known(percent: 32, sign: .up)
        let p = CollapsedNotchPresentation.build(notch: nil, impact: impact)
        #expect(p.layout == .impact(impact))
        #expect(p.pillPulseAmber == false)
    }
}
