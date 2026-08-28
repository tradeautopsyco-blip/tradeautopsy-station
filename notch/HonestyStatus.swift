import Foundation

/// Shared honesty dialect. Lit/success is not a fifth case.
enum HonestyStatus: String, CaseIterable, Hashable {
    case empty
    case unavailable
    case unusable
    case inheritedDark = "inherited_dark"

    /// Short chip label for the catalog.
    var chipLabel: String {
        switch self {
        case .empty: return "empty"
        case .unavailable: return "unavailable"
        case .unusable: return "unusable"
        case .inheritedDark: return "inherited dark"
        }
    }

    /// Parse a desk extract `status` wire string. Unknown dialects (fresh/stale) are not honesty.
    static func fromWire(_ raw: String) -> HonestyStatus? {
        HonestyStatus(rawValue: raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased())
    }
}
