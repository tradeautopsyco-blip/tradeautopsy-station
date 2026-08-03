import SwiftUI

public struct AIWorkflowKeysView: View {
    @ObservedObject private var viewModel: AIWorkflowKeysViewModel

    public init(viewModel: AIWorkflowKeysViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("AI / Workflow")
                    .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(StationDS.Text.primary)

                Text("Bring your own API keys for LLM and workflow providers. Validation is stubbed in v1.")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                    .foregroundStyle(StationDS.Text.muted)

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
                ForEach(AIWorkflowProvider.allCases) { provider in
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
            Text("No AI / workflow keys saved.")
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
                            Text(validationLabel(for: entry.validationState))
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                                .foregroundStyle(StationDS.Text.labels)
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
