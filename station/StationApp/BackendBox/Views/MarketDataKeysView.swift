import Notch
import SwiftUI

public struct MarketDataKeysView: View {
    @ObservedObject private var viewModel: MarketDataKeysViewModel

    public init(viewModel: MarketDataKeysViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Market Data")
                    .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(StationDS.Text.primary)

                Text("Store a Keychain key for a shipping vendor. Paste a key, not a URL.")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                    .foregroundStyle(StationDS.Text.muted)

                Text(viewModel.provenanceStrip(for: viewModel.selectedProvider))
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                    .foregroundStyle(StationDS.Text.labels)

                if let errorMessage = viewModel.errorMessage {
                    Text(errorMessage)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                        .foregroundStyle(StationDS.Text.labels)
                }

                addKeySection
                keyListSection
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task {
            await viewModel.loadKeys()
        }
    }

    private var addKeySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Add key")
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                .foregroundStyle(StationDS.Text.primary)

            Picker("Provider", selection: $viewModel.selectedProvider) {
                ForEach(MarketDataProvider.allCases) { provider in
                    Text(provider.rawValue).tag(provider)
                }
            }
            .pickerStyle(.menu)

            if viewModel.selectedProvider.requiresKey {
                SecureField("API key", text: $viewModel.draftAPIKey)
                    .textFieldStyle(.roundedBorder)

                Button("Save key") {
                    Task {
                        try? await viewModel.addKey(
                            provider: viewModel.selectedProvider,
                            apiKey: viewModel.draftAPIKey
                        )
                    }
                }
                .disabled(viewModel.draftAPIKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            } else {
                Text("AMFI is a public labs file. No key. Enable and pick a fetch mode.")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                    .foregroundStyle(StationDS.Text.muted)
            }
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(Color.white.opacity(0.04))
        )
    }

    @ViewBuilder
    private var keyListSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Bindings")
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                .foregroundStyle(StationDS.Text.primary)

            ForEach(viewModel.keys) { entry in
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(entry.provider.rawValue)
                                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                                .foregroundStyle(StationDS.Text.primary)
                            Text(entry.maskedValue)
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                                .foregroundStyle(StationDS.Text.muted)
                            Text(entry.enabled ? "Enabled" : "Disabled")
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                                .foregroundStyle(StationDS.Text.labels)
                            if entry.provider.requiresKey {
                                Text(validationLabel(for: entry.validationState))
                                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                                    .foregroundStyle(StationDS.Text.labels)
                            }
                            if let status = entry.lastObtainStatus {
                                Text("Last obtain: \(status)")
                                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                                    .foregroundStyle(StationDS.Text.labels)
                            }
                        }
                        Spacer()
                        if entry.enabled {
                            Button("Disable") {
                                Task { try? await viewModel.disable(id: entry.id) }
                            }
                        } else {
                            Button("Enable") {
                                Task { try? await viewModel.enable(id: entry.id) }
                            }
                        }
                        if entry.provider.requiresKey {
                            Button("Delete", role: .destructive) {
                                Task { try? await viewModel.deleteKey(id: entry.id) }
                            }
                        }
                    }
                    Picker("Fetch", selection: fetchModeBinding(for: entry)) {
                        ForEach(VendorFetchMode.allCases, id: \.self) { mode in
                            Text(mode.label).tag(mode)
                        }
                    }
                    .pickerStyle(.menu)
                    Button("Obtain now") {
                        Task { await viewModel.obtainNow(id: entry.id) }
                    }
                    .disabled(!entry.enabled)
                }
                .padding(12)
                .background(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(Color.white.opacity(0.04))
                )
            }
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
