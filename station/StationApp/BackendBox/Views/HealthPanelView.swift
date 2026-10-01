import SwiftUI

public struct HealthPanelView: View {
    @ObservedObject private var viewModel: HealthPanelViewModel
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(viewModel: HealthPanelViewModel, coordinator: StationAppCoordinator) {
        self.viewModel = viewModel
        self.coordinator = coordinator
    }

    public var body: some View {
        DeskPageShell(
            title: "Health",
            subtitle: "Agent, Backend Box, and harness status. Keys never appear here."
        ) {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x2) {
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
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .task {
            await viewModel.refresh()
        }
    }

    private func healthCard(_ row: HealthPanelChrome.Row) -> some View {
        DeskCard(
            title: row.title,
            status: DeskStatusPill(text: row.status, tone: pillTone(row.status))
        ) {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x1) {
                healthLine(row, indent: false)
                ForEach(row.children) { child in
                    healthLine(child, indent: true)
                }
            }
        }
    }

    private func pillTone(_ status: String) -> DeskStatusPill.Tone {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "up", "idle": return .success
        case "down", "error", "failed": return .danger
        default: return .warning
        }
    }

    private func healthLine(_ row: HealthPanelChrome.Row, indent: Bool) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Circle()
                .fill(statusColor(row.status))
                .frame(width: 8, height: 8)
                .padding(.top, 4)
            VStack(alignment: .leading, spacing: 2) {
                if indent {
                    Text(row.title)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
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
