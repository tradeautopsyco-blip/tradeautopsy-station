import SwiftUI

/// Workspace rail — one General tree, favorites, aligned icons (Station desk only).
public struct StationSidebarList: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @ObservedObject private var today: TodayViewModel
    @ObservedObject private var brokers: BrokersViewModel
    @Binding private var collapsed: Bool
    @Binding private var selectedSymbol: String?
    @Binding private var selectedSearch: BookAutopsySearch?

    private let rowHeight: CGFloat = 32
    private let iconSlot: CGFloat = 18
    private let rowLeadingInset: CGFloat = 10

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
            VStack(alignment: .leading, spacing: DeskChrome.Space.x1) {
                header
                section("General")
                ForEach(StationRoute.workspaceRoutes, id: \.self) { route in
                    row(
                        title: route.rawValue,
                        systemImage: route.sfSymbol,
                        trailing: routeTrailingBadge(route),
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
            }
            .padding(.horizontal, DeskChrome.Space.x1)
            .padding(.top, DeskChrome.Space.x1 + 2)
            .padding(.bottom, DeskChrome.Space.x2)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .background(WorkspaceChrome.rail)
        .accessibilityIdentifier("workspace-rail")
    }

    private var todayCountBadge: String? {
        let n = autopsy.closedTradeCount
        guard n > 0 else { return nil }
        return n > 99 ? "99+" : "\(n)"
    }

    private var header: some View {
        HStack(spacing: DeskChrome.Space.x1) {
            if !collapsed {
                Text("TradeAutopsy")
                    .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .semibold))
                    .foregroundStyle(WorkspaceChrome.text)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }
            AgentHealthToolbarStatus(coordinator: coordinator)
            Button { collapsed.toggle() } label: {
                Image(systemName: "sidebar.left")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(WorkspaceChrome.muted)
                    .frame(width: iconSlot, height: iconSlot)
            }
            .buttonStyle(.plain)
            .help(collapsed ? "Show sidebar" : "Hide sidebar")
            .accessibilityIdentifier("workspace-collapse")
        }
        .padding(.horizontal, 4)
        .padding(.bottom, DeskChrome.Space.x1)
    }

    private func section(_ title: String) -> some View {
        Group {
            if !collapsed {
                Text(title.uppercased())
                    .font(DeskChrome.sans(DeskChrome.TypeScale.caption, weight: .semibold))
                    .foregroundStyle(StationDS.Text.section)
                    .kerning(0.6)
                    .padding(.horizontal, rowLeadingInset)
                    .padding(.top, DeskChrome.Space.x2)
                    .padding(.bottom, 4)
            }
        }
    }

    private func quiet(_ title: String) -> some View {
        Group {
            if !collapsed {
                Text(title)
                    .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                    .foregroundStyle(WorkspaceChrome.faint)
                    .padding(.horizontal, rowLeadingInset)
                    .padding(.vertical, 2)
            }
        }
    }


    private func routeTrailingBadge(_ route: StationRoute) -> String? {
        if route == .today { return todayCountBadge }
        if route == .journal, coordinator.journalViewModel.sidebarDue { return "Due" }
        return nil
    }

    private func row(
        title: String,
        systemImage: String? = nil,
        mark: String? = nil,
        trailing: String? = nil,
        selected: Bool,
        identifier: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Group {
                    if let systemImage {
                        Image(systemName: systemImage)
                            .font(.system(size: 13, weight: .medium))
                            .foregroundStyle(selected ? WorkspaceChrome.accent : WorkspaceChrome.muted)
                            .frame(width: iconSlot, height: iconSlot)
                    } else if let mark {
                        Text(mark)
                            .font(DeskChrome.sans(DeskChrome.TypeScale.caption, weight: .semibold))
                            .foregroundStyle(WorkspaceChrome.muted)
                            .frame(width: iconSlot, height: iconSlot)
                            .background(StationDS.Fill.input, in: RoundedRectangle(cornerRadius: 4))
                    }
                }
                if !collapsed {
                    Text(title)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.body))
                        .foregroundStyle(selected ? WorkspaceChrome.text : WorkspaceChrome.muted)
                        .lineLimit(1)
                    Spacer(minLength: 0)
                    if let trailing {
                        Text(trailing)
                            .font(DeskChrome.mono(DeskChrome.TypeScale.footnote))
                            .foregroundStyle(WorkspaceChrome.faint)
                    }
                }
            }
            .padding(.leading, rowLeadingInset)
            .padding(.trailing, 8)
            .frame(height: rowHeight)
            .frame(maxWidth: .infinity, alignment: collapsed ? .center : .leading)
            .background(
                selected ? WorkspaceChrome.selection : Color.clear,
                in: RoundedRectangle(cornerRadius: 8, style: .continuous)
            )
            .overlay {
                if selected {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(WorkspaceChrome.accent.opacity(0.28), lineWidth: StationDS.borderThin)
                }
            }
            .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier(identifier)
    }
}
