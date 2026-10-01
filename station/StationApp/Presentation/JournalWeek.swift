import Foundation

public enum JournalFacet: String, CaseIterable, Sendable {
    case all
    case matched
    case pending
    case unmatched
    case due
    case impulsive
}

public struct JournalCitedTrip: Equatable, Sendable {
    public let declarationId: String
    public let net: Double
    public let currency: String

    public init(declarationId: String, net: Double, currency: String) {
        self.declarationId = declarationId
        self.net = net
        self.currency = currency
    }
}

public struct JournalSnapshot: Equatable, Sendable {
    public let setupLabel: String?
    public let invalidationLine: String?
    public let invalidationKind: String?
    public let calmScale: Double?
    public let confidenceScale: Double?
    public let stopLoss: Double?
    public let target: Double?
    public let stance: String?

    public init(
        setupLabel: String?,
        invalidationLine: String?,
        invalidationKind: String?,
        calmScale: Double?,
        confidenceScale: Double?,
        stopLoss: Double?,
        target: Double?,
        stance: String? = nil
    ) {
        self.setupLabel = setupLabel
        self.invalidationLine = invalidationLine
        self.invalidationKind = invalidationKind
        self.calmScale = calmScale
        self.confidenceScale = confidenceScale
        self.stopLoss = stopLoss
        self.target = target
        self.stance = stance
    }
}

public struct JournalNotes: Equatable, Sendable {
    public let pre: String
    public let live: String
    public let post: String

    public init(pre: String, live: String, post: String) {
        self.pre = pre
        self.live = live
        self.post = post
    }

    public var postIsEmpty: Bool { post.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
}

public struct JournalFidelityDimensions: Equatable, Sendable {
    public let entry: Bool
    public let stop: Bool
    public let target: Bool
    public let size: Bool
    public let inv: Bool

    public static let honoured = JournalFidelityDimensions(
        entry: true, stop: true, target: true, size: true, inv: true
    )

    public init(entry: Bool, stop: Bool, target: Bool, size: Bool, inv: Bool) {
        self.entry = entry
        self.stop = stop
        self.target = target
        self.size = size
        self.inv = inv
    }
}

public struct JournalFidelity: Equatable, Sendable {
    public let score: Double?
    public let dimensions: JournalFidelityDimensions?

    public init(score: Double?, dimensions: JournalFidelityDimensions?) {
        self.score = score
        self.dimensions = dimensions
    }
}

public struct JournalAttachments: Equatable, Sendable {
    public let shots: Int
    public let voice: Bool

    public init(shots: Int, voice: Bool) {
        self.shots = shots
        self.voice = voice
    }
}

public struct JournalDeclarationCard: Equatable, Identifiable, Sendable {
    public let id: String
    public let status: String
    public let declarationKind: String
    public let symbol: String
    public let side: String
    public let quantity: Double
    public let quantityFilled: Double?
    public let localDate: String
    public let protectiveSlConsent: Bool
    public let snapshot: JournalSnapshot
    public let notes: JournalNotes
    public let fidelity: JournalFidelity
    public let attachments: JournalAttachments
    public let citedNet: Double?
    public let citedCurrency: String?
    public let n2DaySheet: JournalN2DaySheet?

    public init(
        id: String,
        status: String,
        declarationKind: String,
        symbol: String,
        side: String,
        quantity: Double,
        quantityFilled: Double?,
        localDate: String,
        protectiveSlConsent: Bool,
        snapshot: JournalSnapshot,
        notes: JournalNotes,
        fidelity: JournalFidelity,
        attachments: JournalAttachments,
        citedNet: Double?,
        citedCurrency: String?,
        n2DaySheet: JournalN2DaySheet? = nil
    ) {
        self.id = id
        self.status = status
        self.declarationKind = declarationKind
        self.symbol = symbol
        self.side = side
        self.quantity = quantity
        self.quantityFilled = quantityFilled
        self.localDate = localDate
        self.protectiveSlConsent = protectiveSlConsent
        self.snapshot = snapshot
        self.notes = notes
        self.fidelity = fidelity
        self.attachments = attachments
        self.citedNet = citedNet
        self.citedCurrency = citedCurrency
        self.n2DaySheet = n2DaySheet
    }

    public var isMatched: Bool { status == "matched" }
    public var isPending: Bool { status == "pending" }
    public var isUnmatched: Bool { status == "expired" || status == "cancelled" }
    public var isPostDue: Bool {
        guard isMatched else { return false }
        if let n2 = n2DaySheet, !n2.debrief.momentCEmpty { return false }
        if let n2 = n2DaySheet, n2.debrief.momentCEmpty { return true }
        return notes.postIsEmpty
    }
}

public struct JournalDaySheet: Equatable, Sendable {
    public let noteId: String
    public let emotion: String
    public let body: String
    public let markdownExportPath: String

    public init(noteId: String, emotion: String, body: String, markdownExportPath: String) {
        self.noteId = noteId
        self.emotion = emotion
        self.body = body
        self.markdownExportPath = markdownExportPath
    }
}

public struct JournalWeekDay: Equatable, Identifiable, Sendable {
    public var id: String { localDate }
    public let localDate: String
    public let sheet: JournalDaySheet?

    public init(localDate: String, sheet: JournalDaySheet?) {
        self.localDate = localDate
        self.sheet = sheet
    }
}

public struct JournalWeekPayload: Equatable, Sendable {
    public let timezone: String
    public let weekStart: String
    public let weekEnd: String
    public let items: [JournalDeclarationCard]
    public let days: [JournalWeekDay]

    public init(
        timezone: String,
        weekStart: String,
        weekEnd: String,
        items: [JournalDeclarationCard],
        days: [JournalWeekDay]
    ) {
        self.timezone = timezone
        self.weekStart = weekStart
        self.weekEnd = weekEnd
        self.items = items
        self.days = days
    }
}

public struct JournalImpulsiveRow: Equatable, Identifiable, Sendable {
    public var id: String { symbol }
    public let symbol: String
    public let qty: Double
    public let direction: String
    public let fidelity: JournalFidelity?
    public let planInNotch: Bool

    public init(symbol: String, qty: Double, direction: String) {
        self.symbol = symbol
        self.qty = qty
        self.direction = direction
        self.fidelity = nil
        self.planInNotch = true
    }
}

public struct JournalWeek: Equatable, Sendable {
    public let days: [JournalWeekDay]
    public let selectedDay: String?
    public let sheet: JournalDaySheet?
    public let declarations: [JournalDeclarationCard]
    public let impulsive: [JournalImpulsiveRow]
    public let sidebarDue: Bool
    /// Never a blended USD+INR total. DualNoBlend.
    public let weekTotalNet: Double?

    public init(
        days: [JournalWeekDay],
        selectedDay: String?,
        sheet: JournalDaySheet?,
        declarations: [JournalDeclarationCard],
        impulsive: [JournalImpulsiveRow],
        sidebarDue: Bool,
        weekTotalNet: Double?
    ) {
        self.days = days
        self.selectedDay = selectedDay
        self.sheet = sheet
        self.declarations = declarations
        self.impulsive = impulsive
        self.sidebarDue = sidebarDue
        self.weekTotalNet = weekTotalNet
    }

    public static func empty() -> JournalWeek {
        JournalWeek(
            days: [],
            selectedDay: nil,
            sheet: nil,
            declarations: [],
            impulsive: [],
            sidebarDue: false,
            weekTotalNet: nil
        )
    }

    public static func build(
        payload: JournalWeekPayload?,
        inventory: [DeskPosition],
        citedTrips: [JournalCitedTrip],
        selectedDay: String?,
        facet: JournalFacet,
        query: String
    ) -> JournalWeek {
        guard let payload else { return .empty() }

        let tripById = Dictionary(uniqueKeysWithValues: citedTrips.map { ($0.declarationId, $0) })
        let citedItems = payload.items.map { item -> JournalDeclarationCard in
            guard let trip = tripById[item.id] else { return item }
            return JournalDeclarationCard(
                id: item.id,
                status: item.status,
                declarationKind: item.declarationKind,
                symbol: item.symbol,
                side: item.side,
                quantity: item.quantity,
                quantityFilled: item.quantityFilled,
                localDate: item.localDate,
                protectiveSlConsent: item.protectiveSlConsent,
                snapshot: item.snapshot,
                notes: item.notes,
                fidelity: item.fidelity,
                attachments: item.attachments,
                citedNet: trip.net,
                citedCurrency: trip.currency
            )
        }

        let sheetByDate = Dictionary(uniqueKeysWithValues: payload.days.map { ($0.localDate, $0.sheet) })
        var dayDates = Set(payload.days.map(\.localDate))
        for item in citedItems { dayDates.insert(item.localDate) }
        let days = dayDates.sorted().map { date in
            JournalWeekDay(localDate: date, sheet: sheetByDate[date] ?? payload.days.first { $0.localDate == date }?.sheet)
        }

        let dayId = selectedDay ?? days.last?.localDate
        let sheet = days.first { $0.localDate == dayId }?.sheet
        let dayItems = citedItems.filter { $0.localDate == dayId }

        let coveredSymbols = TodayDetectJoin.coveredSymbols(from: citedItems)
        let impulsiveAll = inventory
            .filter { !coveredSymbols.contains($0.symbol.uppercased()) }
            .map { JournalImpulsiveRow(symbol: $0.symbol, qty: $0.qty, direction: $0.direction) }

        let sidebarDue = citedItems.contains { $0.isPostDue }

        let q = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let emotionHit = {
            guard !q.isEmpty, let emotion = sheet?.emotion, !emotion.isEmpty else { return false }
            return emotion.lowercased().contains(q)
        }()

        func matchesSearch(_ item: JournalDeclarationCard) -> Bool {
            if q.isEmpty { return true }
            if emotionHit { return true }
            let parts = [
                item.symbol,
                item.snapshot.setupLabel,
                item.snapshot.invalidationLine,
                item.snapshot.invalidationKind,
                item.status,
            ]
            return parts.compactMap { $0 }.joined(separator: " ").lowercased().contains(q)
        }

        let filteredDecls: [JournalDeclarationCard]
        let filteredImpulsive: [JournalImpulsiveRow]
        switch facet {
        case .impulsive:
            filteredDecls = []
            filteredImpulsive = impulsiveAll.filter { row in
                if q.isEmpty { return true }
                return [row.symbol, "impulsive", row.direction].joined(separator: " ").lowercased().contains(q)
            }
        case .all:
            filteredDecls = dayItems.filter(matchesSearch)
            filteredImpulsive = impulsiveAll.filter { row in
                if q.isEmpty { return true }
                return [row.symbol, "impulsive", row.direction].joined(separator: " ").lowercased().contains(q)
            }
        case .matched:
            filteredDecls = dayItems.filter { $0.isMatched && matchesSearch($0) }
            filteredImpulsive = []
        case .pending:
            filteredDecls = dayItems.filter { $0.isPending && matchesSearch($0) }
            filteredImpulsive = []
        case .unmatched:
            filteredDecls = dayItems.filter { $0.isUnmatched && matchesSearch($0) }
            filteredImpulsive = []
        case .due:
            filteredDecls = dayItems.filter { $0.isPostDue && matchesSearch($0) }
            filteredImpulsive = []
        }

        return JournalWeek(
            days: days,
            selectedDay: dayId,
            sheet: sheet,
            declarations: filteredDecls,
            impulsive: filteredImpulsive,
            sidebarDue: sidebarDue,
            weekTotalNet: nil
        )
    }
}
