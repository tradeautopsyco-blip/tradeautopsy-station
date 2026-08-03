import SwiftUI

public struct SettingsView: View {
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Settings")
                    .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(StationDS.Text.primary)

                DeviceLoginView(viewModel: coordinator.deviceLoginViewModel)

                Toggle("Launch at Login", isOn: Binding(
                    get: { coordinator.launchAtLoginEnabled },
                    set: { _ in coordinator.toggleLaunchAtLogin() }
                ))
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                .foregroundStyle(StationDS.Text.primary)
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
