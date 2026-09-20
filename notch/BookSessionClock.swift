import Foundation

/// Per-book session hours. COM is 24/7 (spot lock). Kotak cash/NFO use NSE 09:15–15:30 IST.
enum BookSessionClock {
    enum Hours: Equatable, Sendable {
        case com247
        case nseCashFo
    }

    struct Presentation: Equatable, Sendable {
        let label: String
        let collapsedLabel: String
        let inSession: Bool
        let stale: Bool
    }

    static func hours(bookId: String?, brokerSlug: String?) -> Hours? {
        let book = bookId?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        if book.hasPrefix("binance-com") {
            return .com247
        }
        if book.hasPrefix("kotak") {
            return .nseCashFo
        }
        let slug = brokerSlug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        switch slug {
        case "binance_com", "binance":
            return .com247
        case "kotak_neo", "kotak":
            return .nseCashFo
        default:
            return nil
        }
    }

    static func isInSession(_ hours: Hours, at now: Date, calendar: Calendar = istCalendar()) -> Bool {
        switch hours {
        case .com247:
            return true
        case .nseCashFo:
            return isNseCashSession(at: now, calendar: calendar)
        }
    }

    /// NSE cash Normal market 09:15–15:30 IST, weekdays. Lock: kotak-nse-bse-cash.md (2026-08-22).
    static func isNseCashSession(at now: Date, calendar: Calendar = istCalendar()) -> Bool {
        let wd = calendar.component(.weekday, from: now)
        if wd == 1 || wd == 7 { return false }
        let mins = calendar.component(.hour, from: now) * 60 + calendar.component(.minute, from: now)
        return mins >= (9 * 60 + 15) && mins <= (15 * 60 + 30)
    }

    /// Pulse / positions / trades: any started book in session. COM started → always. Kotak-only → NSE hours.
    static func shouldPollMarketReads(startedSlugs: [String], at now: Date, calendar: Calendar = istCalendar()) -> Bool {
        let slugs = uniqueSlugs(startedSlugs)
        guard !slugs.isEmpty else { return false }
        for slug in slugs {
            guard let hours = hours(bookId: nil, brokerSlug: slug) else { continue }
            if isInSession(hours, at: now, calendar: calendar) {
                return true
            }
        }
        return false
    }

    static func presentation(
        bookId: String?,
        brokerSlug: String?,
        at now: Date,
        calendar: Calendar = istCalendar()
    ) -> Presentation {
        guard let hours = hours(bookId: bookId, brokerSlug: brokerSlug) else {
            return Presentation(label: "—", collapsedLabel: "—", inSession: false, stale: true)
        }
        let open = isInSession(hours, at: now, calendar: calendar)
        switch hours {
        case .com247:
            return Presentation(label: "COM 24/7", collapsedLabel: "24/7", inSession: true, stale: false)
        case .nseCashFo:
            if open {
                return Presentation(
                    label: "NSE 09:15–15:30",
                    collapsedLabel: "NSE",
                    inSession: true,
                    stale: false
                )
            }
            return Presentation(label: "NSE closed", collapsedLabel: "closed", inSession: false, stale: true)
        }
    }

    static func istCalendar() -> Calendar {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = TimeZone(identifier: "Asia/Kolkata") ?? .current
        return c
    }

    private static func uniqueSlugs(_ raw: [String]) -> [String] {
        var seen = Set<String>()
        var out: [String] = []
        for item in raw {
            let slug = item.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            guard !slug.isEmpty, !seen.contains(slug) else { continue }
            seen.insert(slug)
            out.append(slug)
        }
        return out
    }
}
