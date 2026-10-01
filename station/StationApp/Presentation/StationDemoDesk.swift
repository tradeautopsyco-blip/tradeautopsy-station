import Foundation

/// Station-only first-pair Demo fixture. Not a book. Not Console. No NIFTY / FO.
public enum StationDemoDesk {
    public struct Fixture: Equatable {
        public let payload: TodayAgentPayload
        public let positions: [DeskPosition]
        public let journalPayload: JournalWeekPayload
        public let journalTripInventoryRows: [JournalClosedTripInventoryRow]
    }

    public static func istCalendar() -> Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        return calendar
    }

    public static func build(
        now: Date = Date(),
        quoteCurrency: String? = "INR",
        calendar: Calendar = StationDemoDesk.istCalendar()
    ) -> Fixture {
        let localDate = localDayString(now, calendar: calendar)
        let yesterday = calendar.date(byAdding: .day, value: -1, to: now) ?? now
        let firstFilledAt = calendar.date(
            bySettingHour: 15,
            minute: 20,
            second: 0,
            of: yesterday
        ) ?? yesterday

        let trades = [
            trade(closedAt: iso(calendar, now, hour: 9, minute: 41), net: -800),
            trade(closedAt: iso(calendar, now, hour: 10, minute: 12), net: -1_000),
            trade(closedAt: iso(calendar, now, hour: 11, minute: 5), net: -600),
        ]
        let payload = TodayAgentPayload(
            localDate: localDate,
            performanceBasisNotTax: true,
            degradedReason: nil,
            learningBaseline: false,
            hero: TodayHeroPayload(
                pnlTodayUsd: -2_400,
                tradesToday: 3,
                winRate: 0.33,
                winsToday: 1,
                lossesToday: 2
            ),
            topSignals: [],
            trades: trades,
            openPositionCount: 1,
            brokerSlug: quoteCurrency == nil ? nil : "kotak_neo",
            quoteCurrency: quoteCurrency,
            calcProfileId: quoteCurrency == nil ? nil : DeskMoneyFormatting.calcProfileId(forBrokerSlug: "kotak_neo")
        )
        let positions = [
            DeskPosition(
                symbol: "RELIANCE",
                qty: 50,
                unrealizedPnL: nil,
                direction: "LONG",
                firstFilledAt: firstFilledAt
            ),
        ]
        let matchedDue = JournalDeclarationCard(
            id: "demo-reliance-matched",
            status: "matched",
            declarationKind: "intraday",
            symbol: "RELIANCE",
            side: "BUY",
            quantity: 50,
            quantityFilled: 50,
            localDate: localDate,
            protectiveSlConsent: true,
            snapshot: JournalSnapshot(
                setupLabel: "Pullback",
                invalidationLine: "VWAP loss",
                invalidationKind: "behaviour",
                calmScale: 2,
                confidenceScale: 4,
                stopLoss: 1_260,
                target: 1_320
            ),
            notes: JournalNotes(pre: "Declared.", live: "Closed.", post: ""),
            fidelity: JournalFidelity(score: 80, dimensions: .honoured),
            attachments: JournalAttachments(shots: 0, voice: false),
            citedNet: nil,
            citedCurrency: nil
        )
        let pending = JournalDeclarationCard(
            id: "demo-reliance-pending",
            status: "pending",
            declarationKind: "intraday",
            symbol: "RELIANCE",
            side: "BUY",
            quantity: 50,
            quantityFilled: nil,
            localDate: localDate,
            protectiveSlConsent: true,
            snapshot: JournalSnapshot(
                setupLabel: "Pullback",
                invalidationLine: "VWAP loss",
                invalidationKind: "behaviour",
                calmScale: 2,
                confidenceScale: 4,
                stopLoss: 1_260,
                target: 1_320
            ),
            notes: JournalNotes(pre: "", live: "", post: ""),
            fidelity: JournalFidelity(score: nil, dimensions: nil),
            attachments: JournalAttachments(shots: 0, voice: false),
            citedNet: nil,
            citedCurrency: nil
        )
        let journalPayload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: localDate,
            weekEnd: localDate,
            items: [matchedDue, pending],
            days: [JournalWeekDay(localDate: localDate, sheet: nil)]
        )
        let inventoryRows = [
            JournalClosedTripInventoryRow(
                declarationId: "demo-reliance-matched",
                net: -800,
                currency: quoteCurrency ?? "INR"
            ),
        ]
        return Fixture(
            payload: payload,
            positions: positions,
            journalPayload: journalPayload,
            journalTripInventoryRows: inventoryRows
        )
    }

    private static func trade(closedAt: String, net: Double) -> TodayTradeRowPayload {
        TodayTradeRowPayload(
            closedAt: closedAt,
            symbol: "RELIANCE",
            avgEntry: 1_280,
            avgExit: 1_265,
            qty: 50,
            netPnlUsd: net,
            primaryFlag: "CNC",
            flagSeverity: "clean",
            dataQualityFlags: []
        )
    }

    private static func iso(_ calendar: Calendar, _ day: Date, hour: Int, minute: Int) -> String {
        let date = calendar.date(bySettingHour: hour, minute: minute, second: 0, of: day) ?? day
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        return formatter.string(from: date)
    }

    private static func localDayString(_ date: Date, calendar: Calendar) -> String {
        let parts = calendar.dateComponents([.year, .month, .day], from: date)
        guard let year = parts.year, let month = parts.month, let day = parts.day else {
            return ""
        }
        return String(format: "%04d-%02d-%02d", year, month, day)
    }
}
