import Notch
import SwiftUI

public struct StationShellView: View {
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        VStack(spacing: 0) {
            sessionPulseStripPlaceholder

            if let warning = coordinator.agentHealthWarning {
                WarningBannerView(warning: warning) {
                    Task { await coordinator.retryAgent() }
                }
            }

            HStack(spacing: 0) {
                StationSidebar(coordinator: coordinator)
                StationPlaceholderView(route: coordinator.activeRoute)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
    }

    private var sessionPulseStripPlaceholder: some View {
        HStack {
            Text("P&L —")
                .font(BarDS.monoFont(BarDS.FontSize.bodySmall, weight: .medium))
                .foregroundStyle(BarDS.Text.secondary)
            Spacer()
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity)
        .background(BarDS.Fill.appPanel)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(BarDS.Border.divider)
                .frame(height: BarDS.borderThin)
        }
    }
}
