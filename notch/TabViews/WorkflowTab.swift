import SwiftUI

struct WorkflowLeftView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("ACTIVE WORKFLOWS")

            if viewModel.activeWorkflows.isEmpty {
                emptyState("No workflows running")
            } else {
                VStack(spacing: 6) {
                    ForEach(viewModel.activeWorkflows) { wf in
                        HStack(spacing: 8) {
                            Circle()
                                .fill(wf.isRunning
                                    ? Color(hex: "#00E5C0")
                                    : Color.white.opacity(0.2))
                                .frame(width: 5, height: 5)
                                .glowEffect(
                                    wf.isRunning
                                        ? Color(hex: "#00E5C0").opacity(0.5)
                                        : Color.clear,
                                    radius: 3
                                )
                            Text(wf.name)
                                .font(.system(size: 10, weight: .medium, design: .rounded))
                                .foregroundColor(Color.white.opacity(0.7))
                            Spacer()
                            Text(wf.isRunning ? "LIVE" : "next \(wf.nextRunMinutes)m")
                                .font(.system(size: 8, weight: .semibold, design: .monospaced))
                                .foregroundColor(wf.isRunning
                                    ? Color(hex: "#00E5C0")
                                    : Color.white.opacity(0.3))
                                .tracking(0.5)
                        }
                        .padding(8)
                        .glassCard(radius: 8)
                    }
                }
            }

            divider()

            sectionHeader("QUICK RUN")
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible()),
            ], spacing: 6) {
                ForEach(viewModel.quickWorkflows) { wf in
                    quickRunButton(wf)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func quickRunButton(_ wf: QuickWorkflowItem) -> some View {
        Button {
            Task { await viewModel.triggerWorkflow(workflowId: wf.workflowId) }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: "play.fill")
                    .font(.system(size: 8, weight: .bold))
                    .foregroundColor(Color(hex: "#00E5C0").opacity(0.7))
                Text(wf.name)
                    .font(.system(size: 9, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.6))
                    .lineLimit(1)
                Spacer()
            }
            .padding(8)
            .glassCard(radius: 8)
        }
        .buttonStyle(.plain)
    }
}

struct WorkflowRightView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("LAST RUNS")

            VStack(spacing: 5) {
                ForEach(viewModel.recentWorkflowRuns) { run in
                    HStack(spacing: 8) {
                        Circle()
                            .fill(run.success
                                ? Color(hex: "#00E5C0")
                                : Color(hex: "#FF3B30"))
                            .frame(width: 4, height: 4)
                        Text(run.name)
                            .font(.system(size: 10, weight: .medium, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.6))
                            .lineLimit(1)
                        Spacer()
                        Text("\(run.minutesAgo)m ago")
                            .font(.system(size: 9, weight: .regular, design: .monospaced))
                            .foregroundColor(Color.white.opacity(0.25))
                    }
                }
            }

            Spacer()

            ghostButton("Open Workflow Labs") {
                viewModel.openDeepLink("tradeautopsy://autonomous")
            }
        }
        .padding(12)
        .glassCard(radius: 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
