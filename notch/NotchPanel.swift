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

    /// Spotlight summon: the window never animates its frame — the controller snaps it to the
    /// final rect and only these compositor properties move (opacity + scale, no travel).
    private var summonScale: CGFloat {
        if reduceMotion { return 1.0 }
        return vm.isExpanded ? 1.0 : 0.96
    }

    /// Pill hides while the window still holds the expanded frame (collapse exit fade) —
    /// otherwise it would render centered in the big rect.
    private var showsCollapsedPill: Bool {
        !vm.isExpanded && !vm.summonPanelAtExpandedFrame
    }

    var body: some View {
        ZStack(alignment: .top) {
            if showsCollapsedPill {
                collapsedLayer
                    .transition(.opacity)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        // `overlay`, not a ZStack sibling: the pre-warmed expanded layer is laid out at the full
        // expanded size while the window is still the pill, and as a sibling it would size this
        // container to ~visibleFrame and lay the pill out inside that instead of the pill window.
        // An overlay overflows its parent without ever feeding size back into it.
        .overlay(alignment: .top) { expandedLayer }
        .ignoresSafeArea()
        .animation(NotchTheme.expandCollapseAnimation, value: vm.isExpanded)
        .onChange(of: vm.isExpanded) { _, _ in
            vm.syncBarLiveStatePollingForVisibility()
        }
        .onChange(of: vm.activeTab) { _, _ in
            vm.syncBarLiveStatePollingForVisibility()
        }
    }

    private var collapsedLayer: some View {
        ZStack {
            Group {
                if vm.hasPhysicalNotch {
                    Color.black
                } else if reduceTransparency {
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

            CollapsedNotchView(viewModel: vm)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .clipShape(
            NotchShape(
                expansionProgress: 0,
                hardwareChin: vm.hasPhysicalNotch,
            )
        )
    }

    /// Always mounted (pre-warmed at `start()`): first ⌥Space is never a cold SwiftUI mount.
    /// Laid out at the final expanded size even while the window is the collapsed pill, so
    /// the summon does zero layout — only opacity and scale composite in.
    private var expandedLayer: some View {
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
                if let hostedExpandedContent {
                    hostedExpandedContent()
                } else {
                    ExpandedNotchView(viewModel: vm)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(
            width: vm.expandedSurfaceSize.width > 0 ? vm.expandedSurfaceSize.width : nil,
            height: vm.expandedSurfaceSize.height > 0 ? vm.expandedSurfaceSize.height : nil
        )
        .clipShape(NotchShape(expansionProgress: 1))
        .overlay {
            if !vm.hasPhysicalNotch {
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
        .compositingGroup()
        // Constant radius — shadow fades with the surface, its radius never animates.
        .shadow(
            color: vm.isExpanded && !vm.hasPhysicalNotch ? Color.black.opacity(0.55) : .clear,
            radius: 28,
            y: 10
        )
        .opacity(vm.isExpanded ? 1 : 0)
        .scaleEffect(summonScale, anchor: .top)
        .allowsHitTesting(vm.isExpanded)
        .accessibilityHidden(!vm.isExpanded)
    }
}
