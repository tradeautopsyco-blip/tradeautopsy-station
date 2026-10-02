import Foundation
import Testing
@testable import Station

struct StationDemoDeskTests {
    private func istCalendar() -> Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        return calendar
    }

    private func noonIST(_ year: Int, _ month: Int, _ day: Int) -> Date {
        istCalendar().date(from: DateComponents(year: year, month: month, day: day, hour: 12))!
    }

    @Test func kotakFixtureIsFirstPairRelianceNeverNifty() {
        let now = noonIST(2026, 9, 12)
        let fixture = StationDemoDesk.build(now: now, quoteCurrency: "INR", calendar: istCalendar())
        #expect(fixture.payload.brokerSlug == "kotak_neo")
        #expect(fixture.payload.quoteCurrency == "INR")
        #expect(fixture.payload.hero.pnlTodayUsd == -2_400)
        #expect(fixture.payload.hero.tradesToday == 3)
        #expect(fixture.payload.trades.count == 3)
        #expect(fixture.payload.trades.allSatisfy { $0.symbol == "RELIANCE" })
        #expect(fixture.positions.count == 1)
        #expect(fixture.positions.first?.symbol == "RELIANCE")
        #expect(fixture.journalPayload.items.count == 2)
        #expect(fixture.journalPayload.items.first?.symbol == "RELIANCE")
        #expect(fixture.journalPayload.items.map(\.status) == ["matched", "pending"])
        let allSymbols = fixture.payload.trades.map(\.symbol) + fixture.positions.map(\.symbol) + fixture.journalPayload.items.map(\.symbol)
        #expect(allSymbols.allSatisfy { $0 == "RELIANCE" || $0 == "BTCUSDT" })
        #expect(!allSymbols.contains(where: { $0.contains("NIFTY") || $0.contains("FUT") }))
    }

    @Test func openRowIsOvernightFromYesterdayFill() {
        let now = noonIST(2026, 9, 12)
        let fixture = StationDemoDesk.build(now: now, quoteCurrency: "INR", calendar: istCalendar())
        let desk = TodayDeskPresentation.build(
            payload: fixture.payload,
            agentHealthy: true,
            positions: fixture.positions,
            now: now,
            calendar: istCalendar()
        )
        #expect(desk.screen.openRows.first?.behaviorText == "Overnight")
        #expect(fixture.positions.first?.firstFilledAt != nil)
    }

    @Test func pendingRelianceCoversDetectJoin() {
        let now = noonIST(2026, 9, 12)
        let fixture = StationDemoDesk.build(now: now, quoteCurrency: "INR", calendar: istCalendar())
        let input = TodayDetectJoin.input(
            position: fixture.positions.first,
            declarations: fixture.journalPayload.items,
            quoteCurrency: fixture.payload.deskQuoteCurrency
        )
        #expect(input?.planStop == 1_260)
        #expect(input?.entry == nil)
        #expect(input?.tradeCurrency == "INR")
    }

    @Test func missingQuotePathDualNoBlendDashesRemaining() {
        let now = noonIST(2026, 9, 12)
        let fixture = StationDemoDesk.build(now: now, quoteCurrency: nil, calendar: istCalendar())
        let desk = TodayDeskPresentation.build(
            payload: fixture.payload,
            agentHealthy: true,
            positions: fixture.positions,
            dailyFloor: 12_000,
            now: now,
            calendar: istCalendar()
        )
        #expect(fixture.payload.quoteCurrency == nil)
        #expect(desk.remainingAmount == nil)
        #expect(desk.remainingText == TodayScreenPresentation.emDash)
        #expect(!desk.remainingText.contains("INR"))
    }

    @Test func missingFloorStaysDashAndDoesNotInventTwelveThousand() {
        let now = noonIST(2026, 9, 12)
        let fixture = StationDemoDesk.build(now: now, quoteCurrency: "INR", calendar: istCalendar())
        let desk = TodayDeskPresentation.build(
            payload: fixture.payload,
            agentHealthy: true,
            positions: fixture.positions,
            dailyFloor: nil,
            now: now,
            calendar: istCalendar()
        )
        #expect(desk.remainingAmount == nil)
        #expect(desk.remainingText == TodayScreenPresentation.emDash)
        #expect(!desk.remainingText.contains("12,000"))
        #expect(!desk.remainingText.contains("12000"))
        #expect(desk.floorLine == nil)
    }
}
