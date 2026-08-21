import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct CollapsedNotchView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceMotion) private var accessibilityReduceMotion

    /// Subtle pill feedback only — never expands the panel on hover.
    @State private var pillHoverFeedback: Bool = false
    @State private var isPressing: Bool = false
    @State private var dragReducer = CollapsedPillDragReducer()

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
        .padding(.horizontal, 12)
        .frame(height: BarNotchChrome.collapsedStripHeight)
    }

    private func interventionStrip(keyword: String) -> some View {
        HStack(spacing: 12) {
            Text(keyword)
                .font(BarDS.bodyFont(13, weight: .bold))
                .foregroundColor(.white)
                .lineLimit(1)
                .minimumScaleFactor(0.75)
            pillDot(BarDS.Accent.red)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Agent connection: \(viewModel.daemonConnectionLabel). \(keyword)")
    }

    private func intradayStrip(indicator: CollapsedNotchScoreIndicatorKind, behavioralLabel: String) -> some View {
        HStack(spacing: 8) {
            scoreIndicator(for: indicator)

            Text(String(format: "%.2f", viewModel.compositeScore))
                .font(BarDS.monoFont(13, weight: .semibold))
                .monospacedDigit()
                .foregroundColor(BarDS.Text.primary)

            Text(behavioralLabel)
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .kerning(0.012 * 10)

            if viewModel.sessionPnL != 0 {
                Rectangle()
                    .fill(BarDS.Border.section)
                    .frame(width: 1, height: 12)

                Text(viewModel.formattedSessionPnL)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(
                        viewModel.sessionPnL > 0
                            ? BarDS.Accent.green
                            : BarDS.Accent.red
                    )
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "TradeAutopsy notch — \(viewModel.daemonConnectionLabel), score \(String(format: "%.2f", viewModel.compositeScore))"
        )
    }

    private func scalperStrip(trades: String, loss: String?, timeRemaining: String?) -> some View {
        HStack(spacing: 10) {
            Text(trades)
                .font(BarDS.monoFont(13, weight: .semibold))
                .foregroundColor(BarDS.Text.primary)

            if let loss {
                Text(loss)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(Color.white.opacity(0.72))
                    .lineLimit(1)
            }

            if let timeRemaining {
                Rectangle()
                    .fill(BarDS.Border.section)
                    .frame(width: 1, height: 12)
                Text(timeRemaining)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
            }

            pillDot(BarDS.Accent.amber)
        }
    }

    private func swingStrip(daysLabel: String, statusTitle: String, weeklyPnL: String?) -> some View {
        HStack(spacing: 10) {
            Text(daysLabel)
                .font(BarDS.monoFont(13, weight: .semibold))
                .foregroundColor(BarDS.Text.primary)

            Text(statusTitle)
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)

            if let weeklyPnL {
                Rectangle()
                    .fill(BarDS.Border.section)
                    .frame(width: 1, height: 12)
                Text(weeklyPnL)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.green)
            }

            pillDot(BarDS.Accent.green)
        }
    }

    private func pillDot(_ color: Color) -> some View {
        Circle()
            .fill(color)
            .frame(width: 6, height: 6)
            .shadow(color: color.opacity(0.5), radius: 3, x: 0, y: 0)
            .accessibilityHidden(true)
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

    @ViewBuilder
    private func scoreIndicatorCore(kind: CollapsedNotchScoreIndicatorKind, scale: CGFloat) -> some View {
        switch kind {
        case .slMissingAmber:
            pillDot(BarDS.Accent.amber)
                .scaleEffect(scale)
        case .riskColored(let score):
            pillDot(barScoreColor(score))
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
                    .background(capsuleBackground(p))
                    .overlay(tiltPulseRing(p))
            }
        }
        .frame(maxHeight: .infinity)
        .overlay {
            if viewModel.hasPhysicalNotch {
                tiltPulseRing(p)
            }
        }
        .opacity(pillHoverFeedback ? 1.0 : 0.94)
        .scaleEffect(isPressing ? 0.97 : 1)
        .animation(
            isPressing
                ? .easeOut(duration: 0.08)
                : .spring(response: 0.28, dampingFraction: 1.0),
            value: isPressing
        )
        .contentShape(Capsule())
        .gesture(collapsedPillPointerGesture)
        .onHover { hovering in
            pillHoverFeedback = hovering
            if hovering {
                NSCursor.openHand.push()
            } else {
                NSCursor.pop()
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel(
            "TradeAutopsy notch — \(viewModel.daemonConnectionLabel), score \(String(format: "%.2f", viewModel.compositeScore))"
        )
        .accessibilityHint("Click to expand. Drag to move. Option-Space toggles notch visibility.")
        .accessibilityAction(named: "Expand") {
            viewModel.expandFromCollapsedChromeTap()
        }
        .onPasteCommand(of: [.png, .tiff, .jpeg]) { _ in
            viewModel.ingestPastedImage()
        }
    }

    private var collapsedPillPointerGesture: some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                isPressing = true
                switch dragReducer.changed(translation: value.translation) {
                case .move:
                    NSCursor.closedHand.set()
                    viewModel.applyCollapsedPillDragFromScreen()
                case .none, .expand, .moveEnded:
                    break
                }
            }
            .onEnded { value in
                isPressing = false
                switch dragReducer.ended(translation: value.translation) {
                case .expand:
                    viewModel.expandFromCollapsedChromeTap()
                case .moveEnded:
                    viewModel.endCollapsedPillDrag()
                    if pillHoverFeedback {
                        NSCursor.openHand.set()
                    }
                case .none, .move:
                    break
                }
            }
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
                    .fill(Color(hex: "#0A0A0A"))
                    .overlay(
                        Capsule(style: .continuous)
                            .stroke(Color.white.opacity(0.1), lineWidth: 1)
                    )
            }
        }
        .shadow(
            color: Color.black.opacity(0.45),
            radius: 12, x: 0, y: 8
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
}
