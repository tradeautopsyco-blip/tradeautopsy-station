import SwiftUI

/// PLAN expanded surface — sidebar, top bar, and routed main content (visual shell only).
public enum BarNotchScreen: String, CaseIterable {
    case morning = "Morning brief"
    case pretrade = "Pre-trade"
    case live = "Live trade"
    case posttrade = "Post-trade"
    case escrow = "Escrow match"
    case patterns = "Patterns"
    case fidelity = "Fidelity score"
    case triage = "Triage"
    case settings = "Settings"

    public var isSessionGroup: Bool {
        switch self {
        case .morning, .pretrade, .live, .posttrade: return true
        default: return false
        }
    }
}

struct BarNotchShell: View {
    @ObservedObject var viewModel: NotchViewModel
    private let externalActiveScreen: Binding<BarNotchScreen>?
    @State private var localActiveScreen: BarNotchScreen = .morning
    @AppStorage("notch.planMorningBriefConsumed") private var morningBriefConsumed: Bool = false
    @State private var hoveredSession: BarNotchScreen?
    @State private var analysisOpen: Bool = false
    @State private var freshnessClock = Date()
    @ObservedObject private var deskModeStore = RiskDeskModeStore.shared

    init(viewModel: NotchViewModel, activeScreen: Binding<BarNotchScreen>? = nil) {
        self.viewModel = viewModel
        self.externalActiveScreen = activeScreen
    }

    private var activeScreen: Binding<BarNotchScreen> {
        externalActiveScreen ?? $localActiveScreen
    }

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            sidebar
            VStack(alignment: .leading, spacing: 0) {
                topBar
                loadingOrErrorStrip
                mainScroll
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(BarDS.Fill.appPanel)
        }
        .background(BarDS.Fill.appPanel)
        .onAppear {
            syncActiveScreenFromPhase(animated: false)
            syncDeclarationFormFlagToActiveScreen()
            openAnalysisIfNeeded(for: activeScreen.wrappedValue)
        }
        .onReceive(NotchOneSecondClock.publisher) { freshnessClock = $0 }
        .onChange(of: viewModel.barSurfacePhase) { _, _ in
            guard externalActiveScreen == nil else { return }
            syncActiveScreenFromPhase(animated: true)
        }
        .onChange(of: viewModel.morningBrief != nil) { _, has in
            if !has { morningBriefConsumed = false }
        }
        .onChange(of: activeScreen.wrappedValue) { _, new in
            syncDeclarationFormFlagToActiveScreen(screen: new)
            openAnalysisIfNeeded(for: new)
        }
        .onChange(of: viewModel.showingDeclarationForm) { _, showing in
            guard showing, activeScreen.wrappedValue != .pretrade else { return }
            withAnimation(NotchTheme.contentAnimation) {
                activeScreen.wrappedValue = .pretrade
            }
        }
        .onChange(of: viewModel.requestLiveCaptureScreen) { _, flag in
            guard flag else { return }
            if viewModel.consumeLiveCaptureScreenRequest() {
                withAnimation(NotchTheme.contentAnimation) {
                    activeScreen.wrappedValue = .live
                }
            }
        }
        .alert("Replace this trade’s chart?", isPresented: replaceChartAlertPresented) {
            Button("Cancel", role: .cancel) {
                viewModel.cancelReplaceChart()
            }
            Button("Replace", role: .destructive) {
                Task { await viewModel.confirmReplaceChart() }
            }
        } message: {
            Text("\(replaceConfirmSymbol) already has one. Confirm replaces it. One image per trade.")
        }
    }

    private func syncDeclarationFormFlagToActiveScreen(screen: BarNotchScreen? = nil) {
        let s = screen ?? activeScreen.wrappedValue
        viewModel.setIfChanged(\.showingDeclarationForm, s == .pretrade)
    }

    private func openAnalysisIfNeeded(for screen: BarNotchScreen) {
        switch screen {
        case .escrow, .patterns, .fidelity, .triage:
            analysisOpen = true
        default:
            break
        }
    }

    // MARK: - Phase → sidebar (does not fight user while on auxiliary tabs)

    private func syncActiveScreenFromPhase(animated: Bool) {
        let next = BarNotchPhaseRouting.screen(for: viewModel.barSurfacePhase)
        guard next != activeScreen.wrappedValue else { return }
        if animated {
            withAnimation(NotchTheme.contentAnimation) {
                activeScreen.wrappedValue = next
            }
        } else {
            activeScreen.wrappedValue = next
        }
    }

    // MARK: - Sidebar (176px)

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Session")
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(Color.white.opacity(0.5))
                .kerning(0.006 * 11)
                .textCase(.uppercase)
                .padding(.top, 12)
                .padding(.bottom, 6)
                .padding(.horizontal, 16)

            sessionNavButton(.morning, badge: morningUnreadBadgeText)
            sessionNavButton(.pretrade, badge: nil)
            sessionNavButton(.live, badges: liveSessionBadges)
            sessionNavButton(.posttrade, badge: nil)

            Button {
                withAnimation(BarDS.Motion.spring) {
                    analysisOpen.toggle()
                }
            } label: {
                HStack(spacing: 8) {
                    BarNavIcon(glyph: .chevron, size: 14)
                        .rotationEffect(.degrees(analysisOpen ? 180 : 0))
                    Text("More · analysis")
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .kerning(0.006 * 11)
                        .textCase(.uppercase)
                    Spacer(minLength: 0)
                }
                .foregroundColor(analysisOpen ? BarDS.Text.primary : Color.white.opacity(0.45))
                .padding(.vertical, 6)
                .padding(.horizontal, 8)
                .background(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .fill(analysisOpen ? Color.white.opacity(0.04) : Color.clear)
                )
                .contentShape(Rectangle())
            }
            .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
            .padding(.horizontal, 12)
            .padding(.top, 8)
            .accessibilityLabel("More analysis")
            .accessibilityValue(analysisOpen ? "Expanded" : "Collapsed")

            if analysisOpen {
                navButton(.escrow, badge: nil)
                navButton(.patterns, badge: nil)
                navButton(.fidelity, badge: nil)
                navButton(.triage, badge: nil)
            }

            Spacer(minLength: 0)

            Button {
                viewModel.requestHidePill()
            } label: {
                HStack(spacing: 10) {
                    BarNavIcon(glyph: .eyeSlash)
                    Text("Hide notch · ⌥Space")
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    Spacer(minLength: 0)
                }
                .foregroundColor(BarDS.Text.hint)
                .padding(.vertical, 7)
                .padding(.horizontal, 10)
                .contentShape(Rectangle())
            }
            .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
            .accessibilityLabel("Hide notch")
            .accessibilityHint("Hides the notch pill. Press Option-Space to show it again.")
            .padding(.horizontal, 8)
            .padding(.bottom, 4)

            archetypePill
                .padding(.horizontal, 8)
                .padding(.vertical, 8)
        }
        .frame(width: BarDS.sidebarWidth, alignment: .topLeading)
        .background(BarDS.Fill.sidebar.opacity(0.72))
        .overlay(alignment: .trailing) {
            Rectangle()
                .fill(BarDS.Border.section)
                .frame(width: BarDS.borderThin)
        }
    }

    private var morningUnreadBadgeText: String? {
        if viewModel.morningBrief != nil, !morningBriefConsumed { return "1" }
        return nil
    }

    private var liveSessionBadges: [String] {
        var badges: [String] = []
        if (viewModel.barLiveState?.activeInterventions.count ?? 0) > 0 {
            badges.append("!")
        }
        let n = viewModel.unpostedCaptures.count
        if n > 0 { badges.append("\(n)") }
        return badges
    }

    private var replaceChartAlertPresented: Binding<Bool> {
        Binding(
            get: { viewModel.replaceConfirmTradeId != nil },
            set: { if !$0 { viewModel.cancelReplaceChart() } }
        )
    }

    private var replaceConfirmSymbol: String {
        guard let id = viewModel.replaceConfirmTradeId else { return "This trade" }
        return viewModel.recentTrades.first(where: { $0.id == id })?.symbol ?? "This trade"
    }

    private func navButton(_ screen: BarNotchScreen, badge: String?) -> some View {
        let isActive = activeScreen.wrappedValue == screen
        let isHover = hoveredSession == screen
        return Button {
            if screen == .morning {
                morningBriefConsumed = true
            }
            withAnimation(NotchTheme.contentAnimation) {
                activeScreen.wrappedValue = screen
            }
        } label: {
            HStack(spacing: 10) {
                BarNavIcon(glyph: screen.navGlyph, filled: isActive)
                Text(screen.navTitle)
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: isActive ? .medium : .regular))
                    .foregroundColor(navPrimary(isActive: isActive, isHover: isHover))
                Spacer(minLength: 0)
                if let badge {
                    Text(badge)
                        .font(BarDS.bodyFont(10, weight: .medium))
                        .foregroundColor(badge == "!" ? BarDS.Accent.red : BarDS.Accent.amber)
                        .padding(.vertical, 2)
                        .padding(.horizontal, 6)
                        .background(
                            (badge == "!"
                                ? BarDS.Accent.red.opacity(0.2)
                                : BarDS.Accent.amber.opacity(0.2)
                            )
                        )
                        .clipShape(Capsule())
                }
            }
            .padding(.vertical, 7)
            .padding(.horizontal, 10)
            .background(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .fill(navBackground(isActive: isActive, isHover: isHover))
            )
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
        }
        .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
        .onHover { isHover in
            if isHover { hoveredSession = screen }
            else if hoveredSession == screen { hoveredSession = nil }
        }
    }

    /// SESSION group: SF Symbol + hex icon colors (matches spec); label uses same row styling as `navButton`.
    private func sessionNavButton(_ screen: BarNotchScreen, badge: String?) -> some View {
        sessionNavButton(screen, badges: badge.map { [$0] } ?? [])
    }

    private func sessionNavButton(_ screen: BarNotchScreen, badges: [String]) -> some View {
        let isActive = activeScreen.wrappedValue == screen
        let isHover = hoveredSession == screen
        return Button {
            if screen == .morning {
                morningBriefConsumed = true
            }
            withAnimation(NotchTheme.contentAnimation) {
                activeScreen.wrappedValue = screen
            }
        } label: {
            HStack(spacing: 10) {
                BarNavIcon(glyph: screen.navGlyph, filled: isActive)
                Text(screen.navTitle)
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: isActive ? .medium : .regular))
                    .foregroundColor(navPrimary(isActive: isActive, isHover: isHover))
                Spacer(minLength: 0)
                ForEach(badges, id: \.self) { badge in
                    Text(badge)
                        .font(BarDS.bodyFont(10, weight: .medium))
                        .foregroundColor(badge == "!" ? BarDS.Accent.red : BarDS.Accent.amber)
                        .padding(.vertical, 2)
                        .padding(.horizontal, 6)
                        .background(
                            (badge == "!"
                                ? BarDS.Accent.red.opacity(0.2)
                                : BarDS.Accent.amber.opacity(0.2)
                            )
                        )
                        .clipShape(Capsule())
                }
            }
            .padding(.vertical, 7)
            .padding(.horizontal, 10)
            .background(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .fill(navBackground(isActive: isActive, isHover: isHover))
            )
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
        }
        .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
        .onHover { isHover in
            if isHover { hoveredSession = screen }
            else if hoveredSession == screen { hoveredSession = nil }
        }
    }

    private func navBackground(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return Color.white.opacity(0.08) }
        if isHover { return Color.white.opacity(0.04) }
        return .clear
    }

    private func navPrimary(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return BarDS.Text.primary }
        if isHover { return BarDS.Text.primary }
        return BarDS.Text.secondary
    }

    private var archetypePill: some View {
        let a = viewModel.activeArchetype
        return HStack(spacing: 0) {
            HStack(spacing: 7) {
                Circle()
                    .fill(archetypeDot(a))
                    .frame(width: 6, height: 6)
                Text(archetypeLabel(a))
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            Spacer()

            Button {
                withAnimation(NotchTheme.contentAnimation) {
                    activeScreen.wrappedValue = .settings
                }
            } label: {
                BarNavIcon(
                    glyph: .gear,
                    filled: activeScreen.wrappedValue == .settings,
                    size: 16
                )
                    .foregroundColor(
                        activeScreen.wrappedValue == .settings ? BarDS.Text.primary : BarDS.Text.muted
                    )
                    .frame(width: 28, height: 28)
                    .background(
                        RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                            .fill(activeScreen.wrappedValue == .settings ? Color.white.opacity(0.08) : Color.clear)
                    )
                    .contentShape(Rectangle())
            }
            .buttonStyle(NotchPressButtonStyle(pressedScale: 0.94))
            .accessibilityLabel("Settings")
        }
        .padding(.vertical, 7)
        .padding(.horizontal, 10)
        .background(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .fill(Color.white.opacity(0.04))
        )
    }

    private func archetypeDot(_ a: TraderArchetype) -> Color {
        switch a {
        case .intraday: return BarDS.Accent.teal
        case .scalper: return Color(hex: "#3b82f6")
        case .swing: return Color(hex: "#a78bfa")
        }
    }

    private func archetypeLabel(_ a: TraderArchetype) -> String {
        switch a {
        case .intraday: return "Intraday · F&O"
        case .scalper: return "Scalper · F&O"
        case .swing: return "Swing · Equity"
        }
    }

    // MARK: - Top bar

    private var topBar: some View {
        HStack(alignment: .center, spacing: 10) {
            Text(activeScreen.wrappedValue.rawValue)
                .font(BarDS.bodyFont(BarDS.FontSize.topbarTitle, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .kerning(-0.011 * BarDS.FontSize.topbarTitle)
                .frame(maxWidth: .infinity, alignment: .leading)

            Text(String(format: "%.2f", viewModel.compositeScore))
                .font(BarDS.monoFont(11, weight: .regular))
                .monospacedDigit()
                .foregroundColor(BarDS.Text.muted)

            if let mult = viewModel.barBehavioralMultiplierLabel, !mult.isEmpty {
                Text(mult)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .monospacedDigit()
                    .foregroundColor(BarDS.Accent.amber)
            }

            liveStateFreshnessChip
            brokerConnectionPill
            deskCapabilityPills
            statePill
        }
        .padding(.vertical, 11)
        .padding(.horizontal, 16)
        .background(BarDS.Fill.sidebar.opacity(0.70))
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(BarDS.Border.section)
                .frame(height: BarDS.borderThin)
        }
    }

    private var brokerConnectionPill: some View {
        let style = brokerPillStyle
        return HStack(spacing: 5) {
            Circle()
                .fill(style.dot)
                .frame(width: 6, height: 6)
            Text(style.label)
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(style.titleColor)
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 10)
        .background(style.bg)
        .clipShape(Capsule())
    }

    /// Quote vs account stay independent — quote can be green while funds/fills are not.
    private var deskCapabilityPills: some View {
        HStack(spacing: 6) {
            capabilityPill(
                title: "Quote",
                status: viewModel.deskQuoteCapability
            )
            instrumentsCapabilityPill
            capabilityPill(
                title: "Account",
                status: DeskCapabilityChrome.accountStatus(
                    funds: viewModel.deskFundsCapability,
                    fills: viewModel.deskFillsCapability
                )
            )
        }
    }

    @ViewBuilder
    private var instrumentsCapabilityPill: some View {
        let status = viewModel.deskInstrumentsCapability
        if DeskCapabilityChrome.showsRetryInstruments(status: status) {
            Button {
                Task { await viewModel.retryInstruments() }
            } label: {
                capabilityPill(title: "Instruments", status: status)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Instruments \(status). Retry instruments")
        } else {
            capabilityPill(title: "Instruments", status: status)
        }
    }

    private func capabilityPill(title: String, status: String) -> some View {
        let dotName = DeskCapabilityChrome.dotName(forStatus: status)
        let (dot, fg, bg): (Color, Color, Color) = {
            switch dotName {
            case "teal":
                return (BarDS.Accent.teal, BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
            case "amber":
                return (BarDS.Accent.amber, BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.10))
            default:
                return (BarDS.Accent.red, BarDS.Accent.red, BarDS.Accent.red.opacity(0.10))
            }
        }()
        return HStack(spacing: 5) {
            Circle()
                .fill(dot)
                .frame(width: 6, height: 6)
            Text(DeskCapabilityChrome.pillLabel(kind: title, status: status))
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(fg)
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 10)
        .background(bg)
        .clipShape(Capsule())
        .accessibilityLabel("\(title) \(status)")
    }

    private var brokerPillStyle: (dot: Color, label: String, titleColor: Color, bg: Color) {
        let slug = viewModel.activeExecutionBrokerSlug ?? viewModel.barProtectiveBrokerSlug
        let postureKey = slug.trimmingCharacters(in: .whitespacesAndNewlines)
        let chrome = NotchViewModel.brokerPillChrome(
            brokerSyncClass: viewModel.brokerSyncClass,
            slug: slug,
            venuePosture: postureKey.isEmpty ? nil : viewModel.venuePostureBySlug[postureKey]
        )
        switch chrome.dotName {
        case "teal":
            return (BarDS.Accent.teal, chrome.label, BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        case "amber":
            return (BarDS.Accent.amber, chrome.label, BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.10))
        default:
            return (BarDS.Accent.red, chrome.label, BarDS.Accent.red, BarDS.Accent.red.opacity(0.10))
        }
    }

    private var statePill: some View {
        let style = screenBehavioralPillStyle
        return HStack(spacing: 5) {
            Circle()
                .fill(style.dot)
                .frame(width: 6, height: 6)
            Text(style.title)
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(style.titleColor)
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 10)
        .background(style.bg)
        .clipShape(Capsule())
    }

    private var screenBehavioralPillStyle: (dot: Color, title: String, titleColor: Color, bg: Color) {
        switch activeScreen.wrappedValue {
        case .morning:
            return (BarDS.Accent.teal, "Pre-market", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        case .pretrade:
            return (BarDS.Accent.teal, "Declaration", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        case .live:
            let broken = viewModel.compositeScore >= 0.45
            if broken {
                return (BarDS.Accent.amber, "Intact", BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.10))
            }
            return (BarDS.Accent.teal, "Intact", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        case .posttrade:
            return (BarDS.Accent.amber, "Debrief", BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.10))
        case .escrow:
            return (BarDS.Accent.teal, "Match", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        case .patterns:
            return (BarDS.Accent.amber, "Building", BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.10))
        case .fidelity:
            return (
                BarDS.Text.primary,
                String(format: "%.2f", viewModel.compositeScore),
                BarDS.Text.primary,
                Color.white.opacity(0.10)
            )
        case .triage:
            return (BarDS.Accent.teal, "Triage", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        case .settings:
            return (BarDS.Accent.teal, "Config", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.10))
        }
    }

    // MARK: - Header strip (action errors only — live-state poll uses the freshness chip)

    @ViewBuilder
    private var loadingOrErrorStrip: some View {
        if let err = viewModel.barStateError, !err.isEmpty, !viewModel.barStateRequiresDeviceLogin {
            Text(err)
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
                .padding(.horizontal, 16)
                .padding(.vertical, 6)
        }
    }

    private var liveStateFreshnessChip: some View {
        let signIn = viewModel.barStateRequiresDeviceLogin
        let stale = BarLiveStatePollChrome.isStale(
            lastFetched: viewModel.barLastFetched,
            now: freshnessClock
        )
        let label = signIn
            ? "Sign in"
            : BarLiveStatePollChrome.freshnessLabel(
                lastFetched: viewModel.barLastFetched,
                now: freshnessClock
            )
        let fg = signIn || stale ? BarDS.Accent.amber : BarDS.Text.muted
        return HStack(spacing: 5) {
            Text(label)
                .font(BarDS.monoFont(11, weight: .medium))
                .monospacedDigit()
                .foregroundColor(fg)
                .frame(minWidth: 64, alignment: .trailing)
            if signIn {
                Button {
                    viewModel.requestOpenDeviceLogin()
                } label: {
                    Text("Open Station")
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundColor(BarDS.Accent.teal)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 10)
        .background(fg.opacity(0.10))
        .clipShape(Capsule())
        .accessibilityLabel(
            signIn
                ? "Live Plan needs Station sign-in. Open Station."
                : "Live Plan refreshed \(label)"
        )
    }

    // MARK: - Main scroll

    @ViewBuilder
    private var mainScroll: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if activeScreen.wrappedValue == .settings {
                    BarSettingsView(viewModel: viewModel)
                } else if viewModel.barFeaturesActiveFromApi == false {
                    barGateRequired
                } else if activeScreen.wrappedValue == .pretrade {
                    BarDeclarationFlowView(viewModel: viewModel)
                } else if viewModel.barSurfacePhase == .debrief {
                    BarPostTradeView(viewModel: viewModel)
                } else if viewModel.barSurfacePhase == .declaration, viewModel.showingDeclarationForm {
                    declarationFormRoot
                } else {
                    routedBySidebar
                }
            }
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .padding(16)
        }
        .scrollIndicators(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    // MARK: - Routed declaration + phase helpers (from BarCircuitPanelView)

    private var routedDeclarationSurface: BarDeclarationSurface {
        BarDeclarationSurface.resolve(
            archetype: viewModel.activeArchetype,
            declarationKind: viewModel.barLiveState?.pendingDeclaration?.declarationKind,
        )
    }

    @ViewBuilder
    private var declarationFormRoot: some View {
        switch routedDeclarationSurface {
        case .intradayForm:
            BarDeclarationFlowView(viewModel: viewModel)
        case .scalperSessionForm:
            BarScalperSessionView(viewModel: viewModel)
        case .swingForm:
            BarSwingDeclarationView(viewModel: viewModel)
        }
    }

    @ViewBuilder
    private var routedBySidebar: some View {
        switch activeScreen.wrappedValue {
        case .morning:
            BriefLeftView(viewModel: viewModel)
        case .pretrade:
            EmptyView()
        case .live:
            liveBody
        case .posttrade:
            BarPostTradeView(viewModel: viewModel)
        case .escrow:
            BarEscrowMatchView(report: viewModel.barLiveState?.escrowMatchReport)
        case .patterns:
            BarPatternsChartView()
        case .fidelity:
            BarFidelityChartView(viewModel: viewModel)
        case .triage:
            BarTriageRouteView(viewModel: viewModel)
        case .settings:
            BarSettingsView(viewModel: viewModel)
        }
    }

    @ViewBuilder
    private var liveBody: some View {
        let flip = deskModeStore.mode == .notchFlip
        let pending = viewModel.barLiveState?.pendingDeclaration
        let declared = viewModel.barOptimisticArmedDisplay != nil
            || pending != nil
            || viewModel.barSurfacePhase == .armed
        let units = pending.map(\.quantity)
            ?? Double(viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines))
        let entry = Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines))
        let stop = pending?.stopLoss
            ?? Double(viewModel.declStopLoss.trimmingCharacters(in: .whitespacesAndNewlines))
        let sideBuy = !(pending?.side.uppercased().contains("SELL") ?? false)
        let snap = EquityIfSL.snapshot(
            units: units,
            entry: entry,
            stop: stop,
            sideBuy: sideBuy,
            declared: declared
        )
        switch viewModel.barSurfacePhase {
        case .armed:
            if viewModel.barLiveState != nil {
                VStack(alignment: .leading, spacing: 16) {
                    if flip { BarEquityIfSLBox(snapshot: snap) }
                    BarPlanStateView(viewModel: viewModel, embedEscrow: activeScreen.wrappedValue != .escrow)
                    planPanelDivider
                    barArmedWaiting
                }
            } else {
                VStack(alignment: .leading, spacing: 16) {
                    if flip { BarEquityIfSLBox(snapshot: snap) }
                    barArmedWaiting
                }
            }
        default:
            VStack(alignment: .leading, spacing: 16) {
                if flip { BarEquityIfSLBox(snapshot: snap) }
                BarPlanStateView(viewModel: viewModel, embedEscrow: activeScreen.wrappedValue != .escrow)
            }
        }
    }

    private var planPanelDivider: some View {
        Rectangle()
            .fill(Color.white.opacity(0.05))
            .frame(height: BarDS.borderThin)
            .padding(.vertical, 4)
    }

    private var barGateRequired: some View {
        let pending = viewModel.barLiveState?.pendingDeclaration
        return VStack(alignment: .leading, spacing: 10) {
            Text("Bar features are off or need setup on the web app.")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if let pending {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Pending declaration on file — finish activation on the web Bar to proceed.")
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundColor(BarDS.Accent.amber.opacity(0.9))
                        .fixedSize(horizontal: false, vertical: true)
                    Text("\(pending.symbol) · \(pending.side) · qty \(formatQty(pending.quantity))")
                        .font(BarDS.monoFont(10, weight: .medium))
                        .foregroundColor(BarDS.Text.secondary)
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.white.opacity(0.06))
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
            }
        }
    }

    private var barArmedWaiting: some View {
        let p = viewModel.barLiveState?.pendingDeclaration
        let opt = viewModel.barOptimisticArmedDisplay
        return VStack(alignment: .leading, spacing: 10) {
            if let p {
                Text("Declaration on file · waiting for fill")
                    .font(BarDS.bodyFont(13, weight: .semibold))
                    .foregroundColor(BarDS.Text.primary)
                Text("\(p.symbol) · \(p.side) · qty \(formatQty(p.quantity))")
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
                if let sl = p.stopLoss {
                    Text("Declared stop: \(viewModel.formatINR(sl))")
                        .font(BarDS.monoFont(10, weight: .medium))
                        .foregroundColor(BarDS.Text.hint)
                }
                Text(p.protectiveSlConsent ? "Protective SL consent: on file" : "Protective SL consent: missing (see web Bar)")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(
                        p.protectiveSlConsent ? BarDS.Accent.teal.opacity(0.85) : BarDS.Accent.amber
                    )
            } else if let opt {
                Text("Confirming declaration with server…")
                    .font(BarDS.bodyFont(13, weight: .semibold))
                    .foregroundColor(BarDS.Text.primary)
                Text("\(opt.symbol) · \(opt.side) · qty \(opt.quantityLabel)")
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
                Text("Ref: \(shortId(viewModel.barPublishedDeclarationId ?? opt.declarationId))")
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.hint)
            } else {
                Text("Updating plan state…")
                    .font(BarDS.bodyFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.hint)
            }
        }
    }

    private func shortId(_ id: String) -> String {
        if id.count <= 12 { return id }
        return String(id.prefix(8)) + "…"
    }

    private func formatQty(_ q: Double) -> String {
        if q == floor(q) { return String(Int(q)) }
        return String(format: "%.4f", q)
    }
}

// MARK: - Placeholders

private struct BarPlaceholderCard: View {
    let title: String
    let caption: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
            Text(caption)
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 13)
        .padding(.horizontal, 15)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
    }
}

private struct BarPatternsChartView: View {
    @State private var selected: String = "Breakout"
    private let counts: [Double: Int] = [42: 21, 28: 14, 14: 7, 10: 5, 6: 3]
    private let maxPct: Double = 50

    private var readout: String {
        guard let row = BarTAChartDemo.setupMix.first(where: { $0.x == selected }) else {
            return "Breakout · 21 of 50 trades"
        }
        let n = counts[row.y] ?? 0
        return "\(row.x) · \(n) of 50 trades"
    }

    var body: some View {
        BarTAChartFigure(
            kicker: "Setup mix · last 50 trades",
            takeaway: "Breakout is the most used setup at 42% of the last 50 trades. Horizontal bars compare categories — this is not a time series.",
            readout: readout
        ) {
            VStack(spacing: 0) {
                ForEach(BarTAChartDemo.setupMix) { row in
                    BarTABarRow(
                        label: row.x,
                        pct: row.y,
                        maxPct: maxPct,
                        selected: selected == row.x
                    ) {
                        selected = row.x
                    }
                }
            }
        }
    }
}

private struct BarFidelityChartView: View {
    @ObservedObject var viewModel: NotchViewModel
    @State private var lens: String = "last"
    @State private var readout: String = "14 Aug · 94%"

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarTAChartFigure(
                kicker: "Plan fidelity · 14 sessions",
                takeaway: "Plan fidelity averaged 92% over the last 14 sessions, up 6 points from the prior two weeks. Latest session 94%.",
                readout: readout
            ) {
                BarTALinePlot(
                    points: BarTAChartDemo.fidelitySessions,
                    kind: .fidelity,
                    height: 132,
                    includeZero: false,
                    yMinFixed: 80,
                    yMaxFixed: 100,
                    showDiamonds: true
                )
                .accessibilityLabel("Plan fidelity over 14 sessions, latest 94 percent")
            }

            fidelityLens(id: "last", title: "Last session", sub: " — micro", value: "94%") {
                readout = "14 Aug · 94%"
            }
            fidelityLens(id: "avg", title: "Average", sub: " — macro", value: "92%") {
                readout = "Average · 92%"
            }
            fidelityLens(id: "best", title: "Best day", sub: " — 11 Aug", value: "96%") {
                readout = "11 Aug · 96%"
            }

            if let pct = viewModel.barLiveState?.escrowMatchReport?.fidelityPct, pct.isFinite {
                BarProgressBlock(
                    label: "Plan fidelity (escrow)",
                    valueText: String(format: "%.0f%%", pct),
                    pct: min(1, max(0, pct / 100)),
                    fillStyle: .fidelity
                )
                .padding(.top, 8)
            }
        }
    }

    private func fidelityLens(id: String, title: String, sub: String, value: String, action: @escaping () -> Void) -> some View {
        Button {
            lens = id
            action()
        } label: {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                (Text(title).font(BarDS.bodyFont(13, weight: .medium))
                    + Text(sub).font(BarDS.bodyFont(11, weight: .regular)).foregroundColor(BarDS.Text.secondary))
                    .foregroundColor(BarDS.Text.primary)
                Spacer(minLength: 0)
                Text(value)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
            }
            .padding(.vertical, 8)
            .overlay(alignment: .bottom) {
                Rectangle().fill(Color.white.opacity(0.05)).frame(height: 0.5)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
        .opacity(lens == id ? 1 : 0.85)
    }
}

private struct BarTriageRouteView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        Group {
            if viewModel.positions.isEmpty {
                BarPlaceholderCard(
                    title: "Triage",
                    caption: "No open positions — nothing to triage."
                )
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Open positions")
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    ForEach(viewModel.positions) { p in
                        HStack {
                            Text(p.symbol)
                                .font(BarDS.monoFont(12, weight: .medium))
                                .foregroundColor(BarDS.Text.primary)
                            Spacer()
                            Text(p.direction)
                                .font(BarDS.bodyFont(11, weight: .regular))
                                .foregroundColor(BarDS.Text.secondary)
                        }
                        .padding(.vertical, 8)
                        BarDSDivider()
                    }
                }
                .padding(12)
                .background(BarDS.Fill.card)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
                )
            }
        }
    }
}
