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
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(Color.white.opacity(0.04))
        )
    }

    @ViewBuilder
    private var keyListSection: some View {
        if viewModel.keys.isEmpty {
            Text("No market data keys saved.")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                .foregroundStyle(StationDS.Text.muted)
        } else {
            VStack(alignment: .leading, spacing: 8) {
                Text("Saved keys")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                    .foregroundStyle(StationDS.Text.primary)

                ForEach(viewModel.keys) { entry in
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
                            Text(validationLabel(for: entry.validationState))
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                                .foregroundStyle(StationDS.Text.labels)
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
                        Button("Delete", role: .destructive) {
                            Task { try? await viewModel.deleteKey(id: entry.id) }
                        }
                    }
                    .padding(12)
                    .background(
                        RoundedRectangle(cornerRadius: 8, style: .continuous)
                            .fill(Color.white.opacity(0.04))
                    )
                }
            }
        }
    }

    private func validationLabel(for state: ProviderKeyValidationState) -> String {
        switch state {
        case .notValidated: return "Not validated"
        case .unknown: return "Unknown"
        }
    }
}
