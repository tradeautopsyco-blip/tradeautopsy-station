import Foundation

/// Harness top chrome A — books cluster + Status. Never prints a vendor slug or fence line.
struct HarnessTopChrome: Equatable {
    enum StatusKind: Equatable {
        case quiet
        case warn
        case bad
    }

    struct Book: Equatable, Identifiable {
        var id: String { slug }
        let slug: String
        let displayName: String
        let stateLabel: String
        let inCluster: Bool
    }

    struct Hole: Equatable {
        let key: String
        let value: String
        let kind: StatusKind
        let retryInstruments: Bool
    }

    struct DataRow: Equatable {
        let nounLabel: String
        let eligible: Bool
        var statusLabel: String { eligible ? "Eligible" : "Not eligible" }
    }

    let books: [Book]
    let holes: [Hole]
    let dataRows: [DataRow]
    let statusLabel: String
    let statusKind: StatusKind
    let showsRetryInstruments: Bool

    var clusterDisplayNames: [String] {
        books.filter(\.inCluster).map(\.displayName)
    }

    /// User-visible chrome copy. Must never contain adapter ids or `fenceLine`.
    var visibleChromeText: String {
        var parts: [String] = []
        parts.append(contentsOf: books.map(\.displayName))
        parts.append(contentsOf: books.map(\.stateLabel))
        parts.append(statusLabel)
        parts.append(contentsOf: holes.map { "\($0.key) \($0.value)" })
        parts.append(contentsOf: dataRows.map { "\($0.nounLabel) \($0.statusLabel)" })
        return parts.joined(separator: " ")
    }

    @MainActor
    static func compose(
        activeSlug: String?,
        brokerSyncClass: String,
        venuePostureBySlug: [String: VenuePosture],
        quoteStatus: String,
        instrumentsStatus: String,
        accountStatus: String,
        vendorFenceRows: [VendorFenceRow]
    ) -> HarnessTopChrome {
        let books = composeBooks(
            activeSlug: activeSlug,
            brokerSyncClass: brokerSyncClass,
            venuePostureBySlug: venuePostureBySlug
        )
        let dataRows = vendorFenceRows.map { row in
            DataRow(nounLabel: dataNounLabel(obtainNoun: row.obtainNoun), eligible: isEligible(row))
        }
        var holes: [Hole] = []
        if let quoteHole = deskHole(key: "Quote", status: quoteStatus) {
            holes.append(quoteHole)
        }
        let retry = DeskCapabilityChrome.showsRetryInstruments(status: instrumentsStatus)
        if let instHole = deskHole(key: "Instruments", status: instrumentsStatus, retryInstruments: retry) {
            holes.append(instHole)
        }
        if let accountHole = deskHole(key: "Account", status: accountStatus) {
            holes.append(accountHole)
        }
        for row in dataRows where !row.eligible {
            holes.append(Hole(key: row.nounLabel, value: "Not eligible", kind: .warn, retryInstruments: false))
        }

        let statusLabel: String
        let statusKind: StatusKind
        if holes.isEmpty {
            statusLabel = "All clear"
            statusKind = .quiet
        } else if holes.count == 1 {
            statusLabel = "\(holes[0].key) \(holes[0].value)"
            statusKind = holes[0].kind
        } else {
            statusLabel = "\(holes.count) holes"
            statusKind = holes.contains(where: { $0.kind == .bad }) ? .bad : .warn
        }

        return HarnessTopChrome(
            books: books,
            holes: holes,
            dataRows: dataRows,
            statusLabel: statusLabel,
            statusKind: statusKind,
            showsRetryInstruments: retry
        )
    }

    static func isEligible(_ row: VendorFenceRow) -> Bool {
        row.status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == "up"
    }

    static func dataNounLabel(obtainNoun: String?) -> String {
        switch obtainNoun?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "history":
            return "History"
        case "amfi_nav":
            return "NAV"
        default:
            return "Market data"
        }
    }

    private static func deskHole(key: String, status: String, retryInstruments: Bool = false) -> Hole? {
        let normalized = status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard normalized != "fresh" else { return nil }
        let kind: StatusKind
        switch normalized {
        case "stale", "unknown", "loading":
            kind = .warn
        default:
            kind = .bad
        }
        return Hole(key: key, value: normalized, kind: kind, retryInstruments: retryInstruments)
    }

    @MainActor
    private static func composeBooks(
        activeSlug: String?,
        brokerSyncClass: String,
        venuePostureBySlug: [String: VenuePosture]
    ) -> [Book] {
        let active = normalizedSlug(activeSlug)
        var slugs: [String] = []
        if let active, !active.isEmpty {
            slugs.append(active)
        }
        for key in venuePostureBySlug.keys.sorted() {
            let slug = normalizedSlug(key) ?? key
            if !slug.isEmpty, !slugs.contains(slug) {
                slugs.append(slug)
            }
        }

        return slugs.map { slug in
            let posture = venuePostureBySlug[slug] ?? venuePostureBySlug.first(where: {
                normalizedSlug($0.key) == slug
            })?.value
            let isActive = slug == active
            let (stateLabel, inCluster) = bookState(
                isActive: isActive,
                brokerSyncClass: brokerSyncClass,
                posture: posture
            )
            return Book(
                slug: slug,
                displayName: NotchViewModel.brokerDisplayName(forSlug: slug),
                stateLabel: stateLabel,
                inCluster: inCluster
            )
        }
    }

    private static func bookState(
        isActive: Bool,
        brokerSyncClass: String,
        posture: VenuePosture?
    ) -> (String, Bool) {
        switch posture?.posture.lowercased() {
        case "banned":
            return ("Banned", false)
        case "backoff":
            return ("Paused", false)
        default:
            break
        }

        if isActive {
            switch brokerSyncClass.lowercased() {
            case "synced":
                return ("Live", true)
            case "syncing":
                return ("Connecting", false)
            case "stale":
                return ("Degraded", false)
            default:
                return ("Offline", false)
            }
        }

        switch posture?.posture.lowercased() {
        case "live":
            return ("Live", true)
        default:
            return ("Offline", false)
        }
    }

    private static func normalizedSlug(_ raw: String?) -> String? {
        guard let raw else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return trimmed.isEmpty ? nil : trimmed
    }
}
