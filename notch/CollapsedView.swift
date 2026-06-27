import SwiftUI

struct CollapsedNotchView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceMotion) private var accessibilityReduceMotion

    /// Subtle pill feedback only — never expands the panel on hover.
    @State private var pillHoverFeedback: Bool = false

    private var presentation: CollapsedNotchPresentation { viewModel.collapsedNotchPresentation }

    private func coreStrip(_ p: CollapsedNotchPresentation) -> some View {
        Group {
            switch p.layout {
            case .intervention(let keyword, _, _):
                interventionStrip(keyword: keyword)
            case .intraday(let indicator, let behavioralLabel):
                intradayStrip(indicator: indicator, behavioralLabel: behavioralLabel)
            case .scalper(let trades, let loss, let timeRemaining):
                scalperStrip(trades: trades, loss: loss, timeRemaining: timeRemaining)
            case .swing(let daysLabel, let statusTitle, let weeklyPnL):
                swingStrip(daysLabel: daysLabel, statusTitle: statusTitle, weeklyPnL: weeklyPnL)
            }
        }
        .padding(.horizontal, 16)
        .frame(height: BarNotchChrome.collapsedStripHeight)
    }

    private func interventionStrip(keyword: String) -> some View {
        ZStack {
            Text(keyword)
                .font(BarNotchChrome.interventionKeywordFont())
                .foregroundColor(.white)
                .tracking(0.4)
                .lineLimit(1)
                .minimumScaleFactor(0.75)
            HStack(spacing: 8) {
                Spacer(minLength: 0)
                if viewModel.journalCaptureLastPendingCaptureId != nil {
                    Rectangle()
                        .fill(Color.white.opacity(0.1))
                        .frame(width: 0.5, height: 12)
                    cameraCaptureBlock
                }
                connectionCluster
            }
        }
    }

    private func intradayStrip(indicator: CollapsedNotchScoreIndicatorKind, behavioralLabel: String) -> some View {
        HStack(spacing: 10) {
            scoreIndicator(for: indicator)

            HStack(spacing: 6) {
                Text(String(format: "%.2f", viewModel.compositeScore))
                    .font(BarDS.monoFont(13, weight: .semibold))
                    .foregroundColor(BarDS.Text.primary)

                Text(behavioralLabel)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
                    .tracking(0.5)
            }

            if viewModel.sessionPnL != 0 {
                Rectangle()
                    .fill(BarDS.Border.divider)
                    .frame(width: BarDS.borderThin, height: 12)

                Text(viewModel.formattedSessionPnL)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(
                        viewModel.sessionPnL > 0
                            ? BarDS.Accent.teal
                            : BarDS.Accent.red
                    )
            }

            if viewModel.journalCaptureLastPendingCaptureId != nil {
                Rectangle()
                    .fill(BarDS.Border.divider)
                    .frame(width: BarDS.borderThin, height: 12)
                cameraCaptureBlock
            }

            connectionCluster
        }
    }

    private func scalperStrip(trades: String, loss: String?, timeRemaining: String?) -> some View {
        HStack(spacing: 10) {
            Text(trades)
                .font(BarDS.monoFont(12, weight: .semibold))
                .foregroundColor(BarDS.Text.primary)

            if let loss {
                Text(loss)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.primary.opacity(0.72))
                    .lineLimit(1)
            }

            Spacer(minLength: 4)

            if let timeRemaining {
                Text(timeRemaining)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
            }

            if viewModel.journalCaptureLastPendingCaptureId != nil {
                Rectangle()
                    .fill(BarDS.Border.divider)
                    .frame(width: BarDS.borderThin, height: 12)
                cameraCaptureBlock
            }

            connectionCluster
        }
    }

    private func swingStrip(daysLabel: String, statusTitle: String, weeklyPnL: String?) -> some View {
        HStack(spacing: 10) {
            Text(daysLabel)
                .font(BarDS.bodyFont(10, weight: .semibold))
                .foregroundColor(BarDS.Text.secondary)

            Text(statusTitle)
                .font(BarDS.bodyFont(10, weight: .bold))
                .foregroundColor(BarDS.Text.primary)
                .tracking(0.3)

            if let weeklyPnL {
                Text(weeklyPnL)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.primary.opacity(0.72))
            }

            Spacer(minLength: 4)

            if viewModel.journalCaptureLastPendingCaptureId != nil {
                Rectangle()
                    .fill(BarDS.Border.divider)
                    .frame(width: BarDS.borderThin, height: 12)
                cameraCaptureBlock
            }

            connectionCluster
        }
    }

    private var cameraCaptureBlock: some View {
        Button {
            Task { await viewModel.attachJournalCaptureScreenshotToPending() }
        } label: {
            Group {
                if viewModel.journalCaptureScreenshotBusy {
                    ProgressView()
                        .scaleEffect(0.45)
                        .frame(width: 14, height: 14)
                } else {
                    Image(systemName: "camera.viewfinder")
                        .font(.system(size: 11, weight: .medium))
                }
            }
            .foregroundColor(screenshotPillAccent(viewModel))
        }
        .buttonStyle(.plain)
        .help("Attach screenshot to your last pending capture")
        .accessibilityLabel("Attach screenshot to pending capture")
        .disabled(viewModel.journalCaptureScreenshotBusy)
    }

    private var connectionCluster: some View {
        HStack(spacing: 5) {
            Circle()
                .fill(viewModel.daemonConnectionColor)
                .frame(width: 6, height: 6)
                .accessibilityLabel("Agent connection: \(viewModel.daemonConnectionLabel)")
            Image(systemName: "circle.fill")
                .font(.system(size: 5))
                .foregroundColor(brokerSyncDotColor(viewModel.brokerSyncClass))
                .accessibilityLabel("Broker: \(viewModel.brokerSyncClass)")
            Group {
                if viewModel.daemonProtocolError == .protoVersion {
                    HStack(spacing: 4) {
                        Image(systemName: "arrow.down.circle.fill")
                            .font(.system(size: 9))
                            .foregroundColor(BarDS.Accent.amber)
                            .accessibilityHidden(true)
                        Text("Update agent")
                            .font(BarDS.bodyFont(9, weight: .semibold))
                            .foregroundColor(BarDS.Accent.amber)
                    }
                    .accessibilityLabel("Agent update required — version mismatch")
                } else {
                    Text(viewModel.daemonConnectionLabel)
                        .font(BarDS.bodyFont(9, weight: .medium))
                        .foregroundColor(BarDS.Text.secondary)
                        .lineLimit(1)
                }
            }
        }
        .padding(.leading, 2)
    }

    private func scoreIndicator(for kind: CollapsedNotchScoreIndicatorKind) -> some View {
        Group {
            if shouldPulse(kind: kind), !accessibilityReduceMotion {
                TimelineView(.animation(minimumInterval: 1.0 / 30.0)) { timeline in
                    let t = timeline.date.timeIntervalSinceReferenceDate
                    let scale = 1.0 + 0.12 * sin(t * (2 * .pi / 1.2))
                    scoreIndicatorCore(kind: kind, scale: scale)
                }
            } else {
                scoreIndicatorCore(kind: kind, scale: 1.0)
                    .animation(.spring(response: 0.3), value: viewModel.compositeScore)
            }
        }
    }

    private func shouldPulse(kind: CollapsedNotchScoreIndicatorKind) -> Bool {
        switch kind {
        case .slMissingAmber:
            return false
        case .riskColored(let score):
            return score > 0.25
        }
    }

    private func barScoreColor(_ score: Double) -> Color {
        if score < 0.20 { return BarDS.Accent.teal }
        if score < 0.30 { return BarDS.Accent.amber }
        return BarDS.Accent.red
    }

    private func barScoreGlow(_ score: Double) -> Color {
        barScoreColor(score).opacity(0.15)
    }

    @ViewBuilder
    private func scoreIndicatorCore(kind: CollapsedNotchScoreIndicatorKind, scale: CGFloat) -> some View {
        switch kind {
        case .slMissingAmber:
            ZStack {
                Circle()
                    .fill(BarDS.Accent.amber.opacity(0.15))
                    .frame(width: 18, height: 18)
                    .blur(radius: 4)
                Circle()
                    .fill(BarDS.Accent.amber)
                    .frame(width: 8, height: 8)
            }
            .scaleEffect(scale)
        case .riskColored(let score):
            ZStack {
                Circle()
                    .fill(barScoreGlow(score))
                    .frame(width: 18, height: 18)
                    .blur(radius: 4)
                    .opacity(score > 0.20 ? 1 : 0)

                Circle()
                    .fill(barScoreColor(score))
                    .frame(width: 8, height: 8)
            }
            .scaleEffect(scale)
        }
    }

    var body: some View {
        let p = presentation
        Group {
            if viewModel.hasPhysicalNotch {
                coreStrip(p)
                    .background(
                        Capsule(style: .continuous)
                            .fill(Color.black.opacity(0.0))
                    )
            } else {
                coreStrip(p)
                    .padding(.horizontal, 4)
                    .background(capsuleBackground(p))
                    .overlay(tiltPulseRing(p))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .overlay {
            if viewModel.hasPhysicalNotch {
                tiltPulseRing(p)
            }
        }
        .opacity(pillHoverFeedback ? 1.0 : 0.94)
        .onHover { hovering in pillHoverFeedback = hovering }
        .contentShape(Rectangle())
        .onTapGesture {
            viewModel.expandFromCollapsedChromeTap()
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "TradeAutopsy notch — \(viewModel.daemonConnectionLabel), score \(String(format: "%.2f", viewModel.compositeScore))"
        )
        .accessibilityHint("Tap to expand. Option-Space toggles notch visibility.")
    }

    private func capsuleBackground(_ p: CollapsedNotchPresentation) -> some View {
        Group {
            if case .intervention(_, let bgHex, let borderHex) = p.layout {
                Capsule(style: .continuous)
                    .fill(Color(hex: bgHex).opacity(0.16))
                    .overlay(
                        Capsule(style: .continuous)
                            .stroke(Color(hex: borderHex).opacity(0.35), lineWidth: 0.5)
                    )
            } else {
                Capsule(style: .continuous)
                    .fill(Color(hex: "#0A0A0A").opacity(0.95))
                    .overlay(
                        Capsule(style: .continuous)
                            .stroke(Color.white.opacity(0.1), lineWidth: 0.5)
                    )
            }
        }
        .shadow(
            color: Color.black.opacity(0.6),
            radius: 12, x: 0, y: 4
        )
    }

    @ViewBuilder
    private func tiltPulseRing(_ p: CollapsedNotchPresentation) -> some View {
        if p.pillPulseAmber {
            if accessibilityReduceMotion {
                Capsule(style: .continuous)
                    .stroke(BarDS.Accent.amber.opacity(0.55), lineWidth: 1.0)
            } else {
                TimelineView(.animation(minimumInterval: 1.0 / 30.0)) { timeline in
                    let t = timeline.date.timeIntervalSinceReferenceDate
                    let phase = 0.38 + 0.28 * sin(t * (2 * .pi / 1.8))
                    Capsule(style: .continuous)
                        .stroke(BarDS.Accent.amber.opacity(phase), lineWidth: 1.2)
                }
            }
        }
    }

    private func screenshotPillAccent(_ vm: NotchViewModel) -> Color {
        let err = vm.journalCaptureScreenshotError?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !err.isEmpty {
            return BarDS.Accent.red
        }
        return BarDS.Accent.teal
    }

    private func brokerSyncDotColor(_ s: String) -> Color {
        switch s.lowercased() {
        case "synced": return BarDS.Accent.teal
        case "stale": return BarDS.Accent.amber
        case "disconnected": return BarDS.Accent.red
        case "not_connected": return Color.white.opacity(0.25)
        default: return Color.white.opacity(0.25)
        }
    }
}
