import Notch
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
                    .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(BarDS.Text.primary)

                DeviceLoginView(viewModel: coordinator.deviceLoginViewModel)

                Toggle("Launch at Login", isOn: Binding(
                    get: { coordinator.launchAtLoginEnabled },
                    set: { _ in coordinator.toggleLaunchAtLogin() }
                ))
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundStyle(BarDS.Text.primary)
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
    }
}
