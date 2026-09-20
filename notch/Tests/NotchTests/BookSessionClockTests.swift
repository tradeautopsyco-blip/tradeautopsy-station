import Foundation
import Testing
@testable import Notch

struct BookSessionClockTests {
    private var ist: Calendar { BookSessionClock.istCalendar() }

    private func istDate(year: Int, month: Int, day: Int, hour: Int, minute: Int = 0) -> Date {
        var c = DateComponents()
        c.year = year
        c.month = month
        c.day = day
        c.hour = hour
        c.minute = minute
        return ist.date(from: c)!
    }

    @Test func spotAndComBooksAre247() {
        #expect(BookSessionClock.hours(bookId: "binance-com-spot", brokerSlug: nil) == .com247)
        #expect(BookSessionClock.hours(bookId: "binance-com-options", brokerSlug: nil) == .com247)
        #expect(BookSessionClock.hours(bookId: nil, brokerSlug: "binance_com") == .com247)
    }

    @Test func kotakCashAndNfoUseNseHours() {
        #expect(BookSessionClock.hours(bookId: "kotak-nse-bse-cash", brokerSlug: nil) == .nseCashFo)
        #expect(BookSessionClock.hours(bookId: "kotak-nse-nfo", brokerSlug: nil) == .nseCashFo)
        #expect(BookSessionClock.hours(bookId: nil, brokerSlug: "kotak_neo") == .nseCashFo)
    }

    @Test func sundayComPollsKotakDoesNot() {
        // 2026-09-20 is Sunday.
        let sunday = istDate(year: 2026, month: 9, day: 20, hour: 12)
        #expect(BookSessionClock.isNseCashSession(at: sunday, calendar: ist) == false)
        #expect(BookSessionClock.shouldPollMarketReads(startedSlugs: ["binance_com"], at: sunday, calendar: ist))
        #expect(!BookSessionClock.shouldPollMarketReads(startedSlugs: ["kotak_neo"], at: sunday, calendar: ist))
        #expect(BookSessionClock.shouldPollMarketReads(
            startedSlugs: ["binance_com", "kotak_neo"],
            at: sunday,
            calendar: ist
        ))
        #expect(!BookSessionClock.shouldPollMarketReads(startedSlugs: [], at: sunday, calendar: ist))
    }

    @Test func weekdayNseOpenPollsKotak() {
        // 2026-09-21 is Monday.
        let open = istDate(year: 2026, month: 9, day: 21, hour: 10, minute: 0)
        let before = istDate(year: 2026, month: 9, day: 21, hour: 9, minute: 14)
        #expect(BookSessionClock.isNseCashSession(at: open, calendar: ist))
        #expect(!BookSessionClock.isNseCashSession(at: before, calendar: ist))
        #expect(BookSessionClock.shouldPollMarketReads(startedSlugs: ["kotak_neo"], at: open, calendar: ist))
        #expect(!BookSessionClock.shouldPollMarketReads(startedSlugs: ["kotak_neo"], at: before, calendar: ist))
    }

    @Test func focusedClockLabels() {
        let sunday = istDate(year: 2026, month: 9, day: 20, hour: 12)
        let com = BookSessionClock.presentation(bookId: "binance-com-spot", brokerSlug: "binance_com", at: sunday, calendar: ist)
        #expect(com.label == "COM 24/7")
        #expect(com.stale == false)
        let nse = BookSessionClock.presentation(bookId: "kotak-nse-bse-cash", brokerSlug: "kotak_neo", at: sunday, calendar: ist)
        #expect(nse.label == "NSE closed")
        #expect(nse.stale)
    }
}
