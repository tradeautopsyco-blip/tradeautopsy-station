import Testing
@testable import Notch

struct BarNotchScreenTests {
    @Test func shippingCopyIsHarnessOpenPlanWorkingDebrief() {
        #expect(BarNotchScreen.morning.rawValue == "Open")
        #expect(BarNotchScreen.pretrade.rawValue == "Plan")
        #expect(BarNotchScreen.live.rawValue == "Working")
        #expect(BarNotchScreen.posttrade.rawValue == "Debrief")
        #expect(BarNotchScreen.escrow.rawValue == "Match")
        #expect(BarNotchScreen.morning.navTitle == "Open")
        #expect(BarNotchScreen.pretrade.navTitle == "Plan")
        #expect(BarNotchScreen.live.navTitle == "Working")
        #expect(BarNotchScreen.posttrade.navTitle == "Debrief")
        #expect(BarNotchScreen.escrow.navTitle == "Match")
    }

    @Test func barNotchScreenHasAllNineRoutes() {
        #expect(BarNotchScreen.allCases.count == 9)
        #expect(Set(BarNotchScreen.allCases) == Set([
            .morning,
            .pretrade,
            .live,
            .posttrade,
            .escrow,
            .patterns,
            .fidelity,
            .triage,
            .settings,
        ]))
    }

    @Test func sessionGroupContainsFourSessionScreens() {
        let sessionScreens = BarNotchScreen.allCases.filter(\.isSessionGroup)
        #expect(sessionScreens.count == 4)
        #expect(sessionScreens.contains(.morning))
        #expect(sessionScreens.contains(.pretrade))
        #expect(sessionScreens.contains(.live))
        #expect(sessionScreens.contains(.posttrade))
    }

    @Test func deskGroupContainsFiveAuxiliaryScreens() {
        let deskScreens = BarNotchScreen.allCases.filter { !$0.isSessionGroup }
        #expect(deskScreens.count == 5)
        #expect(deskScreens.contains(.escrow))
        #expect(deskScreens.contains(.patterns))
        #expect(deskScreens.contains(.fidelity))
        #expect(deskScreens.contains(.triage))
        #expect(deskScreens.contains(.settings))
    }
}

struct BarNotchPhaseRoutingTests {
    @Test func declarationPhaseRoutesToPretrade() {
        #expect(BarNotchPhaseRouting.screen(for: .declaration) == .pretrade)
    }

    @Test func livePlanAndArmedPhasesRouteToLive() {
        #expect(BarNotchPhaseRouting.screen(for: .livePlan) == .live)
        #expect(BarNotchPhaseRouting.screen(for: .armed) == .live)
    }

    @Test func debriefPhaseRoutesToPosttrade() {
        #expect(BarNotchPhaseRouting.screen(for: .debrief) == .posttrade)
    }
}
