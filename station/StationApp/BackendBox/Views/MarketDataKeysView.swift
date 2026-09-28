import Notch
import SwiftUI

public struct MarketDataKeysView: View {
    @ObservedObject private var viewModel: MarketDataKeysViewModel

    public init(viewModel: MarketDataKeysViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        DeskPageShell(
            title: "Market Data",
            subtitle: "Store a Keychain key for a shipping vendor. Paste a key, not a URL."
        ) {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x2) {
                Text(viewModel.provenanceStrip(for: viewModel.selectedProvider))
                    .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                    .foregroundStyle(StationDS.Text.labels)

                if let errorMessage = viewModel.errorMessage {
                    Text(errorMessage)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.callout))
                        .foregroundStyle(StationDS.Accent.amber)
                }

                addKeySection
                keyListSection
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .task {
            await viewModel.loadKeys()
        }
    }

    private var addKeySection: some View {
        DeskCard(title: "Add key") {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x1) {
                Picker("Provider", selection: $viewModel.selectedProvider) {
                    ForEach(MarketDataProvider.allCases) { provider in
                        Text(provider.rawValue).tag(provider)
                    }
                }
                .pickerStyle(.menu)

                if viewModel.selectedProvider.requiresKey {
                    SecureField("API key", text: $viewModel.draftAPIKey)
                        .textFieldStyle(.roundedBorder)
                } else {
                    Text("AMFI is a public labs file. No key. Enable and pick a fetch mode.")
                        .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.muted)
                }
            }
        } footer: {
            if viewModel.selectedProvider.requiresKey {
                Button("Save key") {
                    Task {
                        try? await viewModel.addKey(
                            provider: viewModel.selectedProvider,
                            apiKey: viewModel.draftAPIKey
                        )
                    }
                }
                .buttonStyle(.deskPrimary)
                .disabled(viewModel.draftAPIKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
    }

    @ViewBuilder
    private var keyListSection: some View {
        if viewModel.keys.isEmpty {
            Text("No market data bindings yet.")
                .font(DeskChrome.sans(DeskChrome.TypeScale.body))
                .foregroundStyle(StationDS.Text.muted)
        } else {
            DeskCard(title: "Bindings") {
                VStack(alignment: .leading, spacing: DeskChrome.Space.x2) {
                    ForEach(viewModel.keys) { entry in
                        bindingRow(entry)
                    }
                }
            }
        }
    }

    private func bindingRow(_ entry: MarketDataKeyListItem) -> some View {
        VStack(alignment: .leading, spacing: DeskChrome.Space.x1) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.provider.rawValue)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
                    Text(entry.maskedValue)
                        .font(DeskChrome.mono(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.muted)
                    Text(entry.enabled ? "Enabled" : "Disabled")
                        .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.labels)
                    if entry.provider.requiresKey {
                        Text(validationLabel(for: entry.validationState))
                            .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                            .foregroundStyle(StationDS.Text.labels)
                    }
                    if let status = entry.lastObtainStatus {
                        Text("Last obtain: \(status)")
                            .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                            .foregroundStyle(StationDS.Text.labels)
                    }
                }
                Spacer()
                VStack(alignment: .trailing, spacing: 6) {
                    if entry.enabled {
                        Button("Disable") { Task { try? await viewModel.disable(id: entry.id) } }
                            .buttonStyle(.deskSecondary)
                    } else {
                        Button("Enable") { Task { try? await viewModel.enable(id: entry.id) } }
                            .buttonStyle(.deskSecondary)
                    }
                    if entry.provider.requiresKey {
                        Button("Delete", role: .destructive) {
                            Task { try? await viewModel.deleteKey(id: entry.id) }
                        }
                        .buttonStyle(.deskDestructive)
                    }
                }
            }
            Picker("Fetch", selection: fetchModeBinding(for: entry)) {
                ForEach(VendorFetchMode.allCases, id: \.self) { mode in
                    Text(mode.label).tag(mode)
                }
            }
            .pickerStyle(.menu)
            Button("Obtain now") { Task { await viewModel.obtainNow(id: entry.id) } }
                .buttonStyle(.deskSecondary)
                .disabled(!entry.enabled)
        }
        .padding(.vertical, DeskChrome.Space.x1)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(StationDS.Border.divider)
                .frame(height: StationDS.borderThin)
        }
    }

    private func fetchModeBinding(for entry: MarketDataKeyListItem) -> Binding<VendorFetchMode> {
        Binding(
            get: { entry.fetchMode },
            set: { mode in
                Task { await viewModel.setFetchMode(mode, id: entry.id) }
            }
        )
    }

    private func validationLabel(for state: ProviderKeyValidationState) -> String {
        switch state {
        case .notValidated: return "Not validated"
        case .unknown: return "Unknown"
        }
    }
}
