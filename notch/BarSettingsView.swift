import Foundation
import SwiftUI

/// PLAN settings surface — broker, risk limits, behavior, notifications (four tabs).
struct BarSettingsView: View {
    @ObservedObject var viewModel: NotchViewModel

    private enum SettingsTab: String, CaseIterable {
        case broker, risk, behavior, notifications
    }

    private enum BehaviorArchetypeTab: String, CaseIterable {
        case intraday, scalper, swing, positional

        var traderArchetype: TraderArchetype? {
            switch self {
            case .intraday: return .intraday
            case .scalper: return .scalper
            case .swing: return .swing
            case .positional: return nil
            }
        }
    }

    private enum SyncPosture {
        case connected, connecting, degraded, offline

        init(brokerSyncClass: String?) {
            switch brokerSyncClass?.lowercased() ?? "" {
            case "synced": self = .connected
            case "syncing": self = .connecting
            case "stale": self = .degraded
            default: self = .offline
            }
        }
    }

    @State private var activeTab: SettingsTab = .broker
    @State private var clock: Date = Date()

    @State private var dailyLossLimit: String = ""
    @State private var weeklyLossLimit: String = ""
    @State private var marginCapPct: String = ""
    @State private var limitsLoadError: String?
    @State private var limitsSaveMessage: String?
    @State private var limitsSaveError: String?
    @State private var limitsBusy: Bool = false

    @State private var behaviorArchetype: BehaviorArchetypeTab = .intraday

    @State private var killSwitchNotif = true
    @State private var nakedWindowNotif = true
    @State private var compositeRedNotif = false
    @State private var morningBriefNotif = true
    @State private var swingCheckinNotif = true
    @State private var misWarningNotif = false
    @State private var coolingNotif = true

    private static let killSwitchDomains: [(label: String, url: String)] = [
        ("Kotak CIS", "cis.kotaksecurities.com"),
        ("Kotak Neo", "neo.kotaksecurities.com"),
        ("Kotak MIS", "mis.kotaksecurities.com"),
    ]

    private static let intradayWeights: [(String, Double)] = [
        ("Overtrading", 0.35),
        ("Disposition", 0.20),
        ("Herding / FOMO", 0.15),
        ("Sizing error", 0.10),
        ("Loss chasing", 0.10),
        ("Symbol drift", 0.05),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            settingsTabRow
            tabContent
        }
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .onAppear {
            loadNotificationPrefs()
            behaviorArchetype = mapBehaviorArchetype(from: viewModel.activeArchetype)
            Task { await fetchLossLimits() }
        }
        .onReceive(NotchOneSecondClock.publisher) { clock = $0 }
    }

    // MARK: - Tab row

    private var settingsTabRow: some View {
        HStack(spacing: 5) {
            BarTab(label: "Broker", active: activeTab == .broker) { activeTab = .broker }
            BarTab(label: "Risk limits", active: activeTab == .risk) { activeTab = .risk }
            BarTab(label: "Behavior", active: activeTab == .behavior) { activeTab = .behavior }
            BarTab(label: "Notifications", active: activeTab == .notifications) { activeTab = .notifications }
        }
        .padding(.bottom, 14)
    }

    @ViewBuilder
    private var tabContent: some View {
        switch activeTab {
        case .broker: brokerTab
        case .risk: riskTab
        case .behavior: behaviorTab
        case .notifications: notificationsTab
        }
    }

    // MARK: - Broker

    private var brokerTab: some View {
        VStack(alignment: .leading, spacing: 0) {
            BarSectionLabel(text: "Live connection")
            syncStatusBar
            brokerMetricsGrid
            connectedBrokerCard
            brokerActionRow

            Rectangle()
                .fill(BarDS.Border.divider)
                .frame(height: BarDS.borderThin)
                .padding(.vertical, 12)

            BarSectionLabel(text: "Kill switch domains")
            killSwitchDomainsCard
        }
    }

    private var syncPosture: SyncPosture {
        SyncPosture(brokerSyncClass: viewModel.brokerSyncClass)
    }

    private var syncStatusBar: some View {
        HStack(alignment: .center, spacing: 10) {
            HStack(spacing: 8) {
                syncStatusDot
                VStack(alignment: .leading, spacing: 2) {
                    Text(syncPrimaryLabel)
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    Text(syncSecondaryLabel)
                        .font(BarDS.bodyFont(BarDS.FontSize.chip, weight: .regular))
                        .foregroundColor(BarDS.Text.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 8)
            Text(currentTimeText)
                .font(BarDS.monoFont(BarDS.FontSize.chip, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        }
        .padding(.vertical, 13)
        .padding(.horizontal, 15)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
        .padding(.bottom, 8)
    }

    @ViewBuilder
    private var syncStatusDot: some View {
        switch syncPosture {
        case .connected:
            Circle()
                .fill(BarDS.Accent.teal)
                .frame(width: 8, height: 8)
        case .connecting, .degraded:
            SettingsPulsingDot(color: BarDS.Accent.amber, duration: 1.8)
        case .offline:
            SettingsPulsingDot(color: BarDS.Accent.red, duration: 0.9)
        }
    }

    private var syncPrimaryLabel: String {
        switch syncPosture {
        case .connected: return "Broker sync · live"
        case .connecting: return "Broker sync · connecting"
        case .degraded: return "Broker sync · degraded"
        case .offline: return "No broker sync"
        }
    }

    private var syncSecondaryLabel: String {
        switch syncPosture {
        case .connected:
            let tick = brokerPollSecondsAgo.map { "\($0)s" } ?? "—"
            return "\(brokerDisplayName) · last poll \(tick) ago · mirrors Station Brokers"
        case .connecting:
            return "\(brokerDisplayName) · Start in progress on Enforcer"
        case .degraded:
            let ago = brokerPollMinutesAgo.map { "\($0)m" } ?? "—"
            return "\(brokerDisplayName) · last good poll \(ago) ago — re-auth in Station if needed"
        case .offline:
            return "Not connected — Connect / Start broker once in Station Brokers"
        }
    }

    private var brokerPollSecondsAgo: Int? {
        guard let ms = viewModel.brokerSyncLastPollAtMs else { return lastSyncSecondsAgo }
        let date = Date(timeIntervalSince1970: Double(ms) / 1000.0)
        return max(0, Int(clock.timeIntervalSince(date).rounded(.down)))
    }

    private var brokerPollMinutesAgo: Int? {
        guard let secs = brokerPollSecondsAgo else { return nil }
        return max(0, secs / 60)
    }

    private var lastSyncSecondsAgo: Int? {
        guard let iso = viewModel.barLiveState?.lastSyncAt else { return nil }
        guard let date = parseISO8601(iso) else { return nil }
        return max(0, Int(clock.timeIntervalSince(date).rounded(.down)))
    }

    private var currentTimeText: String {
        let f = DateFormatter()
        f.dateFormat = "HH:mm:ss"
        return f.string(from: clock)
    }

    private var brokerMetricsGrid: some View {
        LazyVGrid(
            columns: [GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8)],
            spacing: 8,
        ) {
            BarMetric(label: "Latency", value: latencyText, valueColor: latencyColor, sub: "Last 60s avg")
            BarMetric(label: "Uptime today", value: "—", sub: "Since 09:15 AM")
        }
        .padding(.bottom, 8)
    }

    private var latencyText: String {
        guard syncPosture == .connected else { return "—" }
        return "—"
    }

    private var latencyColor: Color {
        BarDS.Text.primary
    }

    private var connectedBrokerCard: some View {
        HStack(alignment: .center, spacing: 10) {
            HStack(spacing: 10) {
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .fill(Color.white.opacity(0.06))
                    .frame(width: 32, height: 32)
                    .overlay(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .stroke(Color.white.opacity(0.10), lineWidth: BarDS.borderThin),
                    )
                    .overlay {
                        Text(brokerInitials)
                            .font(BarDS.bodyFont(10, weight: .medium))
                            .foregroundColor(BarDS.Text.secondary)
                    }
                VStack(alignment: .leading, spacing: 2) {
                    Text(brokerDisplayName)
                        .font(BarDS.bodyFont(13, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    Text(brokerMetaLine)
                        .font(BarDS.bodyFont(11, weight: .regular))
                        .foregroundColor(BarDS.Text.hint)
                }
            }
            Spacer(minLength: 8)
            brokerConnectionBadge
        }
        .padding(.vertical, 12)
        .padding(.horizontal, 14)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
        .padding(.bottom, 8)
    }

    private var brokerDisplayName: String {
        NotchViewModel.brokerDisplayName(
            forSlug: viewModel.activeBrokerSlug ?? viewModel.barProtectiveBrokerSlug
        )
    }

    private var brokerInitials: String {
        let parts = brokerDisplayName.split(separator: " ")
        if parts.count >= 2 {
            return String(parts[0].prefix(1) + parts[1].prefix(1)).uppercased()
        }
        return String(brokerDisplayName.prefix(2)).uppercased()
    }

    private var brokerMetaLine: String {
        let sync = viewModel.brokerSyncClass.lowercased()
        if sync == "synced" { return "Connected via Station · sync live" }
        if sync == "syncing" { return "Connecting via Station Enforcer" }
        if sync == "stale" { return "Session stale · re-auth may be required in Station" }
        return "Connect once in Station Brokers — Notch only mirrors status"
    }

    private var brokerConnectionBadge: some View {
        let (title, fg, bg): (String, Color, Color) = {
            switch syncPosture {
            case .connected:
                return ("Connected", BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.12))
            case .connecting:
                return ("Connecting", BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.12))
            case .degraded:
                return ("Degraded", BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.12))
            case .offline:
                return ("Not connected", BarDS.Accent.red, BarDS.Accent.red.opacity(0.12))
            }
        }()
        return Text(title)
            .font(BarDS.bodyFont(10, weight: .medium))
            .foregroundColor(fg)
            .padding(.vertical, 4)
            .padding(.horizontal, 8)
            .background(bg)
            .clipShape(Capsule())
    }

    private var brokerActionRow: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                if NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: viewModel.brokerSyncClass) {
                    brokerActionButton(title: "Open Station Brokers", icon: "macwindow", danger: false) {
                        viewModel.requestOpenBrokerConnect()
                    }
                } else {
                    brokerActionButton(title: "Re-auth token", icon: "arrow.triangle.2.circlepath", danger: false) {
                        viewModel.requestOpenBrokerReauth()
                    }
                    brokerActionButton(title: "Test connection", icon: "antenna.radiowaves.left.and.right", danger: false) {
                        Task { await viewModel.testBrokerConnection() }
                    }
                    brokerActionButton(title: "Disconnect", icon: nil, danger: true) {
                        Task { await viewModel.disconnectBrokerSync() }
                    }
                }
            }

            if NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: viewModel.brokerSyncClass) {
                Text("One Connect in Station. Notch never logs into the broker itself.")
                    .font(BarDS.bodyFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                    .fixedSize(horizontal: false, vertical: true)
            }

            if viewModel.brokerActionBusy {
                ProgressView()
                    .controlSize(.small)
            }
            if let msg = viewModel.brokerActionResultMessage, !msg.isEmpty {
                Text(msg)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.teal)
            }
            if let err = viewModel.brokerActionError, !err.isEmpty {
                Text(err)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
            }
        }
        .padding(.bottom, 4)
    }

    private func brokerActionButton(
        title: String,
        icon: String?,
        danger: Bool,
        action: @escaping () -> Void,
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 4) {
                if let icon {
                    Image(systemName: icon)
                        .font(.system(size: 10, weight: .medium))
                }
                Text(title)
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            .foregroundColor(danger ? BarDS.Accent.red : BarDS.Text.secondary)
            .frame(maxWidth: .infinity)
            .padding(7)
            .background(danger ? BarDS.Accent.red.opacity(0.06) : Color.white.opacity(0.04))
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .stroke(Color.white.opacity(danger ? 0.12 : 0.09), lineWidth: BarDS.borderThin),
            )
        }
        .buttonStyle(.plain)
    }

    private var killSwitchDomainsCard: some View {
        BarCard {
            Text("DNS sinkhole targets — blocked when kill switch fires.")
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 8)
            ForEach(Array(Self.killSwitchDomains.enumerated()), id: \.offset) { idx, row in
                HStack {
                    Text(row.label)
                        .font(BarDS.bodyFont(11, weight: .regular))
                        .foregroundColor(BarDS.Text.hint)
                    Spacer(minLength: 8)
                    Text(row.url)
                        .font(BarDS.monoFont(11, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                }
                .padding(.vertical, 7)
                if idx < Self.killSwitchDomains.count - 1 {
                    Rectangle()
                        .fill(Color.white.opacity(0.05))
                        .frame(height: BarDS.borderThin)
                }
            }
        }
    }

    // MARK: - Risk limits

    private var riskTab: some View {
        VStack(alignment: .leading, spacing: 0) {
            BarSectionLabel(text: "Current usage")
            usageProgressCard

            if let limitsLoadError, !limitsLoadError.isEmpty {
                Text(limitsLoadError)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
                    .padding(.bottom, 6)
            }

            BarSectionLabel(text: "Edit limits")
            editLimitsCard

            BarSectionLabel(text: "Notch chip")
            notchChipCatalogCard

            BarNonNegotiableCard(
                label: "Kill switch rule",
                text: "Arms automatically when daily loss exceeds limit or weekly loss exceeds limit. Requires manual reset.",
                kind: .amber,
            )
        }
    }

    private var usageProgressCard: some View {
        BarCard {
            BarProgressBlock(
                label: "Daily loss limit",
                valueText: dailyUsageText,
                pct: dailyUsagePct,
            )
            BarProgressBlock(
                label: "Margin utilisation",
                valueText: marginUsageText,
                pct: marginUsagePct,
            )
            BarProgressBlock(
                label: "Weekly loss limit",
                valueText: weeklyUsageText,
                pct: weeklyUsagePct,
            )
        }
    }

    private var parsedDailyLimit: Double? { Double(dailyLossLimit.trimmingCharacters(in: .whitespaces)) }
    private var parsedWeeklyLimit: Double? { Double(weeklyLossLimit.trimmingCharacters(in: .whitespaces)) }
    private var parsedMarginCap: Double? { Double(marginCapPct.trimmingCharacters(in: .whitespaces)) }

    private var dailyUsed: Double {
        viewModel.barLiveState?.composite?.worstCase ?? 0
    }

    private var weeklyUsed: Double {
        let pnl = viewModel.barLiveState?.weeklyPnL ?? 0
        return pnl < 0 ? abs(pnl) : 0
    }

    private var marginUsedPct: Double {
        viewModel.barLiveState?.composite?.budgetPct ?? 0
    }

    private var dailyUsagePct: Double {
        guard let limit = parsedDailyLimit, limit > 0 else { return 0 }
        return min(1, max(0, dailyUsed / limit))
    }

    private var weeklyUsagePct: Double {
        guard let limit = parsedWeeklyLimit, limit > 0 else { return 0 }
        return min(1, max(0, weeklyUsed / limit))
    }

    private var marginUsagePct: Double {
        guard let cap = parsedMarginCap, cap > 0 else { return 0 }
        return min(1, max(0, marginUsedPct / cap))
    }

    private var dailyUsageText: String {
        guard let limit = parsedDailyLimit else { return "—" }
        return String(format: "₹%.0f of ₹%.0f", dailyUsed, limit)
    }

    private var weeklyUsageText: String {
        guard let limit = parsedWeeklyLimit else { return "—" }
        return String(format: "₹%.0f of ₹%.0f", weeklyUsed, limit)
    }

    private var marginUsageText: String {
        guard let cap = parsedMarginCap else { return "—" }
        return String(format: "%.0f%% of %.0f%% cap", marginUsedPct, cap)
    }

    private var editLimitsCard: some View {
        BarCard {
            BarSectionLabel(text: "Daily loss limit ₹")
            BarInputField(placeholder: "e.g. 5000", text: $dailyLossLimit)
            BarSectionLabel(text: "Weekly loss limit ₹")
            BarInputField(placeholder: "e.g. 20000", text: $weeklyLossLimit)
            BarSectionLabel(text: "Margin utilisation cap %")
            BarInputField(placeholder: "e.g. 50", text: $marginCapPct)

            if let limitsSaveMessage, !limitsSaveMessage.isEmpty {
                Text(limitsSaveMessage)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.teal)
            }
            if let limitsSaveError, !limitsSaveError.isEmpty {
                Text(limitsSaveError)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
            }

            BarBigButton(
                label: limitsBusy ? "Saving…" : "Save limits →",
                style: .primary,
            ) {
                Task { await saveLossLimits() }
            }
            .disabled(limitsBusy)
            .opacity(limitsBusy ? 0.5 : 1)
        }
    }

    private var notchChipCatalogCard: some View {
        BarCard {
            Text("From the system. Impact stays. Cap 4 extras. No LTP.")
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 10)
            catalogRow(
                id: NotchChipCatalogState.impactId,
                title: "Account impact",
                meta: "Pinned names vs daily limit — locked",
                locked: true,
            )
            ForEach(viewModel.chipCatalog.seenSymbols, id: \.self) { symbol in
                catalogRow(
                    id: symbol,
                    title: symbol,
                    meta: "Live P&L on the account",
                    locked: false,
                )
            }
            ForEach(NotchChipSystemSlot.allCases, id: \.self) { slot in
                catalogRow(
                    id: slot.rawValue,
                    title: slot.title,
                    meta: slot.meta,
                    locked: false,
                )
            }
        }
    }

    private func catalogRow(id: String, title: String, meta: String, locked: Bool) -> some View {
        let on = viewModel.chipCatalog.isOn(id)
        return HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(BarDS.bodyFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                Text(meta)
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
            }
            Spacer(minLength: 8)
            Button {
                viewModel.toggleChipExtra(id)
            } label: {
                Text(locked ? "Locked" : on ? "On" : "Add")
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(locked || on ? BarDS.Accent.teal : BarDS.Text.secondary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .background(
                        Capsule(style: .continuous)
                            .fill(locked || on ? BarDS.Accent.teal.opacity(0.16) : Color.white.opacity(0.08))
                    )
            }
            .buttonStyle(.plain)
            .disabled(locked)
            .opacity(locked ? 0.55 : 1)
        }
        .padding(.vertical, 6)
    }

    // MARK: - Behavior

    private var behaviorTab: some View {
        VStack(alignment: .leading, spacing: 0) {
            BarSectionLabel(text: "Archetype")
            BarCard {
                Text("Controls signal weights, intervention thresholds, and which pre-trade form you see.")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, 10)
                HStack(spacing: 5) {
                    ForEach(BehaviorArchetypeTab.allCases, id: \.self) { tab in
                        BarTab(label: tab.rawValue.capitalized, active: behaviorArchetype == tab) {
                            behaviorArchetype = tab
                            if let arch = tab.traderArchetype {
                                viewModel.setUserDeclarationArchetype(arch)
                            }
                        }
                    }
                }
            }

            BarSectionLabel(text: "Signal weights")
            BarCard {
                ForEach(Array(Self.intradayWeights.enumerated()), id: \.offset) { _, row in
                    BarProgressBlock(
                        label: row.0,
                        valueText: String(format: "%.2f", row.1),
                        pct: row.1,
                    )
                }
                Text("Weights locked to research defaults. Phase B (Optuna) unlocks after 500 trades per archetype.")
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.top, 4)
            }

            BarSectionLabel(text: "Score multipliers")
            BarPlanRowsCard(rows: [
                ("F&O instruments", "1.5×", BarDS.Accent.amber),
                ("Wednesday", "1.35×", BarDS.Accent.amber),
                ("Intraday", "1.4×", BarDS.Accent.amber),
            ])
        }
    }

    // MARK: - Notifications

    private var notificationsTab: some View {
        VStack(alignment: .leading, spacing: 0) {
            BarSectionLabel(text: "Intervention alerts")
            BarToggleRow(
                label: "Kill switch fired",
                sub: "macOS notification when kill switch arms",
                isOn: $killSwitchNotif,
            )
            .onChange(of: killSwitchNotif) { _, v in saveNotif("kill_switch_fired", v) }
            BarToggleRow(
                label: "Naked window alert",
                sub: "No SL placed within 5s of fill",
                isOn: $nakedWindowNotif,
            )
            .onChange(of: nakedWindowNotif) { _, v in saveNotif("naked_window", v) }
            BarToggleRow(
                label: "Composite RED alert",
                sub: "Risk score exceeds 90% of daily limit",
                isOn: $compositeRedNotif,
            )
            .onChange(of: compositeRedNotif) { _, v in saveNotif("composite_red", v) }

            BarSectionLabel(text: "Session reminders")
            BarToggleRow(
                label: "Morning brief ready",
                sub: "9:00 AM daily before market open",
                isOn: $morningBriefNotif,
            )
            .onChange(of: morningBriefNotif) { _, v in saveNotif("morning_brief", v) }
            BarToggleRow(
                label: "Swing daily check-in",
                sub: "While swing position is open",
                isOn: $swingCheckinNotif,
            )
            .onChange(of: swingCheckinNotif) { _, v in saveNotif("swing_checkin", v) }
            BarToggleRow(
                label: "MIS auto-square warning",
                sub: "15 min before 3:15 PM auto-square",
                isOn: $misWarningNotif,
            )
            .onChange(of: misWarningNotif) { _, v in saveNotif("mis_warning", v) }

            BarSectionLabel(text: "Cooling window")
            BarToggleRow(
                label: "Post-loss cooling",
                sub: "Block re-entry for 5 min after a loss",
                isOn: $coolingNotif,
            )
            .onChange(of: coolingNotif) { _, v in saveNotif("cooling", v) }
        }
    }

    // MARK: - API

    private func fetchLossLimits() async {
        limitsLoadError = nil
        guard let url = lossLimitsURL() else {
            limitsLoadError = "Web base URL not configured."
            return
        }
        guard !viewModel.daemonSecret.isEmpty else {
            limitsLoadError = "Daemon secret not configured."
            return
        }
        let req = daemonBarRequest(url: url, method: "GET", body: nil)
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard code == 200 else {
                limitsLoadError = "Could not load limits (\(code))."
                return
            }
            guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let limits = json["limits"] as? [String: Any]
            else {
                limitsLoadError = "Unexpected limits response."
                return
            }
            if let d = limits["dailyLossLimit"] as? String, !d.isEmpty { dailyLossLimit = d }
            else if let n = limits["dailyLossLimit"] as? NSNumber { dailyLossLimit = n.stringValue }
            viewModel.applyDailyLossLimit(parsedDailyLimit)
            if let w = limits["weeklyLossLimit"] as? String, !w.isEmpty { weeklyLossLimit = w }
            else if let n = limits["weeklyLossLimit"] as? NSNumber { weeklyLossLimit = n.stringValue }
            if let m = limits["marginUtilizationCapPct"] as? String, !m.isEmpty { marginCapPct = m }
            else if let n = limits["marginUtilizationCapPct"] as? NSNumber { marginCapPct = n.stringValue }
        } catch {
            limitsLoadError = error.localizedDescription
        }
    }

    private func saveLossLimits() async {
        limitsSaveMessage = nil
        limitsSaveError = nil
        guard let daily = parsedDailyLimit, daily > 0,
              let weekly = parsedWeeklyLimit, weekly > 0,
              let margin = parsedMarginCap, margin > 0, margin <= 100
        else {
            limitsSaveError = "Enter valid daily, weekly, and margin cap values."
            return
        }
        guard let url = lossLimitsURL() else {
            limitsSaveError = "Web base URL not configured."
            return
        }
        limitsBusy = true
        defer { limitsBusy = false }
        let body: [String: Any] = [
            "daily_loss_limit": daily,
            "weekly_loss_limit": weekly,
            "margin_utilization_cap_pct": margin,
            "acknowledge_bar_features_and_loss_limits": true,
        ]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else {
            limitsSaveError = "Could not encode request."
            return
        }
        let req = daemonBarRequest(url: url, method: "POST", body: payload)
        do {
            let (data, resp) = try await URLSession.shared.data(for: req)
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200 ... 299).contains(code) else {
                limitsSaveError = AgentHTTPErrorPresentation.message(httpStatus: code, body: data)
                return
            }
            limitsSaveMessage = "Saved"
            viewModel.applyDailyLossLimit(daily)
            await fetchLossLimits()
        } catch {
            limitsSaveError = error.localizedDescription
        }
    }

    private func lossLimitsURL() -> URL? {
        // Station-hosted Notch: webBase is agent loopback — use daemon bar proxy → Console.
        return URL(string: "http://127.0.0.1:\(viewModel.daemonPort)/api/daemon/bar/profile/loss-limits")
    }

    private func daemonBarRequest(url: URL, method: String, body: Data?) -> URLRequest {
        let payload = body ?? Data()
        let path = url.path.isEmpty ? "/" : url.path
        var r = StationWireClient.signedRequest(
            method: method,
            path: path,
            body: payload,
            daemonSecret: viewModel.daemonSecret
        )
        r.url = url
        r.setValue("notch-settings", forHTTPHeaderField: "x-daemon-source")
        if body != nil {
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
            r.httpBody = body
        }
        return r
    }

    // MARK: - Notifications persistence

    private func loadNotificationPrefs() {
        let d = UserDefaults.standard
        killSwitchNotif = d.object(forKey: notifKey("kill_switch_fired")) as? Bool ?? true
        nakedWindowNotif = d.object(forKey: notifKey("naked_window")) as? Bool ?? true
        compositeRedNotif = d.object(forKey: notifKey("composite_red")) as? Bool ?? false
        morningBriefNotif = d.object(forKey: notifKey("morning_brief")) as? Bool ?? true
        swingCheckinNotif = d.object(forKey: notifKey("swing_checkin")) as? Bool ?? true
        misWarningNotif = d.object(forKey: notifKey("mis_warning")) as? Bool ?? false
        coolingNotif = d.object(forKey: notifKey("cooling")) as? Bool ?? true
    }

    private func saveNotif(_ key: String, _ value: Bool) {
        UserDefaults.standard.set(value, forKey: notifKey(key))
    }

    private func notifKey(_ key: String) -> String { "notif_\(key)" }

    // MARK: - Helpers

    private func mapBehaviorArchetype(from arch: TraderArchetype) -> BehaviorArchetypeTab {
        switch arch {
        case .intraday: return .intraday
        case .scalper: return .scalper
        case .swing: return .swing
        }
    }

    private func parseISO8601(_ raw: String) -> Date? {
        let frac = ISO8601DateFormatter()
        frac.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let d = frac.date(from: raw) { return d }
        let coarse = ISO8601DateFormatter()
        coarse.formatOptions = [.withInternetDateTime]
        return coarse.date(from: raw)
    }
}

private struct SettingsPulsingDot: View {
    let color: Color
    let duration: Double
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: reduceMotion)) { ctx in
            let t = ctx.date.timeIntervalSinceReferenceDate
            let amp = 0.5 + 0.5 * sin(t * (2 * .pi / duration))
            let op = reduceMotion ? 1.0 : (0.3 + 0.7 * amp)
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
                .opacity(op)
        }
    }
}
