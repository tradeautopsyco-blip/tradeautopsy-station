import Foundation
import Testing
@testable import Station

struct JournalWeekProjectionTests {
    private func card(
        id: String,
        status: String,
        symbol: String,
        localDate: String = "2026-09-11",
        setup: String? = "Pullback",
        invalidation: String? = "VWAP loss",
        post: String = "",
        emotion: String? = nil
    ) -> JournalDeclarationCard {
        JournalDeclarationCard(
            id: id,
            status: status,
            declarationKind: "intraday",
            symbol: symbol,
            side: "BUY",
            quantity: 50,
            quantityFilled: status == "matched" ? 50 : nil,
            localDate: localDate,
            protectiveSlConsent: true,
            snapshot: JournalSnapshot(
                setupLabel: setup,
                invalidationLine: invalidation,
                invalidationKind: "behaviour",
                calmScale: 2,
                confidenceScale: 4,
                stopLoss: 1260,
                target: 1320
            ),
            notes: JournalNotes(pre: "Declared.", live: "Intact.", post: post),
            fidelity: JournalFidelity(score: status == "matched" ? 80 : nil, dimensions: status == "matched" ? .honoured : nil),
            attachments: JournalAttachments(shots: 0, voice: false),
            citedNet: nil,
            citedCurrency: nil
        )
    }

    @Test func failClosedPayloadIsHonestEmptyNotInventedRows() {
        let week = JournalWeek.build(
            payload: nil,
            inventory: [DeskPosition(symbol: "RELIANCE", qty: 10, unrealizedPnL: nil, direction: "LONG")],
            citedTrips: [],
            selectedDay: nil,
            facet: .all,
            query: ""
        )
        #expect(week.declarations.isEmpty)
        #expect(week.impulsive.isEmpty)
        #expect(week.sidebarDue == false)
        #expect(week.days.isEmpty)
    }

    @Test func cancelledAndExpiredStayOnTheSheet() {
        let payload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: "2026-09-06T18:30:00.000Z",
            weekEnd: "2026-09-13T18:30:00.000Z",
            items: [
                card(id: "c", status: "cancelled", symbol: "NIFTY"),
                card(id: "e", status: "expired", symbol: "HDFCBANK"),
                card(id: "m", status: "matched", symbol: "RELIANCE", post: "Out."),
            ],
            days: []
        )
        let week = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "")
        #expect(week.declarations.map(\.id).sorted() == ["c", "e", "m"])
        let unmatched = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .unmatched, query: "")
        #expect(unmatched.declarations.map(\.id).sorted() == ["c", "e"])
    }

    @Test func emptyMomentCOnN2SetsDueEvenWhenPostFilled() {
        let n2 = JournalN2DaySheet(
            localDate: "2026-09-11",
            debrief: JournalN2Debrief(momentANote: "Gap", momentCNote: "", momentCContext: ""),
            conditionFires: [JournalN2ConditionFire(ruleId: "invalidation_price", firedAtMs: 1)],
            hasWorkingSnapshot: true
        )
        var c = card(id: "n2due", status: "matched", symbol: "INFY", post: "legacy post")
        c = JournalDeclarationCard(
            id: c.id,
            status: c.status,
            declarationKind: c.declarationKind,
            symbol: c.symbol,
            side: c.side,
            quantity: c.quantity,
            quantityFilled: c.quantityFilled,
            localDate: c.localDate,
            protectiveSlConsent: c.protectiveSlConsent,
            snapshot: c.snapshot,
            notes: c.notes,
            fidelity: c.fidelity,
            attachments: c.attachments,
            citedNet: c.citedNet,
            citedCurrency: c.citedCurrency,
            n2DaySheet: n2
        )
        #expect(c.isPostDue == true)
        #expect(JournalCardPaint.statusChips(c).contains("Due"))
    }

    @Test func emptyPostOnMatchedSetsSidebarDue() {
        let payload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: "2026-09-06T18:30:00.000Z",
            weekEnd: "2026-09-13T18:30:00.000Z",
            items: [
                card(id: "due", status: "matched", symbol: "RELIANCE", post: ""),
                card(id: "done", status: "matched", symbol: "HDFCBANK", post: "Target."),
            ],
            days: []
        )
        let all = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "")
        #expect(all.sidebarDue == true)
        #expect(all.declarations.map(\.id).sorted() == ["done", "due"])
        #expect(JournalFacet.allCases == [.all, .matched, .pending, .unmatched, .impulsive])
    }

    @Test func paintedCardKeepsSnapshotAndStripsProcessFidelity() {
        let painted = card(id: "due", status: "matched", symbol: "RELIANCE", post: "")
        let line = JournalCardPaint.snapLine(painted)
        #expect(line.contains("Pullback"))
        #expect(line.contains("SL 1260"))
        #expect(!line.lowercased().contains("fidelity"))
        #expect(JournalCardPaint.statusChips(painted) == ["Matched", "Due"])
        let labels = JournalCardPaint.drawerRows(painted).map(\.label)
        #expect(labels.contains("SL"))
        #expect(labels.contains("Target"))
        #expect(labels.contains("Setup"))
        #expect(labels.contains("Pre"))
        #expect(labels.contains("Live"))
        #expect(labels.contains("Post"))
        #expect(!labels.contains("Fidelity"))
    }

    @Test func reactiveStanceIsAJournalCardOnThatId() {
        let payload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: "2026-09-06T18:30:00.000Z",
            weekEnd: "2026-09-13T18:30:00.000Z",
            items: [
                JournalDeclarationCard(
                    id: "imp-1",
                    status: "matched",
                    declarationKind: "intraday",
                    symbol: "BANKNIFTY",
                    side: "BUY",
                    quantity: 15,
                    quantityFilled: 15,
                    localDate: "2026-09-11",
                    protectiveSlConsent: false,
                    snapshot: JournalSnapshot(
                        setupLabel: nil,
                        invalidationLine: nil,
                        invalidationKind: nil,
                        calmScale: nil,
                        confidenceScale: nil,
                        stopLoss: nil,
                        target: nil,
                        stance: "reactive"
                    ),
                    notes: JournalNotes(pre: "", live: "", post: "Filled without a plan."),
                    fidelity: JournalFidelity(score: nil, dimensions: nil),
                    attachments: JournalAttachments(shots: 0, voice: false),
                    citedNet: nil,
                    citedCurrency: nil
                )
            ],
            days: []
        )
        let week = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "")
        #expect(week.declarations.map(\.id) == ["imp-1"])
        #expect(JournalCardPaint.statusChips(week.declarations[0]).contains("Reactive"))
        #expect(week.declarations[0].notes.post == "Filled without a plan.")
    }

    @Test func impulsiveIsInventoryWithoutDeclarationIdAndHasNoFidelity() {
        let payload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: "2026-09-06T18:30:00.000Z",
            weekEnd: "2026-09-13T18:30:00.000Z",
            items: [card(id: "d1", status: "pending", symbol: "RELIANCE")],
            days: []
        )
        let inventory = [
            DeskPosition(symbol: "RELIANCE", qty: 50, unrealizedPnL: nil, direction: "LONG"),
            DeskPosition(symbol: "BANKNIFTY", qty: 15, unrealizedPnL: nil, direction: "LONG"),
        ]
        let week = JournalWeek.build(payload: payload, inventory: inventory, citedTrips: [], selectedDay: "2026-09-11", facet: .impulsive, query: "")
        #expect(week.impulsive.map(\.symbol) == ["BANKNIFTY"])
        #expect(week.impulsive.first?.fidelity == nil)
        #expect(week.impulsive.first?.planInNotch == true)
        #expect(week.declarations.isEmpty)
    }

    @Test func searchCoversSymbolSetupEmotionInvalidation() {
        let payload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: "2026-09-06T18:30:00.000Z",
            weekEnd: "2026-09-13T18:30:00.000Z",
            items: [
                card(id: "a", status: "matched", symbol: "RELIANCE", setup: "Pullback", invalidation: "Gap through 1150", post: "x"),
                card(id: "b", status: "pending", symbol: "HDFCBANK", setup: "Breakout", invalidation: "Time cut"),
            ],
            days: [
                JournalWeekDay(localDate: "2026-09-11", sheet: JournalDaySheet(noteId: "n1", emotion: "Tense", body: "Cap size.", markdownExportPath: "/md")),
            ]
        )
        let bySymbol = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "hdfc")
        #expect(bySymbol.declarations.map(\.id) == ["b"])
        let bySetup = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "pullback")
        #expect(bySetup.declarations.map(\.id) == ["a"])
        let byInv = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "1150")
        #expect(byInv.declarations.map(\.id) == ["a"])
        let byEmotion = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "tense")
        #expect(byEmotion.declarations.map(\.id) == ["a", "b"])
    }

    @Test func matchedNetIsCitedStationTripOnlyNeverBlended() {
        let payload = JournalWeekPayload(
            timezone: "Asia/Kolkata",
            weekStart: "2026-09-06T18:30:00.000Z",
            weekEnd: "2026-09-13T18:30:00.000Z",
            items: [
                card(id: "inr", status: "matched", symbol: "RELIANCE", post: "Out."),
                card(id: "usd", status: "matched", symbol: "BTCUSDT", post: "Out."),
            ],
            days: []
        )
        let trips = [
            JournalCitedTrip(declarationId: "inr", net: 1100, currency: "INR"),
            JournalCitedTrip(declarationId: "usd", net: 42, currency: "USD"),
        ]
        let week = JournalWeek.build(payload: payload, inventory: [], citedTrips: trips, selectedDay: "2026-09-11", facet: .all, query: "")
        #expect(week.declarations.first { $0.id == "inr" }?.citedNet == 1100)
        #expect(week.declarations.first { $0.id == "inr" }?.citedCurrency == "INR")
        #expect(week.declarations.first { $0.id == "usd" }?.citedNet == 42)
        #expect(week.declarations.first { $0.id == "usd" }?.citedCurrency == "USD")
        #expect(week.weekTotalNet == nil)
        let none = JournalWeek.build(payload: payload, inventory: [], citedTrips: [], selectedDay: "2026-09-11", facet: .all, query: "")
        #expect(none.declarations.allSatisfy { $0.citedNet == nil })
    }

    @Test func weekWireDecodeFailsClosedOnGarbageAndMapsSnapshot() {
        #expect(JournalWeekWire.decode(Data("not-json".utf8)) == nil)
        let json = """
        {"ok":true,"scope":"week","timezone":"Asia/Kolkata","week_start":"2026-09-06T18:30:00.000Z","week_end":"2026-09-13T18:30:00.000Z","items":[{"id":"d1","status":"cancelled","declaration_kind":"intraday","symbol":"HDFCBANK","side":"BUY","quantity":20,"quantity_filled":null,"local_date":"2026-09-11","protective_sl_consent":true,"snapshot":{"setup_label":"Pullback","invalidation_line":"No fill by 14:30","invalidation_kind":"time","calm_scale":2,"confidence_scale":4,"stop_loss":1640,"target":1710},"notes":{"pre":"Declared.","live":"","post":""},"fidelity":{"score":null,"dimensions":null},"attachments":{"shots":0,"voice":false}}],"days":[]}
        """.data(using: .utf8)!
        let payload = JournalWeekWire.decode(json)
        #expect(payload?.items.count == 1)
        #expect(payload?.items.first?.snapshot.setupLabel == "Pullback")
        #expect(payload?.items.first?.citedNet == nil)
    }
}
