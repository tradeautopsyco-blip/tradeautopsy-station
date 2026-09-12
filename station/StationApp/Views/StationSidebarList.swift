import SwiftUI

public struct StationSidebarList: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @ObservedObject private var journalViewModel: JournalViewModel

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
        self.journalViewModel = coordinator.journalViewModel
    }

    // Local to this view rather than on StationAppCoordinator: the coordinator has no
    // SwiftUI import today and is consumed by non-UI callers (StatusItemController,
    // HotkeyRegistrar); it already exposes what's needed (activeRoute getter,
    // navigateTo(_:)), so keep the UI-framework type out of it.
    private var selection: Binding<StationRoute?> {
        Binding(
            get: { coordinator.activeRoute },
            set: { newValue in
                if let newValue {
                    coordinator.navigateTo(newValue)
                }
            }
        )
    }

    public var body: some View {
        List(selection: selection) {
            Section("Session") {
                ForEach(StationRoute.sessionRoutes, id: \.self) { route in
                    Label(route.rawValue, systemImage: route.sfSymbol).tag(route)
                }
            }
            .collapsible(false)

            Section("Backend Box") {
                ForEach(StationRoute.backendBoxRoutes, id: \.self) { route in
                    Label(route.rawValue, systemImage: route.sfSymbol).tag(route)
                }
            }
            .collapsible(false)

            Section("Desk") {
                ForEach(StationRoute.deskRoutes, id: \.self) { route in
                    if route == .journal && journalViewModel.sidebarDue {
                        Label {
                            HStack {
                                Text(route.rawValue)
                                Spacer()
                                Text("Due")
                                    .font(StationDS.bodyFont(StationDS.FontSize.chrome, weight: .semibold))
                                    .foregroundStyle(StationDS.Accent.amber)
                            }
                        } icon: {
                            Image(systemName: route.sfSymbol)
                        }
                        .tag(route)
                    } else {
                        Label(route.rawValue, systemImage: route.sfSymbol).tag(route)
                    }
                }
            }
            .collapsible(false)
        }
        .listStyle(.sidebar)
        .scrollContentBackground(.hidden)
        .background(StationDS.Fill.sidebar)
        .tint(StationDS.Accent.teal)
    }
}
