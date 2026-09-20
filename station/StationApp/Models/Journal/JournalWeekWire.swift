import Foundation

/// Wire DTO for `GET /api/daemon/bar/declarations?scope=week` (Console snake_case).
enum JournalWeekWire {
    struct Envelope: Decodable {
        var timezone: String?
        var weekStart: String?
        var weekEnd: String?
        var items: [Item]?
        var days: [Day]?

        enum CodingKeys: String, CodingKey {
            case timezone, items, days
            case weekStart = "week_start"
            case weekEnd = "week_end"
        }
    }

    struct Item: Decodable {
        var id: String
        var status: String
        var declarationKind: String?
        var symbol: String
        var side: String?
        var quantity: Double?
        var quantityFilled: Double?
        var localDate: String?
        var protectiveSlConsent: Bool?
        var snapshot: Snapshot?
        var notes: Notes?
        var fidelity: Fidelity?
        var attachments: Attachments?

        enum CodingKeys: String, CodingKey {
            case id, status, symbol, side, quantity, snapshot, notes, fidelity, attachments
            case declarationKind = "declaration_kind"
            case quantityFilled = "quantity_filled"
            case localDate = "local_date"
            case protectiveSlConsent = "protective_sl_consent"
        }
    }

    struct Snapshot: Decodable {
        var setupLabel: String?
        var invalidationLine: String?
        var invalidationKind: String?
        var calmScale: Double?
        var confidenceScale: Double?
        var stopLoss: Double?
        var target: Double?

        var intent: String?
        var stance: String?
        var invalidationPrice: Double?
        var product: String?

        enum CodingKeys: String, CodingKey {
            case target, intent, stance, product
            case setupLabel = "setup_label"
            case invalidationLine = "invalidation_line"
            case invalidationKind = "invalidation_kind"
            case invalidationPrice = "invalidation_price"
            case calmScale = "calm_scale"
            case confidenceScale = "confidence_scale"
            case stopLoss = "stop_loss"
        }
    }

    struct Notes: Decodable {
        var pre: String?
        var live: String?
        var post: String?
    }

    struct Fidelity: Decodable {
        var score: Double?
        var dimensions: Dimensions?
    }

    struct Dimensions: Decodable {
        var entry: Bool?
        var stop: Bool?
        var target: Bool?
        var size: Bool?
        var inv: Bool?
    }

    struct Attachments: Decodable {
        var shots: Int?
        var voice: Bool?
    }

    struct Day: Decodable {
        var localDate: String?
        var sheet: Sheet?

        enum CodingKeys: String, CodingKey {
            case sheet
            case localDate = "local_date"
        }
    }

    struct Sheet: Decodable {
        var noteId: String?
        var emotion: String?
        var body: String?
        var markdownExportPath: String?

        enum CodingKeys: String, CodingKey {
            case emotion, body
            case noteId = "note_id"
            case markdownExportPath = "markdown_export_path"
        }
    }

    static func decode(_ data: Data) -> JournalWeekPayload? {
        guard let env = try? JSONDecoder().decode(Envelope.self, from: data) else { return nil }
        let items = (env.items ?? []).map(mapItem)
        let days = (env.days ?? []).compactMap { day -> JournalWeekDay? in
            guard let date = day.localDate, !date.isEmpty else { return nil }
            let sheet: JournalDaySheet? = {
                guard let s = day.sheet, let noteId = s.noteId, !noteId.isEmpty else { return nil }
                return JournalDaySheet(
                    noteId: noteId,
                    emotion: s.emotion ?? "",
                    body: s.body ?? "",
                    markdownExportPath: s.markdownExportPath ?? "/api/journal/notebook/export/markdown?noteId=\(noteId)"
                )
            }()
            return JournalWeekDay(localDate: date, sheet: sheet)
        }
        return JournalWeekPayload(
            timezone: env.timezone ?? "Asia/Kolkata",
            weekStart: env.weekStart ?? "",
            weekEnd: env.weekEnd ?? "",
            items: items,
            days: days
        )
    }

    private static func mapItem(_ item: Item) -> JournalDeclarationCard {
        let snap = item.snapshot
        let dims = item.fidelity?.dimensions
        return JournalDeclarationCard(
            id: item.id,
            status: item.status,
            declarationKind: item.declarationKind ?? "",
            symbol: item.symbol,
            side: item.side ?? "",
            quantity: item.quantity ?? 0,
            quantityFilled: item.quantityFilled,
            localDate: item.localDate ?? "",
            protectiveSlConsent: item.protectiveSlConsent ?? false,
            snapshot: JournalSnapshot(
                setupLabel: snap?.setupLabel,
                invalidationLine: snap?.invalidationLine,
                invalidationKind: snap?.invalidationKind,
                calmScale: snap?.calmScale,
                confidenceScale: snap?.confidenceScale,
                stopLoss: snap?.stopLoss,
                target: snap?.target,
                stance: snap?.stance
            ),
            notes: JournalNotes(
                pre: item.notes?.pre ?? "",
                live: item.notes?.live ?? "",
                post: item.notes?.post ?? ""
            ),
            fidelity: JournalFidelity(
                score: item.fidelity?.score,
                dimensions: dims.map {
                    JournalFidelityDimensions(
                        entry: $0.entry ?? false,
                        stop: $0.stop ?? false,
                        target: $0.target ?? false,
                        size: $0.size ?? false,
                        inv: $0.inv ?? false
                    )
                }
            ),
            attachments: JournalAttachments(
                shots: item.attachments?.shots ?? 0,
                voice: item.attachments?.voice ?? false
            ),
            citedNet: nil,
            citedCurrency: nil
        )
    }
}
