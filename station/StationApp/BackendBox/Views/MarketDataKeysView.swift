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
                    .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(BarDS.Text.primary)

                Text("Store API keys for market-data providers. Validation is stubbed in v1.")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundStyle(BarDS.Text.muted)

                addKeySection
                keyListSection
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
        .task {
            await viewModel.loadKeys()
        }
    }

    private var addKeySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Add key")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                .foregroundStyle(BarDS.Text.primary)

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
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundStyle(BarDS.Text.muted)
        } else {
            VStack(alignment: .leading, spacing: 8) {
                Text("Saved keys")
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                    .foregroundStyle(BarDS.Text.primary)

                ForEach(viewModel.keys) { entry in
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(entry.provider.rawValue)
                                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                                .foregroundStyle(BarDS.Text.primary)
                            Text(entry.maskedValue)
                                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                                .foregroundStyle(BarDS.Text.muted)
                            Text(validationLabel(for: entry.validationState))
                                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                                .foregroundStyle(BarDS.Text.labels)
                        }
                        Spacer()
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
