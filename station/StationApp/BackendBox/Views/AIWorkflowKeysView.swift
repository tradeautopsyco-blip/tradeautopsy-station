import SwiftUI

public struct AIWorkflowKeysView: View {
    @ObservedObject private var viewModel: AIWorkflowKeysViewModel

    public init(viewModel: AIWorkflowKeysViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        DeskPageShell(
            title: "AI / Workflow",
            subtitle: "Bring your own API keys for LLM and workflow providers."
        ) {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x2) {
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
                    ForEach(AIWorkflowProvider.allCases) { provider in
                        Text(provider.rawValue).tag(provider)
                    }
                }
                .pickerStyle(.menu)

                SecureField("API key", text: $viewModel.draftAPIKey)
                    .textFieldStyle(.roundedBorder)
            }
        } footer: {
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

    @ViewBuilder
    private var keyListSection: some View {
        if viewModel.keys.isEmpty {
            Text("No AI / workflow keys saved.")
                .font(DeskChrome.sans(DeskChrome.TypeScale.body))
                .foregroundStyle(StationDS.Text.muted)
        } else {
            DeskCard(title: "Saved keys") {
                VStack(alignment: .leading, spacing: DeskChrome.Space.x2) {
                    ForEach(viewModel.keys) { entry in
                        HStack(alignment: .top) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(entry.provider.rawValue)
                                    .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .medium))
                                    .foregroundStyle(StationDS.Text.primary)
                                Text(entry.maskedValue)
                                    .font(DeskChrome.mono(DeskChrome.TypeScale.footnote))
                                    .foregroundStyle(StationDS.Text.muted)
                                Text(validationLabel(for: entry.validationState))
                                    .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                                    .foregroundStyle(StationDS.Text.labels)
                            }
                            Spacer()
                            Button("Delete", role: .destructive) {
                                Task { try? await viewModel.deleteKey(id: entry.id) }
                            }
                            .buttonStyle(.deskDestructive)
                        }
                    }
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
