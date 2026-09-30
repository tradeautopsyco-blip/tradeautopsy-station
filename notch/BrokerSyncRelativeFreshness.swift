import Foundation

/// Relative broker connection age — same buckets as Station Brokers `Last synced`.
enum BrokerSyncRelativeFreshness {
    static func format(epochMs: Int64?, now: Date = Date()) -> String? {
        guard let epochMs else { return nil }
        let synced = Date(timeIntervalSince1970: TimeInterval(epochMs) / 1000.0)
        let seconds = max(0, Int(now.timeIntervalSince(synced)))
        if seconds < 5 {
            return "just now"
        }
        if seconds < 60 {
            return "\(seconds)s ago"
        }
        let minutes = seconds / 60
        if minutes < 60 {
            return "\(minutes)m ago"
        }
        let hours = minutes / 60
        if hours < 48 {
            return "\(hours)h ago"
        }
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: synced)
    }

    static func formatShortAge(since date: Date?, now: Date = Date()) -> String? {
        guard let date else { return nil }
        let seconds = max(0, Int(now.timeIntervalSince(date).rounded(.down)))
        if seconds < 60 { return "\(seconds)s" }
        return "\(seconds / 60)m"
    }
}
