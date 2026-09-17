import Foundation
import Notch

/// T6′ Health panel — Station · Backend Box · Kill · Harness. Vendor rows hang under Box.
/// Never prints a vendor slug or a key.
struct HealthPanelChrome: Equatable {
    struct Row: Equatable, Identifiable {
        let id: String
        let title: String
        let status: String
        let what: String
        let why: String
        let error: String?
        let children: [Row]
    }

    let rows: [Row]

    var visibleText: String {
        func walk(_ rows: [Row]) -> [String] {
            rows.flatMap { row in
                [row.title, row.status, row.what, row.why, row.error ?? ""] + walk(row.children)
            }
        }
        return walk(rows).joined(separator: " ")
    }

    static func compose(
        agentHealthy: Bool,
        agentMessage: String?,
        vendorRows: [VendorFenceRow],
        killActive: Bool
    ) -> HealthPanelChrome {
        let stationStatus = agentHealthy ? "up" : "down"
        let stationWhy = agentHealthy ? "ok" : (agentMessage?.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty ?? "offline")
        let vendors = vendorRows.map { row in
            Row(
                id: row.adapterId,
                title: nounTitle(row.obtainNoun),
                status: row.status,
                what: nounWhat(row.obtainNoun),
                why: row.why,
                error: row.error,
                children: []
            )
        }
        let boxStatus = boxStatus(from: vendors.map(\.status))
        let killStatus = killActive ? "firing" : "idle"
        return HealthPanelChrome(rows: [
            Row(
                id: "station",
                title: "Station",
                status: stationStatus,
                what: "Loopback agent",
                why: stationWhy,
                error: nil,
                children: []
            ),
            Row(
                id: "backend_box",
                title: "Backend Box",
                status: boxStatus,
                what: "Declared vendor bindings",
                why: boxStatus,
                error: nil,
                children: vendors
            ),
            Row(
                id: "kill",
                title: "Kill",
                status: killStatus,
                what: "DNS teeth",
                why: killStatus,
                error: nil,
                children: []
            ),
            Row(
                id: "harness",
                title: "Harness",
                status: stationStatus,
                what: "Session overlay",
                why: stationWhy,
                error: nil,
                children: []
            ),
        ])
    }

    static func nounTitle(_ obtainNoun: String?) -> String {
        switch obtainNoun?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "history":
            return "History"
        case "amfi_nav":
            return "NAV"
        default:
            return "Market data"
        }
    }

    static func nounWhat(_ obtainNoun: String?) -> String {
        switch obtainNoun?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "history":
            return "India ohlcv. Kotak has none."
        case "amfi_nav":
            return "AMFI NAV (labs)"
        default:
            return "Eligible query — never a last"
        }
    }

    private static func boxStatus(from childStatuses: [String]) -> String {
        let normalized = childStatuses.map { $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() }
        if normalized.contains("exhausted") { return "exhausted" }
        if normalized.contains("unsupported") { return "unsupported" }
        if normalized.contains("down") { return "down" }
        if !normalized.isEmpty, normalized.allSatisfy({ $0 == "up" }) { return "up" }
        if normalized.isEmpty { return "unsupported" }
        return "unsupported"
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
