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
            case .impact:
                impactStrip()
            }
        }
        .padding(.horizontal, viewModel.hasPhysicalNotch ? 0 : 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func interventionStrip(keyword: String) -> some View {
        let row = HStack(spacing: 8) {
            pillDot(BarDS.Accent.red)
            Text(keyword)
                .font(BarDS.bodyFont(13, weight: .bold))
                .foregroundColor(.white)
                .lineLimit(1)
                .minimumScaleFactor(0.75)
        }
        return Group {
            if viewModel.hasPhysicalNotch {
                hangRow { row }
            } else {
                HStack(spacing: 12) {
                    Text(keyword)
                        .font(BarDS.bodyFont(13, weight: .bold))
                        .foregroundColor(.white)
                        .lineLimit(1)
                        .minimumScaleFactor(0.75)
                    pillDot(BarDS.Accent.red)
                }
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Agent connection: \(viewModel.daemonConnectionLabel). \(keyword)")
    }

    private func impactStrip() -> some View {
        let pnl = presentation.pnlText
        let row = HStack(alignment: .center, spacing: 8) {
            collapsedLogo
            Spacer(minLength: 4)
            Text(pnl)
                .font(BarDS.monoFont(11, weight: .semibold))
                .monospacedDigit()
                .foregroundColor(sessionPnLColor)
                .lineLimit(1)
                .minimumScaleFactor(0.65)
        }
        return Group {
            if viewModel.hasPhysicalNotch {
                hangRow { row }
            } else {
                row
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "TradeAutopsy notch — \(viewModel.daemonConnectionLabel), P&L \(pnl)"
        )
    }

    /// Left ear — same slot as the Music app mark on Dynamic Island.
    private var collapsedLogo: some View {
        Text("T")
            .font(.system(size: 10, weight: .bold, design: .rounded))
            .foregroundColor(.white)
            .frame(width: 16, height: 16)
            .background(Circle().fill(Color.white.opacity(0.14)))
            .accessibilityHidden(true)
    }

    private var sessionPnLColor: Color {
        if viewModel.sessionPnL > 0 { return BarDS.Accent.green }
        if viewModel.sessionPnL < 0 { return BarDS.Accent.red }
        return .white
    }

    /// Camera spacer on top; one centered row in the visible hang strip.
    @ViewBuilder
    private func hangRow<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        VStack(spacing: 0) {
            Color.clear
                .frame(height: viewModel.notchTopInset)
            content()
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                .padding(.horizontal, 10)
                .padding(.bottom, 4)
        }
    }

    private func impactTone(_ impact: AccountImpact) -> Color {
        switch impact {
        case .unknown, .known(_, .zero):
            return BarDS.Text.secondary
        case .known(_, .down):
            return Color(red: 196 / 255, green: 72 / 255, blue: 68 / 255)
        case .known(_, .up):
            return Color(red: 52 / 255, green: 168 / 255, blue: 96 / 255)
        }
    }

    private func pillDot(_ color: Color) -> some View {
        Circle()
            .fill(color)
            .frame(width: 6, height: 6)
            .shadow(color: color.opacity(0.5), radius: 3, x: 0, y: 0)
            .accessibilityHidden(true)
    }

    private func collapsedAccessibilityLabel(_ p: CollapsedNotchPresentation) -> String {
        switch p.layout {
        case .intervention(let keyword, _, _):
            return "TradeAutopsy notch — \(viewModel.daemonConnectionLabel), \(keyword)"
        case .impact:
            return "TradeAutopsy notch — \(viewModel.daemonConnectionLabel), P&L \(p.pnlText)"
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
        .accessibilityLabel(collapsedAccessibilityLabel(p))
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
            } else if case .impact(let impact) = p.layout {
                Capsule(style: .continuous)
                    .fill(Color(hex: "#0A0A0A"))
                    .overlay(
                        Capsule(style: .continuous)
                            .stroke(impactTone(impact).opacity(0.22), lineWidth: 1)
                    )
                    .shadow(color: impactTone(impact).opacity(0.18), radius: 10, x: 0, y: 4)
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
