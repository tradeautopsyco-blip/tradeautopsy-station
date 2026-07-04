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

    var isSessionGroup: Bool {
        switch self {
        case .morning, .pretrade, .live, .posttrade: return true
        default: return false
        }
    }

    var sfSymbol: String {
        switch self {
        case .morning: return "sun.max"
        case .pretrade: return "checkmark.clipboard"
        case .live: return "bolt"
        case .posttrade: return "checklist"
        case .escrow: return "shield.fill"
        case .patterns: return "brain"
        case .fidelity: return "chart.bar.fill"
        case .triage: return "arrow.down.circle"
        case .settings: return "gear"
        }
    }
}

struct BarNotchShell: View {
    @ObservedObject var viewModel: NotchViewModel
    private let externalActiveScreen: Binding<BarNotchScreen>?
    @State private var localActiveScreen: BarNotchScreen = .morning
    @AppStorage("notch.planMorningBriefConsumed") private var morningBriefConsumed: Bool = false
    @State private var hoveredSession: BarNotchScreen?

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
            Task { await viewModel.fetchBarLiveState() }
        }
        .onChange(of: viewModel.barSurfacePhase) { _, _ in
            guard externalActiveScreen == nil else { return }
            syncActiveScreenFromPhase(animated: true)
        }
        .onChange(of: viewModel.morningBrief != nil) { _, has in
            if !has { morningBriefConsumed = false }
        }
        .onChange(of: activeScreen.wrappedValue) { _, new in
            syncDeclarationFormFlagToActiveScreen(screen: new)
        }
    }

    private func syncDeclarationFormFlagToActiveScreen(screen: BarNotchScreen? = nil) {
        let s = screen ?? activeScreen.wrappedValue
        viewModel.showingDeclarationForm = (s == .pretrade)
    }

    // MARK: - Phase → sidebar (does not fight user while on auxiliary tabs)

    private func syncActiveScreenFromPhase(animated: Bool) {
        let next: BarNotchScreen
        switch viewModel.barSurfacePhase {
        case .declaration: next = .pretrade
        case .livePlan, .armed: next = .live
        case .debrief: next = .posttrade
        }
        guard next != activeScreen.wrappedValue else { return }
        if animated {
            withAnimation(.easeInOut(duration: 0.15)) {
                activeScreen.wrappedValue = next
            }
        } else {
            activeScreen.wrappedValue = next
        }
    }

    // MARK: - Sidebar (192px)

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 1) {
                Text("TradeAutopsy")
                    .font(BarDS.bodyFont(13, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .kerning(-0.01 * 13)
                Text("Behavioral trading OS")
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .padding(.top, 1)
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(BarDS.Border.section)
                    .frame(height: BarDS.borderThin)
            }

            BarSectionLabel(text: "SESSION")
                .padding(.horizontal, 10)
                .padding(.top, 0)

            sessionNavButton(.morning, badge: morningUnreadBadgeText)
            sessionNavButton(.pretrade, badge: nil)
            sessionNavButton(.live, badge: liveInterventionBadgeText)
            sessionNavButton(.posttrade, badge: nil)

            BarSectionLabel(text: "ANALYSIS")
                .padding(.horizontal, 10)

            navButton(.escrow, badge: nil)
            navButton(.patterns, badge: nil)
            navButton(.fidelity, badge: nil)
            navButton(.triage, badge: nil)

            Spacer(minLength: 0)

            Rectangle()
                .fill(BarDS.Border.section)
                .frame(height: BarDS.borderThin)
            archetypePill
                .padding(.horizontal, 8)
                .padding(.vertical, 10)
        }
        .frame(width: 192, alignment: .topLeading)
        .background(BarDS.Fill.sidebar)
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

    private var liveInterventionBadgeText: String? {
        let n = viewModel.barLiveState?.activeInterventions.count ?? 0
        return n > 0 ? "!" : nil
    }

    private func navButton(_ screen: BarNotchScreen, badge: String?) -> some View {
        let isActive = activeScreen.wrappedValue == screen
        let isHover = hoveredSession == screen
        return Button {
            if screen == .morning {
                morningBriefConsumed = true
            }
            activeScreen.wrappedValue = screen
        } label: {
            HStack(spacing: 8) {
                Image(systemName: screen.sfSymbol)
                    .font(.system(size: 14, weight: .regular))
                    .foregroundColor(navPrimary(isActive: isActive, isHover: isHover))
                    .opacity(isActive ? 1 : 0.7)
                Text(screen.rawValue)
                    .font(BarDS.bodyFont(12, weight: isActive ? .medium : .regular))
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
            .padding(.vertical, 6)
            .padding(.horizontal, 10)
            .background(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .fill(navBackground(isActive: isActive, isHover: isHover))
            )
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
        }
        .buttonStyle(.plain)
        .onHover { isHover in
            if isHover { hoveredSession = screen }
            else if hoveredSession == screen { hoveredSession = nil }
        }
    }

    /// SESSION group: SF Symbol + hex icon colors (matches spec); label uses same row styling as `navButton`.
    private func sessionNavButton(_ screen: BarNotchScreen, badge: String?) -> some View {
        let isActive = activeScreen.wrappedValue == screen
        let isHover = hoveredSession == screen
        return Button {
            if screen == .morning {
                morningBriefConsumed = true
            }
            activeScreen.wrappedValue = screen
        } label: {
            HStack(spacing: 8) {
                Image(systemName: screen.sfSymbol)
                    .font(.system(size: 14))
                    .foregroundColor(isActive ? Color(hex: "#ededed") : Color(hex: "#666666"))
                    .opacity(isActive ? 1.0 : 0.7)
                Text(screen.rawValue)
                    .font(BarDS.bodyFont(12, weight: isActive ? .medium : .regular))
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
            .padding(.vertical, 6)
            .padding(.horizontal, 10)
            .background(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .fill(navBackground(isActive: isActive, isHover: isHover))
            )
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
        }
        .buttonStyle(.plain)
        .onHover { isHover in
            if isHover { hoveredSession = screen }
            else if hoveredSession == screen { hoveredSession = nil }
        }
    }

    private func navBackground(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return Color.white.opacity(0.07) }
        if isHover { return Color.white.opacity(0.04) }
        return .clear
    }

    private func navPrimary(isActive: Bool, isHover: Bool) -> Color {
        if isActive { return BarDS.Text.primary }
        if isHover { return Color(hex: "#aaaaaa") }
        return Color(hex: "#666666")
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

            Image(systemName: "gearshape")
                .font(.system(size: 14))
                .foregroundColor(
                    activeScreen.wrappedValue == .settings ? Color(hex: "#00e5c0") : Color(hex: "#888888")
                )
                .frame(width: 36, height: 36)
                .contentShape(Rectangle())
                .onTapGesture {
                    activeScreen.wrappedValue = .settings
                }
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
                .frame(maxWidth: .infinity, alignment: .leading)

            Text("score \(String(format: "%.2f", viewModel.compositeScore))")
                .font(BarDS.monoFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)

            if let mult = viewModel.barBehavioralMultiplierLabel, !mult.isEmpty {
                Text(mult)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.amber)
            }

            brokerConnectionPill

            statePill

            Button {
                viewModel.showingDeclarationForm = true
                activeScreen.wrappedValue = .pretrade
            } label: {
                Text("+ New trade")
                    .font(BarDS.bodyFont(12, weight: .medium))
                    .foregroundColor(BarDS.Fill.sidebar)
                    .padding(.vertical, 5)
                    .padding(.horizontal, 12)
                    .background(BarDS.Text.primary)
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            }
            .buttonStyle(.plain)
        }
        .padding(.vertical, 11)
        .padding(.horizontal, 16)
        .background(BarDS.Fill.sidebar)
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

    private var brokerPillStyle: (dot: Color, label: String, titleColor: Color, bg: Color) {
        let sync = viewModel.barLiveState?.syncState.uppercased() ?? ""
        switch sync {
        case "GREEN":
            return (
                BarDS.Accent.teal,
                "Kotak Neo · live",
                BarDS.Accent.teal,
                BarDS.Accent.teal.opacity(0.10),
            )
        case "AMBER":
            return (
                BarDS.Accent.amber,
                "Kotak Neo · degraded",
                BarDS.Accent.amber,
                BarDS.Accent.amber.opacity(0.10),
            )
        default:
            return (
                BarDS.Accent.red,
                "No broker · offline",
                BarDS.Accent.red,
                BarDS.Accent.red.opacity(0.10),
            )
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
        case .live:
            let interventions = viewModel.barLiveState?.activeInterventions.count ?? 0
            if interventions > 0 {
                return (
                    BarDS.Accent.amber,
                    "LIVE · ACTION",
                    BarDS.Accent.amber,
                    BarDS.Accent.amber.opacity(0.12),
                )
            }
            return behavioralPillStyle
        case .posttrade:
            return (
                BarDS.Accent.teal,
                "DEBRIEF",
                BarDS.Accent.teal,
                BarDS.Accent.teal.opacity(0.10),
            )
        case .pretrade:
            if viewModel.compositeScore >= 0.35 {
                return (
                    BarDS.Accent.amber,
                    "PRE · ELEVATED",
                    BarDS.Accent.amber,
                    BarDS.Accent.amber.opacity(0.12),
                )
            }
            return (
                BarDS.Accent.teal,
                "PRE · CALM",
                BarDS.Accent.teal,
                BarDS.Accent.teal.opacity(0.10),
            )
        default:
            return behavioralPillStyle
        }
    }

    private var behavioralPillStyle: (dot: Color, title: String, titleColor: Color, bg: Color) {
        let raw = viewModel.behavioralState.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        if raw.contains("DANGER") || raw == "HIGH" || viewModel.compositeScore >= 0.45 {
            return (
                BarDS.Accent.red,
                "HIGH RISK",
                BarDS.Accent.red,
                BarDS.Accent.red.opacity(0.12)
            )
        }
        if raw == "CAUTION" || viewModel.compositeScore >= 0.25 {
            return (
                BarDS.Accent.amber,
                "CAUTION",
                BarDS.Accent.amber,
                BarDS.Accent.amber.opacity(0.12)
            )
        }
        return (
            BarDS.Accent.teal,
            "CALM",
            BarDS.Accent.teal,
            BarDS.Accent.teal.opacity(0.10)
        )
    }

    // MARK: - Header strip (loading / errors) — preserves BarCircuitPanelView diagnostics

    @ViewBuilder
    private var loadingOrErrorStrip: some View {
        VStack(alignment: .leading, spacing: 0) {
            if viewModel.barStateLoading {
                HStack {
                    Spacer(minLength: 0)
                    ProgressView()
                        .controlSize(.small)
                        .scaleEffect(0.75)
                    Spacer(minLength: 0)
                }
                .padding(.vertical, 6)
            }
            if let err = viewModel.barStateError, !err.isEmpty {
                Text(err)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
                    .padding(.horizontal, 16)
                    .padding(.bottom, 6)
            }
        }
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
            BarPlaceholderCard(
                title: "Patterns",
                caption: "Building patterns — need 50+ trades"
            )
        case .fidelity:
            BarFidelityRouteView(viewModel: viewModel)
        case .triage:
            BarTriageRouteView(viewModel: viewModel)
        case .settings:
            BarSettingsView(viewModel: viewModel)
        }
    }

    @ViewBuilder
    private var liveBody: some View {
        switch viewModel.barSurfacePhase {
        case .livePlan:
            ScrollView {
                BarPlanStateView(viewModel: viewModel, embedEscrow: activeScreen.wrappedValue != .escrow)
            }
            .scrollIndicators(.hidden)
        case .armed:
            if viewModel.barLiveState != nil {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        BarPlanStateView(viewModel: viewModel, embedEscrow: activeScreen.wrappedValue != .escrow)
                        planPanelDivider
                        barArmedWaiting
                    }
                }
                .scrollIndicators(.hidden)
            } else {
                barArmedWaiting
            }
        default:
            ScrollView {
                BarPlanStateView(viewModel: viewModel, embedEscrow: activeScreen.wrappedValue != .escrow)
            }
            .scrollIndicators(.hidden)
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

private struct BarFidelityRouteView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let pct = viewModel.barLiveState?.escrowMatchReport?.fidelityPct, pct.isFinite {
                BarProgressBlock(
                    label: "Plan fidelity (escrow)",
                    valueText: String(format: "%.0f%%", pct),
                    pct: min(1, max(0, pct / 100))
                )
            } else {
                BarPlaceholderCard(
                    title: "Fidelity score",
                    caption: "Escrow match data loads after your next reconciled trade."
                )
            }
            BarProgressBlock(
                label: "Composite behavioral score",
                valueText: String(format: "%.2f", viewModel.compositeScore),
                pct: min(1, max(0, viewModel.compositeScore))
            )
        }
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
