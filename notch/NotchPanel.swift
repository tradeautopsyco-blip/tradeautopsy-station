import SwiftUI

extension NotchTab: Identifiable {
    var id: String { rawValue }
}

extension NotchTab {
    var icon: String { sfSymbol }

    var label: String { pillLabel }

    var sfSymbol: String {
        switch self {
        case .pulse: return "waveform.path.ecg"
        case .brief: return "sun.horizon.fill"
        case .workflows: return "arrow.triangle.2.circlepath"
        case .tai: return "brain.head.profile"
        case .positions: return "chart.bar.fill"
        case .plan: return "shield.lefthalf.filled"
        case .capture: return "square.and.pencil"
        }
    }

    var pillLabel: String {
        switch self {
        case .pulse: return "PULSE"
        case .brief: return "BRIEF"
        case .workflows: return "FLOWS"
        case .tai: return "TAI"
        case .positions: return "POS"
        case .plan: return "PLAN"
        case .capture: return "CAP"
        }
    }

    var accessibilityPillLabel: String {
        switch self {
        case .pulse: return "Open Pulse tab"
        case .brief: return "Open Brief tab"
        case .workflows: return "Open Workflows tab"
        case .tai: return "Open TAI tab"
        case .positions: return "Open Positions tab"
        case .plan: return "Open Plan tab"
        case .capture: return "Open Capture tab"
        }
    }

    var accessibilityTabName: String {
        switch self {
        case .pulse: return "Pulse"
        case .brief: return "Brief"
        case .workflows: return "Workflows"
        case .tai: return "TAI"
        case .positions: return "Positions"
        case .plan: return "Plan"
        case .capture: return "Capture"
        }
    }
}

struct NotchRootView: View {
    @ObservedObject var vm: NotchViewModel
    let hostedExpandedContent: (() -> AnyView)?
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    init(vm: NotchViewModel, hostedExpandedContent: (() -> AnyView)? = nil) {
        self.vm = vm
        self.hostedExpandedContent = hostedExpandedContent
    }

    private var expansion: CGFloat {
        vm.isExpanded ? 1.0 : 0.0
    }

    /// Spatial: expand/collapse along the notch axis (top). Reduce Motion → opacity only.
    private var expandedTransition: AnyTransition {
        reduceMotion
            ? .opacity
            : .opacity.combined(with: .move(edge: .top))
    }

    var body: some View {
        ZStack {
            Group {
                if reduceTransparency {
                    Color(hex: "#0d0d0d").opacity(0.97)
                } else {
                    NotchTheme.bgApp
                        .background(.ultraThinMaterial)
                }
            }

            vm.backgroundPulseColor
                .opacity(vm.backgroundPulseOpacity)
                .allowsHitTesting(false)

            if vm.killSwitchActive {
                Color.taDanger.opacity(0.12)
                    .allowsHitTesting(false)
            }

            Group {
                if vm.isExpanded {
                    if let hostedExpandedContent {
                        hostedExpandedContent()
                            .transition(expandedTransition)
                    } else {
                        ExpandedNotchView(viewModel: vm)
                            .transition(expandedTransition)
                    }
                } else {
                    CollapsedNotchView(viewModel: vm)
                        .transition(.opacity)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .clipShape(NotchShape(expansionProgress: expansion))
        .overlay {
            if vm.isExpanded, !vm.hasPhysicalNotch {
                RoundedRectangle(cornerRadius: BarDS.Radius.sheet, style: .continuous)
                    .strokeBorder(
                        LinearGradient(
                            colors: [
                                Color.white.opacity(0.22),
                                Color.white.opacity(0.08),
                                Color.white.opacity(0.04),
                            ],
                            startPoint: .top,
                            endPoint: .bottom
                        ),
                        lineWidth: 1
                    )
            }
        }
        .shadow(
            color: vm.isExpanded && !vm.hasPhysicalNotch ? Color.black.opacity(0.55) : .clear,
            radius: vm.isExpanded ? 28 : 0,
            y: vm.isExpanded ? 10 : 0
        )
        .animation(NotchTheme.expandCollapseAnimation, value: vm.isExpanded)
        .animation(NotchTheme.expandCollapseAnimation, value: expansion)
        .onChange(of: vm.isExpanded) { _, _ in
            vm.syncBarLiveStatePollingForVisibility()
        }
        .onChange(of: vm.activeTab) { _, _ in
            vm.syncBarLiveStatePollingForVisibility()
        }
    }
}
