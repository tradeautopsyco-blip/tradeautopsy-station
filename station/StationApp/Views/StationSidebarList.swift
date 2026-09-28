import SwiftUI

/// Workspace rail. Counts and favorites from today; styling follows StationDS + tradeautopsy-design skill.
public struct StationSidebarList: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @ObservedObject private var today: TodayViewModel
    @ObservedObject private var brokers: BrokersViewModel
    @Binding private var collapsed: Bool
    @Binding private var selectedSymbol: String?
    @Binding private var selectedSearch: BookAutopsySearch?

    public init(
        coordinator: StationAppCoordinator,
        collapsed: Binding<Bool>,
        selectedSymbol: Binding<String?>,
        selectedSearch: Binding<BookAutopsySearch?>
    ) {
        self.coordinator = coordinator
        self.today = coordinator.todayViewModel
        self.brokers = coordinator.brokersViewModel
        self._collapsed = collapsed
        self._selectedSymbol = selectedSymbol
        self._selectedSearch = selectedSearch
    }

    private var autopsy: BookAutopsyPresentation {
        BookAutopsyPresentation.build(
            screen: today.presentation,
            brokerSlug: today.desk.brokerSlug,
            configuredBrokerSlugs: brokers.configuredBrokerSlugs,
            search: selectedSearch
        )
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                header
                countRow(
                    title: "Signals",
                    systemImage: "tray.full",
                    count: autopsy.firingSignalCount,
                    identifier: "workspace-signals"
                ) { coordinator.navigateTo(.today) }
                countRow(
                    title: "Today",
                    systemImage: "sun.max",
                    count: autopsy.closedTradeCount,
                    identifier: "workspace-today-count"
                ) { coordinator.navigateTo(.today) }
                section("General")
                ForEach(StationRoute.workspaceRoutes, id: \.self) { route in
                    row(
                        title: route.rawValue,
                        systemImage: route.sfSymbol,
                        selected: coordinator.activeRoute == route,
                        identifier: "workspace-route-\(route.rawValue)"
                    ) { coordinator.navigateTo(route) }
                }
                section("Favorites")
                if autopsy.favorites.isEmpty {
                    quiet("No symbols today")
                } else {
                    ForEach(autopsy.favorites, id: \.self) { symbol in
                        row(
                            title: symbol,
                            mark: String(symbol.prefix(1)),
                            selected: coordinator.activeRoute == .report && selectedSymbol == symbol,
                            identifier: "workspace-favorite-\(symbol)"
                        ) {
                            if coordinator.activeRoute == .report, selectedSymbol == symbol {
                                selectedSymbol = nil
                            } else {
                                selectedSymbol = symbol
                                coordinator.navigateTo(.report)
                            }
                        }
                    }
                }
                section("Searches")
                ForEach(BookAutopsySearch.allCases) { search in
                    row(
                        title: search.rawValue,
                        dot: selectedSearch == search,
                        selected: selectedSearch == search,
                        identifier: "workspace-search-\(search.rawValue)"
                    ) {
                        if selectedSearch == search {
                            selectedSearch = nil
                        } else {
                            selectedSearch = search
                            coordinator.navigateTo(.report)
                        }
                    }
                }
            }
            .padding(.horizontal, 8)
            .padding(.top, 10)
            .padding(.bottom, 14)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .background(WorkspaceChrome.rail)
        .accessibilityIdentifier("workspace-rail")
    }

    private var header: some View {
        HStack(spacing: 6) {
            if !collapsed {
                Text("TradeAutopsy")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(WorkspaceChrome.text)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }
            AgentHealthToolbarStatus(coordinator: coordinator)
            Button { collapsed.toggle() } label: {
                Image(systemName: "sidebar.left")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(WorkspaceChrome.muted)
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(.plain)
            .help(collapsed ? "Show sidebar" : "Hide sidebar")
            .accessibilityIdentifier("workspace-collapse")
        }
        .padding(.horizontal, 4)
        .padding(.bottom, 8)
    }

    private func section(_ title: String) -> some View {
        Group {
            if !collapsed {
                Text(title)
                    .font(StationDS.bodyFont(StationDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundStyle(StationDS.Text.section)
                    .padding(.horizontal, 8)
                    .padding(.top, 12)
                    .padding(.bottom, 4)
            }
        }
    }

    private func quiet(_ title: String) -> some View {
        Group {
            if !collapsed {
                Text(title)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                    .foregroundStyle(WorkspaceChrome.faint)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
            }
        }
    }

    private func countRow(
        title: String,
        systemImage: String,
        count: Int,
        identifier: String,
        action: @escaping () -> Void
    ) -> some View {
        row(
            title: title,
            systemImage: systemImage,
            trailing: count > 0 ? (count > 99 ? "99+" : "\(count)") : nil,
            selected: false,
            identifier: identifier,
            action: action
        )
    }

    private func row(
        title: String,
        systemImage: String? = nil,
        mark: String? = nil,
        dot: Bool = false,
        trailing: String? = nil,
        selected: Bool,
        identifier: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 8) {
                if let systemImage {
                    Image(systemName: systemImage)
                        .font(.system(size: 12))
                        .foregroundStyle(selected ? WorkspaceChrome.accent : WorkspaceChrome.muted)
                        .frame(width: 14)
                } else if let mark {
                    Text(mark)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .semibold))
                        .foregroundStyle(WorkspaceChrome.muted)
                        .frame(width: 16, height: 16)
                        .background(StationDS.Fill.input, in: RoundedRectangle(cornerRadius: 4))
                } else if dot {
                    Circle()
                        .fill(selected ? WorkspaceChrome.accent : WorkspaceChrome.faint)
                        .frame(width: 6, height: 6)
                }
                if !collapsed {
                    Text(title)
                        .font(StationDS.bodyFont(StationDS.FontSize.body))
                        .foregroundStyle(selected ? WorkspaceChrome.text : WorkspaceChrome.muted)
                        .lineLimit(1)
                    Spacer(minLength: 0)
                    if let trailing {
                        Text(trailing)
                            .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(WorkspaceChrome.faint)
                    }
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .frame(maxWidth: .infinity, alignment: collapsed ? .center : .leading)
            .background(selected ? WorkspaceChrome.selection : Color.clear, in: RoundedRectangle(cornerRadius: 6))
            .overlay {
                if selected {
                    RoundedRectangle(cornerRadius: 6)
                        .stroke(WorkspaceChrome.accent.opacity(0.35), lineWidth: StationDS.borderThin)
                }
            }
            .contentShape(RoundedRectangle(cornerRadius: 6))
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier(identifier)
    }
}
