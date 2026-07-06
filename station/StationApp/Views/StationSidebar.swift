import Notch
import SwiftUI

public struct StationSidebar: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @State private var hoveredRoute: StationRoute?

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            sectionHeader("SESSION")
            ForEach(StationRoute.sessionRoutes, id: \.self) { route in
                sidebarItem(route)
            }

            sectionHeader("BACKEND BOX")
                .padding(.top, 8)
            ForEach(StationRoute.backendBoxRoutes, id: \.self) { route in
                sidebarItem(route)
            }

            sectionHeader("DESK")
                .padding(.top, 8)
            ForEach(StationRoute.deskRoutes, id: \.self) { route in
                sidebarItem(route)
            }

            Spacer(minLength: 0)
        }
        .padding(.vertical, 12)
        .frame(width: 220, alignment: .topLeading)
        .frame(maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
        .overlay(alignment: .trailing) {
            Rectangle()
                .fill(BarDS.Border.divider)
                .frame(width: BarDS.borderThin)
        }
    }

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .semibold))
            .foregroundStyle(BarDS.Text.labels)
            .textCase(.uppercase)
            .padding(.horizontal, 16)
            .padding(.bottom, 4)
    }

    private func sidebarItem(_ route: StationRoute) -> some View {
        let isActive = coordinator.activeRoute == route
        let isHover = hoveredRoute == route

        return Button {
            coordinator.navigateTo(route)
        } label: {
            HStack(spacing: 8) {
                Image(systemName: route.sfSymbol)
                    .font(.system(size: 14, weight: .regular))
                    .foregroundStyle(navIconColor(isActive: isActive, isHover: isHover))
                    .opacity(isActive ? 1 : 0.7)
                    .frame(width: 16)

                Text(route.rawValue)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: isActive ? .medium : .regular))
                    .foregroundStyle(navLabelColor(isActive: isActive, isHover: isHover))

                Spacer(minLength: 0)
            }
            .padding(.vertical, 6)
            .padding(.horizontal, 10)
            .background(
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .fill(navBackground(isActive: isActive, isHover: isHover))
            )
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
        }
        .buttonStyle(.plain)
        .onHover { isHover in
            if isHover {
                hoveredRoute = route
            } else if hoveredRoute == route {
                hoveredRoute = nil
            }
        }
    }

    private func navBackground(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return Color.white.opacity(0.07) }
        if isHover { return Color.white.opacity(0.04) }
        return .clear
    }

    private func navLabelColor(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return BarDS.Text.primary }
        if isHover { return Color(hex: "#aaaaaa") }
        return Color(hex: "#666666")
    }

    private func navIconColor(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return BarDS.Text.primary }
        if isHover { return Color(hex: "#aaaaaa") }
        return Color(hex: "#666666")
    }
}
