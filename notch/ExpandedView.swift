import SwiftUI

struct LeftZoneView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        Group {
            switch viewModel.activeTab {
            case .pulse: PulseLeftView(viewModel: viewModel)
            case .brief: BriefLeftView(viewModel: viewModel)
            case .workflows: WorkflowLeftView(viewModel: viewModel)
            case .tai: TAILeftView(viewModel: viewModel)
            case .positions: PositionsLeftView(viewModel: viewModel)
            case .plan:
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            case .capture:
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .transition(.opacity.combined(with: .move(edge: .leading)))
    }
}

struct RightZoneView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        Group {
            switch viewModel.activeTab {
            case .pulse: PulseRightView(viewModel: viewModel)
            case .brief: BriefRightView(viewModel: viewModel)
            case .workflows: WorkflowRightView(viewModel: viewModel)
            case .tai: TAIRightView(viewModel: viewModel)
            case .positions: PositionsRightView(viewModel: viewModel)
            case .plan:
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            case .capture:
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .transition(.opacity.combined(with: .move(edge: .trailing)))
    }
}

struct ExpandedNotchView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    var body: some View {
        VStack(spacing: 0) {
            Button {
                viewModel.collapseExpandedFromChromeTap()
            } label: {
                VStack(spacing: 0) {
                    RoundedRectangle(cornerRadius: 99, style: .continuous)
                        .fill(Color.white.opacity(0.18))
                        .frame(width: 36, height: 4)
                        .padding(.top, 5)
                    Spacer(minLength: 0)
                }
                .frame(minHeight: BarNotchChrome.collapsedStripHeight)
                .frame(height: chromeTapStripHeight(viewModel))
                .frame(maxWidth: .infinity)
                .contentShape(Rectangle())
            }
            .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
            .accessibilityLabel("Collapse notch")

            // `NotchPanelController` sizes the panel to nearly fill `visibleFrame`; fill that space here too.
            VStack(alignment: .leading, spacing: 0) {
                if viewModel.daemonProtocolError == .protoVersion {
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(Color(hex: "#F5A524"))
                            .font(.system(size: 11))
                            .accessibilityHidden(true)
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Agent update required")
                                .font(.system(size: 10, weight: .bold, design: .rounded))
                                .foregroundColor(.white)
                            Text("Restart the app after updating TradeAutopsy.")
                                .font(.system(size: 9, weight: .regular, design: .rounded))
                                .foregroundColor(Color.white.opacity(0.6))
                        }
                        Spacer()
                    }
                    .padding(10)
                    .background(Color(hex: "#F5A524").opacity(0.1))
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color(hex: "#F5A524").opacity(0.25), lineWidth: 0.5)
                    )
                    .accessibilityLabel("Agent update required. Restart the app after updating.")
                    .padding(.horizontal, 20)
                    .padding(.top, 8)
                }

                if !viewModel.planSurfaceOnly,
                   viewModel.activeTab != .capture,
                   viewModel.activeTab != .plan {
                    Text("Daemon connection: \(viewModel.daemonConnectionLabel)")
                        .font(.system(size: 9, weight: .medium, design: .rounded))
                        .foregroundColor(Color.white.opacity(0.5))
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 20)
                        .padding(.top, 6)
                        .accessibilityLabel("Daemon connection: \(viewModel.daemonConnectionLabel)")
                        .accessibilityAddTraits(.isStaticText)
                        .onChange(of: viewModel.daemonConnectionLabel) { _, label in
                            NotchVoiceOver.announce(
                                "Daemon connection: \(label)",
                                assertive: false
                            )
                        }
                }

                ZStack {
                    if reduceTransparency {
                        Color(hex: "#050505")
                    } else {
                        Color(hex: "#050505")

                        RadialGradient(
                            colors: [
                                BarDS.Accent.teal.opacity(0.14),
                                Color.clear,
                            ],
                            center: .top,
                            startRadius: 0,
                            endRadius: 180
                        )
                        .opacity(0.9)
                        .allowsHitTesting(false)
                    }

                    // Station-hosted: always PLAN shell — never multi-tab CenterZone chrome.
                    if viewModel.planSurfaceOnly {
                        BarCircuitPanelView(viewModel: viewModel)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    } else if viewModel.activeTab == .capture {
                        JournalCapturePanelView(viewModel: viewModel)
                    } else if viewModel.activeTab == .plan {
                        BarCircuitPanelView(viewModel: viewModel)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    } else {
                        HStack(spacing: 0) {
                            LeftZoneView(viewModel: viewModel)
                                .frame(minWidth: 240, maxWidth: .infinity)
                                .padding(.leading, 20)

                            CenterZoneView(viewModel: viewModel)
                                .frame(minWidth: 260, idealWidth: 300, maxWidth: 360)

                            RightZoneView(viewModel: viewModel)
                                .frame(minWidth: 240, maxWidth: .infinity)
                                .padding(.trailing, 20)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                    }
                }
                .overlay(
                    Rectangle()
                        .fill(Color.white.opacity(0.04))
                        .frame(height: 0.5),
                    alignment: .bottom
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    /// Collapse target: notch camera strip height, or floating HUD chrome height (parity with collapsed pill strip).
    private func chromeTapStripHeight(_ vm: NotchViewModel) -> CGFloat {
        vm.notchTopInset > 0 ? vm.notchTopInset : BarNotchChrome.collapsedStripHeight
    }
}
