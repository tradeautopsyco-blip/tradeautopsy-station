import Notch
import SwiftUI
import UniformTypeIdentifiers

struct BrokerConnectSheetView: View {
    @ObservedObject var viewModel: BrokersViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Connect \(viewModel.connectBrokerDisplayName)")
                .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .medium))
                .foregroundStyle(BarDS.Text.primary)

            Text(viewModel.connectDisclosure)
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                .foregroundStyle(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)

            credentialField(
                title: "API Key",
                text: Binding(
                    get: { viewModel.connectApiKey },
                    set: { viewModel.updateConnectFields(apiKey: $0, apiSecret: viewModel.connectApiSecret) }
                ),
                invalid: viewModel.connectInvalidFields.contains(.apiKey)
            )

            credentialField(
                title: "API Secret",
                text: Binding(
                    get: { viewModel.connectApiSecret },
                    set: { viewModel.updateConnectFields(apiKey: viewModel.connectApiKey, apiSecret: $0) }
                ),
                invalid: viewModel.connectInvalidFields.contains(.apiSecret),
                secure: true
            )

            if let message = viewModel.connectMessage {
                Text(message)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                    .foregroundStyle(BarDS.Accent.amber)
            }

            HStack {
                Button("Cancel") {
                    viewModel.isConnectSheetPresented = false
                }
                .keyboardShortcut(.cancelAction)

                Spacer()

                Button(viewModel.isConnecting ? "Connecting…" : "Connect") {
                    Task { await viewModel.submitConnect() }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(viewModel.isConnecting)
            }
        }
        .padding(24)
        .frame(width: 420)
    }

    private func credentialField(
        title: String,
        text: Binding<String>,
        invalid: Bool,
        secure: Bool = false
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundStyle(BarDS.Text.labels)

            Group {
                if secure {
                    SecureField(title, text: text)
                } else {
                    TextField(title, text: text)
                }
            }
            .textFieldStyle(.plain)
            .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
            .foregroundStyle(BarDS.Text.primary)
            .padding(10)
            .background(BarDS.Fill.input)
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small)
                    .stroke(invalid ? BarDS.Accent.red : BarDS.Border.outlineBtn, lineWidth: BarDS.borderThin)
            )
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small))
            .onPasteCommand(of: [.plainText]) { providers in
                applyPastedText(from: providers, to: text)
            }
        }
    }

    private func applyPastedText(from providers: [NSItemProvider], to text: Binding<String>) {
        guard let provider = providers.first else { return }
        provider.loadObject(ofClass: String.self) { value, _ in
            guard let value else { return }
            DispatchQueue.main.async {
                text.wrappedValue = value
            }
        }
    }
}
