import Foundation

struct UnpostedCaptureRecord: Codable, Equatable, Identifiable {
    var id: UUID
    var createdAt: Date
    var updatedAt: Date
    var filename: String
    var contentType: String
    var byteSize: Int
    var caption: String
    var capturePhase: String
    var lastError: String?
    var consolePendingId: String?
    var idempotencyKey: String?
}

/// Local-only unposted screenshots. Cap: 20 items or 7 days (oldest dropped).
final class UnpostedCaptureStore {
    static let maxItems = 20
    static let maxAge: TimeInterval = 7 * 24 * 60 * 60

    private let directory: URL
    private let indexURL: URL
    private let fileManager: FileManager
    private let now: () -> Date
    private var records: [UnpostedCaptureRecord] = []

    init(
        directory: URL,
        fileManager: FileManager = .default,
        now: @escaping () -> Date = Date.init
    ) {
        self.directory = directory
        self.indexURL = directory.appendingPathComponent("index.json")
        self.fileManager = fileManager
        self.now = now
        try? fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        load()
        _ = sweep()
    }

    func all() -> [UnpostedCaptureRecord] {
        records.sorted { $0.createdAt > $1.createdAt }
    }

    func fileURL(for record: UnpostedCaptureRecord) -> URL {
        directory.appendingPathComponent(record.filename)
    }

    @discardableResult
    func insert(
        imageData: Data,
        contentType: String,
        caption: String,
        capturePhase: String
    ) throws -> UnpostedCaptureRecord {
        let ext = contentType == "image/jpeg" ? "jpg" : "png"
        let id = UUID()
        let filename = "\(id.uuidString).\(ext)"
        let dest = directory.appendingPathComponent(filename)
        try imageData.write(to: dest, options: .atomic)
        let ts = now()
        let rec = UnpostedCaptureRecord(
            id: id,
            createdAt: ts,
            updatedAt: ts,
            filename: filename,
            contentType: contentType,
            byteSize: imageData.count,
            caption: caption,
            capturePhase: capturePhase,
            lastError: nil,
            consolePendingId: nil,
            idempotencyKey: nil
        )
        records.append(rec)
        let dropped = sweep()
        persist()
        _ = dropped
        return rec
    }

    func update(_ record: UnpostedCaptureRecord) {
        guard let idx = records.firstIndex(where: { $0.id == record.id }) else { return }
        records[idx] = record
        persist()
    }

    func delete(id: UUID) {
        if let rec = records.first(where: { $0.id == id }) {
            try? fileManager.removeItem(at: fileURL(for: rec))
        }
        records.removeAll { $0.id == id }
        persist()
    }

    /// Drops expired / overflow items. Returns how many were removed.
    @discardableResult
    func sweep() -> Int {
        let cutoff = now().addingTimeInterval(-Self.maxAge)
        var dropped = 0
        let expired = records.filter { $0.createdAt < cutoff }
        for rec in expired {
            try? fileManager.removeItem(at: fileURL(for: rec))
            dropped += 1
        }
        records.removeAll { $0.createdAt < cutoff }
        if records.count > Self.maxItems {
            let sorted = records.sorted { $0.createdAt < $1.createdAt }
            let extra = records.count - Self.maxItems
            for rec in sorted.prefix(extra) {
                try? fileManager.removeItem(at: fileURL(for: rec))
                records.removeAll { $0.id == rec.id }
                dropped += 1
            }
        }
        if dropped > 0 {
            persist()
        }
        return dropped
    }

    private func load() {
        guard let data = try? Data(contentsOf: indexURL) else { return }
        let dec = JSONDecoder()
        dec.dateDecodingStrategy = .iso8601
        records = (try? dec.decode([UnpostedCaptureRecord].self, from: data)) ?? []
    }

    private func persist() {
        let enc = JSONEncoder()
        enc.dateEncodingStrategy = .iso8601
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? enc.encode(records) else { return }
        try? data.write(to: indexURL, options: .atomic)
    }

    static func defaultDirectory() -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        return base.appendingPathComponent("TradeAutopsy/unposted-captures", isDirectory: true)
    }
}
