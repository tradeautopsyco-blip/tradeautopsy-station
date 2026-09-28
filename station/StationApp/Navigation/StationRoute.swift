import Foundation

public enum SidebarSection: String, CaseIterable, Codable, Hashable {
    case session = "Session"
    case backendBox = "Backend Box"
    case desk = "Desk"
}

public enum StationRoute: String, CaseIterable, Codable, Hashable {
    case today = "Today"
    case journal = "Journal"
    case brokers = "Brokers"
    case health = "Health"
    case marketData = "Market Data"
    case aiWorkflow = "AI / Workflow"
    case report = "Report"
    case settings = "Settings"

    public var isDesk: Bool {
        sidebarSection == .desk
    }

    public var isSession: Bool {
        sidebarSection == .session
    }

    public var sidebarSection: SidebarSection {
        switch self {
        case .today:
            return .session
        case .journal, .brokers, .health, .marketData, .aiWorkflow, .report, .settings:
            return .desk
        }
    }

    public var sfSymbol: String {
        switch self {
        case .today: return "sun.max"
        case .journal: return "book"
        case .brokers: return "link"
        case .health: return "heart.text.square"
        case .marketData: return "chart.line.uptrend.xyaxis"
        case .aiWorkflow: return "sparkles"
        case .report: return "chart.bar"
        case .settings: return "gear"
        }
    }

    /// Rail order. Report sits with the desk, Settings stays reachable.
    public static var workspaceRoutes: [StationRoute] {
        [.today, .journal, .brokers, .health, .marketData, .aiWorkflow, .report, .settings]
    }

    public static var sessionRoutes: [StationRoute] {
        allCases.filter(\.isSession)
    }

    public static var backendBoxRoutes: [StationRoute] {
        BackendBoxRoute.allCases.map(\.stationRoute)
    }

    public static var deskRoutes: [StationRoute] {
        allCases.filter(\.isDesk).filter { BackendBoxRoute.from(stationRoute: $0) == nil }
    }

    public var isBackendBox: Bool {
        BackendBoxRoute.from(stationRoute: self) != nil
    }
}
