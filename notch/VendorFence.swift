import Foundation

/// Harness fence row — Health of a vendor binding, never a last price.
public struct VendorFenceRow: Equatable, Sendable, Identifiable {
    public var id: String { adapterId }
    public let adapterId: String
    public let status: String
    public let what: String
    public let why: String
    public let error: String?
    public let obtainNoun: String?

    public init(
        adapterId: String,
        status: String,
        what: String,
        why: String,
        error: String? = nil,
        obtainNoun: String? = nil
    ) {
        self.adapterId = adapterId
        self.status = status
        self.what = what
        self.why = why
        self.error = error
        self.obtainNoun = obtainNoun
    }

    /// `licensed_history · up — licensed_history India ohlcv (Kotak has none)`
    public var fenceLine: String {
        var line = "\(adapterId) · \(status) — \(what)"
        if let error, !error.isEmpty {
            line += " (\(error))"
        }
        return line
    }
}

public enum VendorFence {
    public static func rows(fromHealthJSON json: [String: Any]) -> [VendorFenceRow] {
        guard let vendors = json["vendors"] as? [[String: Any]] else { return [] }
        return vendors.compactMap { row in
            let adapter = (row["adapter_id"] as? String)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            guard !adapter.isEmpty else { return nil }
            if adapter == "kotak_neo" || adapter == "binance_com" {
                return nil
            }
            let status = (row["status"] as? String) ?? "unsupported"
            let what = (row["what"] as? String) ?? adapter
            let why = (row["why"] as? String) ?? status
            let error = row["error"] as? String
            let noun = row["obtain_noun"] as? String
            return VendorFenceRow(
                adapterId: adapter,
                status: status,
                what: what,
                why: why,
                error: error,
                obtainNoun: noun
            )
        }
    }
}
