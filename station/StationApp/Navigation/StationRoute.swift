import Foundation

public enum SidebarSection: String, CaseIterable, Codable, Hashable {
    case session = "Session"
    case desk = "Desk"
}

public enum StationRoute: String, CaseIterable, Codable, Hashable {
    case today = "Today"
    case preTrade = "Pre-trade"
    case liveTrade = "Live trade"
    case postTrade = "Post-trade"
    case journal = "Journal"
    case brokers = "Brokers"
    case escrowMatch = "Escrow match"
    case patterns = "Patterns"
    case fidelityScore = "Fidelity score"
    case settings = "Settings"

    public var isDesk: Bool {
        sidebarSection == .desk
    }

    public var isSession: Bool {
        sidebarSection == .session
    }

    public var sidebarSection: SidebarSection {
        switch self {
        case .today, .preTrade, .liveTrade, .postTrade:
            return .session
        case .journal, .brokers, .escrowMatch, .patterns, .fidelityScore, .settings:
            return .desk
        }
    }

    public var sfSymbol: String {
        switch self {
        case .today: return "sun.max"
        case .preTrade: return "checkmark.clipboard"
        case .liveTrade: return "bolt"
        case .postTrade: return "checklist"
        case .journal: return "book"
        case .brokers: return "link"
        case .escrowMatch: return "shield.fill"
        case .patterns: return "brain"
        case .fidelityScore: return "chart.bar.fill"
        case .settings: return "gear"
        }
    }

    public static var sessionRoutes: [StationRoute] {
        allCases.filter(\.isSession)
    }

    public static var deskRoutes: [StationRoute] {
        allCases.filter(\.isDesk)
    }
}
