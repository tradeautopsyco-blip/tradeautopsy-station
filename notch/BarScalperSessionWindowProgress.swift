import Foundation

// MARK: - #4 — scalper session time-window progress (pure, testable)

enum BarScalperSessionWindowProgress {
    /// Elapsed fraction 0…1 of `[start, end]`. If `entryTimeISO` is nil, uses `fallbackDurationSeconds` before `end`.
    static func elapsedFraction(
        now: Date,
        windowEndsISO: String,
        entryTimeISO: String?,
        fallbackDurationSeconds: TimeInterval = 4 * 3600,
    ) -> Double? {
        guard let end = parseISO8601(windowEndsISO) else { return nil }
        let start: Date?
        if let raw = entryTimeISO, let e = parseISO8601(raw) {
            start = e
        } else {
            start = end.addingTimeInterval(-fallbackDurationSeconds)
        }
        guard let s = start, end > s else { return nil }
        let span = end.timeIntervalSince(s)
        guard span > 0 else { return nil }
        let t = now.timeIntervalSince(s) / span
        return min(1, max(0, t))
    }

    static func humanReadableRemaining(now: Date, windowEndsISO: String) -> String? {
        guard let end = parseISO8601(windowEndsISO) else { return nil }
        let secs = end.timeIntervalSince(now)
        guard secs > 0 else { return "0m" }
        let m = Int(secs / 60)
        let h = m / 60
        let mm = m % 60
        if h > 0 { return "\(h)h \(mm)m" }
        return "\(mm)m"
    }

    private static func parseISO8601(_ s: String) -> Date? {
        let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.isEmpty { return nil }
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let d = f.date(from: t) { return d }
        f.formatOptions = [.withInternetDateTime]
        return f.date(from: t)
    }
}
