import SwiftUI

public struct HealthPanelView: View {
    @ObservedObject private var viewModel: HealthPanelViewModel
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(viewModel: HealthPanelViewModel, coordinator: StationAppCoordinator) {
        self.viewModel = viewModel
        self.coordinator = coordinator
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Health")
                    .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(StationDS.Text.primary)
                Text("Station · Backend Box · Kill · Harness. Vendor meters hang under Box. Keys never appear.")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                    .foregroundStyle(StationDS.Text.muted)

                ForEach(viewModel.chrome.rows) { row in
                    healthCard(row)
                }

                Button {
                    Task { await coordinator.restartAgentProcess() }
                } label: {
                    Label("Restart agent on 9137", systemImage: "arrow.clockwise.circle")
                }
                .buttonStyle(.bordered)
                .help("Stops and respawns the Enforcer unless a kill switch latch is active")
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(StationDS.Fill.appPanel)
        .task {
            await viewModel.refresh()
        }
    }

    private func healthCard(_ row: HealthPanelChrome.Row) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            healthLine(row, indent: false)
            ForEach(row.children) { child in
                healthLine(child, indent: true)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(Color.white.opacity(0.04))
        )
    }

    private func healthLine(_ row: HealthPanelChrome.Row, indent: Bool) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Circle()
                .fill(statusColor(row.status))
                .frame(width: 8, height: 8)
                .padding(.top, 4)
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(row.title)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
                    Spacer()
                    Text(row.status)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .medium))
                        .foregroundStyle(statusColor(row.status))
                }
                Text(row.what)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                    .foregroundStyle(StationDS.Text.muted)
                Text(row.why)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                    .foregroundStyle(StationDS.Text.labels)
                if let error = row.error, !error.isEmpty {
                    Text(error)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                        .foregroundStyle(StationDS.Accent.red)
                }
            }
        }
        .padding(.leading, indent ? 16 : 0)
        .accessibilityLabel("\(row.title) \(row.status)")
    }

    private func statusColor(_ status: String) -> Color {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "up", "idle":
            return StationDS.Accent.green
        default:
            return StationDS.Accent.red
        }
    }
}
