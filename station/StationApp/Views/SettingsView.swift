import Notch
import SwiftUI

public struct SettingsView: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @ObservedObject private var deskRules: DeskRulesStore
    @ObservedObject private var demoDesk: DemoDeskStore
    @State private var floorText = ""
    @State private var meanText = ""
    @State private var tripsText = ""
    @FocusState private var focusedLimit: LimitField?

    private enum LimitField: Hashable {
        case floor, mean, trips
    }

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
        self.deskRules = coordinator.deskRulesStore
        self.demoDesk = coordinator.demoDeskStore
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Settings")
                        .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
                    Text("This Mac · limits sync to the Notch · they do not fire Kill")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                }

                generalGroup
                journalCaptureGroup
                riskLimitsGroup
                notchGroup
                softwareUpdatesGroup
                privacyGroup
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(StationDS.Fill.appPanel)
        .onAppear { seedLimitFields() }
        .onDisappear { commitLimit(focusedLimit) }
        .onChange(of: focusedLimit) { old, _ in
            commitLimit(old)
        }
    }

    private var generalGroup: some View {
        settingsGroup(title: "General") {
            inset {
                toggleRow(
                    title: "Launch at login",
                    caption: "Start Station when this Mac wakes to the desk",
                    isOn: Binding(
                        get: { coordinator.launchAtLoginEnabled },
                        set: { _ in coordinator.toggleLaunchAtLogin() }
                    )
                )
                appearanceRow
                toggleRow(
                    title: "Demo data",
                    caption: "Paints Today from a labeled first-pair fixture. Not live. Does not fire Kill.",
                    isOn: Binding(
                        get: { demoDesk.demoEnabled },
                        set: { coordinator.setDemoEnabled($0) }
                    )
                )
            }
            DeviceLoginView(viewModel: coordinator.deviceLoginViewModel)
                .padding(.top, 8)
        }
    }

    private var appearanceRow: some View {
        HStack(alignment: .center, spacing: 12) {
            Text("Appearance")
                .font(StationDS.bodyFont(17))
                .foregroundStyle(StationDS.Text.primary)
            Spacer(minLength: 0)
            Text("System")
                .font(StationDS.bodyFont(17))
                .foregroundStyle(StationDS.Text.muted)
        }
        .frame(minHeight: 44)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    private var journalCaptureGroup: some View {
        settingsGroup(
            title: "Journal capture",
            footer: "Toolbar captures queue on this Mac until Console ACKs. Named reasons below are safe to share — no API keys."
        ) {
            inset {
                CaptureOutboxSettingsSection()
            }
        }
    }

    private var riskLimitsGroup: some View {
        settingsGroup(
            title: "Risk limits",
            footer: "These numbers sync to the Notch. They do not fire Kill. Stop pauses sync. Kill is the Notch overlay."
        ) {
            inset {
                limitRow(
                    title: "Daily floor",
                    caption: usedTodayCaption,
                    text: $floorText,
                    field: .floor,
                    placeholder: "—"
                )
                limitRow(
                    title: "Mean loss",
                    caption: nil,
                    text: $meanText,
                    field: .mean,
                    placeholder: "—"
                )
                limitRow(
                    title: "Max round trips",
                    caption: nil,
                    text: $tripsText,
                    field: .trips,
                    placeholder: "—"
                )
            }
        }
    }

    private var notchGroup: some View {
        settingsGroup(title: "Notch") {
            inset {
                toggleRow(
                    title: "Hide Notch",
                    caption: "⌥Space still summons it when you need the hands",
                    isOn: Binding(
                        get: { deskRules.hideNotch },
                        set: { coordinator.setHideNotch($0) }
                    )
                )
                Button(action: { coordinator.toggleNotch() }) {
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 1) {
                            Text("Open Notch")
                                .font(StationDS.bodyFont(17))
                                .foregroundStyle(StationDS.Text.primary)
                            Text("Live circuit · declare · kill path")
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                                .foregroundStyle(StationDS.Text.secondary)
                        }
                        Spacer(minLength: 0)
                        Image(systemName: "chevron.right")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(StationDS.Text.muted)
                    }
                    .frame(minHeight: 44)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
    }

    private var softwareUpdatesGroup: some View {
        settingsGroup(
            title: "Software updates",
            footer: "Checks https://updates.tradeautopsy.in for signed builds. Ad-hoc dev installs do not receive updates until you install a notarized release."
        ) {
            inset {
                toggleRow(
                    title: "Automatically check for updates",
                    caption: "",
                    isOn: Binding(
                        get: { coordinator.automaticallyChecksForUpdates },
                        set: { coordinator.setAutomaticallyChecksForUpdates($0) }
                    )
                )
                toggleRow(
                    title: "Automatically download updates",
                    caption: "",
                    isOn: Binding(
                        get: { coordinator.automaticallyDownloadsUpdates },
                        set: { coordinator.setAutomaticallyDownloadsUpdates($0) }
                    )
                )
                Button(action: { coordinator.checkForSoftwareUpdates() }) {
                    HStack(spacing: 12) {
                        Text("Check for Updates")
                            .font(StationDS.bodyFont(17))
                            .foregroundStyle(
                                coordinator.canCheckForSoftwareUpdates
                                    ? StationDS.Text.primary
                                    : StationDS.Text.muted
                            )
                        Spacer(minLength: 0)
                    }
                    .frame(minHeight: 44)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled(!coordinator.canCheckForSoftwareUpdates)
            }
        }
    }

    private var privacyGroup: some View {
        settingsGroup(
            title: "Privacy",
            footer: "Patterns, escrow, and fidelity stay Console — they are not Settings, and they are not Backend Box."
        ) {
            inset {
                Button(action: openAPIKeys) {
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 1) {
                            Text("API keys")
                                .font(StationDS.bodyFont(17))
                                .foregroundStyle(StationDS.Text.primary)
                            Text("Sit next to the broker they affect")
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                                .foregroundStyle(StationDS.Text.secondary)
                        }
                        Spacer(minLength: 0)
                        Text("Backend Box")
                            .font(StationDS.bodyFont(17))
                            .foregroundStyle(StationDS.Text.muted)
                        Image(systemName: "chevron.right")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(StationDS.Text.muted)
                    }
                    .frame(minHeight: 44)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
    }

    private var usedTodayCaption: String {
        DeskRulesPresentation.usedTodayCaption(
            closedPnL: coordinator.todayViewModel.sessionPnLUsd,
            quoteCurrency: captionQuoteCurrency,
            healthy: coordinator.todayViewModel.presentation.state == .healthyActive
        )
    }

    private var captionQuoteCurrency: String? {
        let configured = coordinator.brokersViewModel.configuredBrokerSlugs
        switch DeskHonesty.resolve(activeSlugs: configured) {
        case let .single(quoteCurrency, _, _):
            return quoteCurrency
        case .dualNoBlend:
            guard let active = coordinator.sessionModel.activeBrokerSlug else { return nil }
            return DeskHonesty.heroQuoteCurrency(activeSlugs: [active])
        case .none:
            return coordinator.todayViewModel.lastDeskQuoteCurrency
        }
    }

    private func openAPIKeys() {
        guard let destination = DeskRulesPresentation.navigationDestination(for: .apiKeys) else {
            return
        }
        coordinator.navigateTo(destination)
    }

    private func seedLimitFields() {
        floorText = formattedMoney(deskRules.dailyFloor)
        meanText = formattedMoney(deskRules.meanLoss)
        tripsText = deskRules.maxRoundTrips.map(String.init) ?? ""
    }

    private func commitLimit(_ field: LimitField?) {
        switch field {
        case .floor:
            deskRules.setDailyFloor(parseMoney(floorText))
            floorText = formattedMoney(deskRules.dailyFloor)
        case .mean:
            deskRules.setMeanLoss(parseMoney(meanText))
            meanText = formattedMoney(deskRules.meanLoss)
        case .trips:
            deskRules.setMaxRoundTrips(parseInt(tripsText))
            tripsText = deskRules.maxRoundTrips.map(String.init) ?? ""
        case nil:
            break
        }
    }

    private func formattedMoney(_ value: Double?) -> String {
        guard let value else { return "" }
        if let quote = captionQuoteCurrency {
            return DeskMoneyFormatting.formatWhole(value, quoteCurrency: quote)
        }
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = 0
        return formatter.string(from: NSNumber(value: value)) ?? ""
    }

    private func parseMoney(_ raw: String) -> Double? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed != "—" else { return nil }
        let stripped = trimmed
            .replacingOccurrences(of: ",", with: "")
            .replacingOccurrences(of: "₹", with: "")
            .replacingOccurrences(of: "$", with: "")
            .replacingOccurrences(of: "INR", with: "")
            .replacingOccurrences(of: "USD", with: "")
            .replacingOccurrences(of: "+", with: "")
            .trimmingCharacters(in: .whitespaces)
        return Double(stripped)
    }

    private func parseInt(_ raw: String) -> Int? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed != "—" else { return nil }
        return Int(trimmed)
    }

    private func settingsGroup<Content: View>(
        title: String,
        footer: String? = nil,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(StationDS.bodyFont(13))
                .foregroundStyle(StationDS.Text.secondary)
                .padding(.horizontal, 16)
            content()
            if let footer {
                Text(footer)
                    .font(StationDS.bodyFont(12))
                    .foregroundStyle(StationDS.Text.muted)
                    .padding(.horizontal, 16)
                    .padding(.top, 0)
            }
        }
    }

    private func inset<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        VStack(spacing: 0) {
            content()
        }
        .background(StationDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
    }

    private func toggleRow(title: String, caption: String, isOn: Binding<Bool>) -> some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(StationDS.bodyFont(17))
                    .foregroundStyle(StationDS.Text.primary)
                Text(caption)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.secondary)
            }
            Spacer(minLength: 0)
            Toggle("", isOn: isOn)
                .labelsHidden()
                .tint(StationDS.Accent.teal)
        }
        .frame(minHeight: 44)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    private func limitRow(
        title: String,
        caption: String?,
        text: Binding<String>,
        field: LimitField,
        placeholder: String
    ) -> some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(StationDS.bodyFont(17))
                    .foregroundStyle(StationDS.Text.primary)
                if let caption {
                    Text(caption)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.secondary)
                }
            }
            Spacer(minLength: 0)
            TextField(placeholder, text: text)
                .textFieldStyle(.plain)
                .multilineTextAlignment(.trailing)
                .font(StationDS.bodyFont(17))
                .foregroundStyle(StationDS.Text.muted)
                .frame(minWidth: 88, maxWidth: 160)
                .focused($focusedLimit, equals: field)
                .onSubmit { commitLimit(field) }
        }
        .frame(minHeight: 44)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}
