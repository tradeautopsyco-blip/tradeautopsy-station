import Foundation
import Testing
@testable import Notch

struct BarPlanHorizonTests {
    @Test func defaultForKindValues() {
        #expect(BarPlanHorizon.defaultFor("intraday") == 1)
        #expect(BarPlanHorizon.defaultFor("scalper_session") == 1)
        #expect(BarPlanHorizon.defaultFor("pre_market") == 1)
        #expect(BarPlanHorizon.defaultFor("swing") == 5)
        #expect(BarPlanHorizon.defaultFor("positional") == 20)
    }

    @Test func expiryModeNeedsDTE() {
        #expect(BarPlanHorizon.days(for: .expiry, dte: 33) == 33)
        #expect(BarPlanHorizon.days(for: .expiry, dte: nil) == nil)
        #expect(BarPlanHorizon.mode(forDays: 1, dte: nil) == .today)
        #expect(BarPlanHorizon.mode(forDays: 3, dte: nil) == .threeSessions)
    }

    @Test func changingStyleReturnsNewDefault() {
        var days = BarPlanHorizon.defaultFor("intraday")
        #expect(days == 1)
        days = BarPlanHorizon.defaultFor("swing")
        #expect(days == 5)
        days = BarPlanHorizon.defaultFor("positional")
        #expect(days == 20)
        days = BarPlanHorizon.defaultFor("scalper_session")
        #expect(days == 1)
    }
}
