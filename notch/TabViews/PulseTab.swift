import SwiftUI

struct PulseLeftView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            sectionHeader("BEHAVIORAL SIGNALS")

            VStack(spacing: 8) {
                ForEach(viewModel.signalRows) { signal in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(signal.name)
                                .font(.system(size: 10, weight: .medium, design: .rounded))
                                .foregroundColor(Color.white.opacity(0.55))
                                .frame(width: 85, alignment: .leading)
                            Spacer()
                            Text(String(format: "%.3f", signal.value))
                                .font(.system(size: 10, weight: .medium, design: .monospaced))
                                .foregroundColor(Color.white.opacity(0.7))
                        }
                        GeometryReader { geo in
                            ZStack(alignment: .leading) {
                                Capsule()
                                    .fill(Color.white.opacity(0.05))
                                    .frame(height: 2)
                                Capsule()
                                    .fill(
                                        LinearGradient(
                                            colors: [
                                                Color(hex: "#00E5C0").opacity(0.7),
                                                Color(hex: "#00E5C0"),
                                            ],
                                            startPoint: .leading,
                                            endPoint: .trailing
                                        )
                                    )
                                    .frame(
                                        width: max(2, geo.size.width * CGFloat(signal.value)),
                                        height: 2
                                    )
                                    .animation(.spring(response: 0.5), value: signal.value)
                            }
                        }
                        .frame(height: 2)
                    }
                }
            }
            .padding(12)
            .glassCard(radius: 10)

            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

struct PulseRightView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text("SESSION P&L")
                    .microLabel()
                Text(viewModel.formattedSessionPnL)
                    .font(.system(size: 28, weight: .light, design: .monospaced))
                    .foregroundColor(
                        viewModel.sessionPnL >= 0
                            ? Color(hex: "#00E5C0")
                            : Color(hex: "#FF3B30")
                    )
                    .glowEffect(
                        viewModel.sessionPnL >= 0
                            ? Color(hex: "#00E5C0").opacity(0.2)
                            : Color(hex: "#FF3B30").opacity(0.2),
                        radius: 10
                    )
                    .accessibilityLabel("Session P&L: \(viewModel.formattedSessionPnL)")
            }
            .padding(12)
            .glassCard(radius: 10)
            .frame(maxWidth: .infinity, alignment: .leading)

            HStack(spacing: 8) {
                miniStat("WIN RATE", viewModel.winRateFormatted, .taSafe)
                miniStat("TRADES", "\(viewModel.tradesToday)", .white)
                miniStat(
                    "STATE",
                    viewModel.behavioralState,
                    NotchTheme.scoreColor(viewModel.compositeScore)
                )
            }

            Spacer()

            if !viewModel.agentLocalDiagnostics.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("LOCAL AGENT")
                        .microLabel()
                    Text(viewModel.agentLocalDiagnostics)
                        .font(.system(size: 9, weight: .regular, design: .monospaced))
                        .foregroundColor(Color.white.opacity(0.45))
                        .multilineTextAlignment(.leading)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .padding(10)
                .glassCard(radius: 8)
            }

            VStack(spacing: 6) {
                primaryButton("ACTIVATE ALGO", accessibilityLabel: "Activate trading algorithm") {
                    Task { await viewModel.sendTAIMessage("Activate algo mode") }
                }
                dangerButton("KILL SWITCH", accessibilityLabel: "Activate kill switch") {
                    NotchHaptics.play(.heavy)
                    Task { await viewModel.activateKillSwitch() }
                }
                .accessibilityHint("Blocks all new entries for the session")
                .accessibilityAddTraits(.isButton)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func miniStat(_ label: String, _ value: String, _ color: Color) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(label).microLabel()
            Text(value)
                .font(.system(size: 13, weight: .semibold, design: .monospaced))
                .foregroundColor(color)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .glassCard(radius: 8)
    }
}
