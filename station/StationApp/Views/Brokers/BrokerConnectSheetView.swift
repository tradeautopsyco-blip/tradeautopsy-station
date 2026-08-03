import SwiftUI
import UniformTypeIdentifiers

struct BrokerConnectSheetView: View {
    @ObservedObject var viewModel: BrokersViewModel
    /// Local buffers for one-time secrets — SecureField must not bind only to cleared VM strings.
    @State private var totpDraft = ""
    @State private var mpinDraft = ""
    @State private var secretFieldsEpoch = 0

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(sheetTitle)
                .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                .foregroundStyle(StationDS.Text.primary)

            Text(viewModel.connectDisclosure)
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                .foregroundStyle(StationDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)

            if viewModel.connectAuthScheme == .kotakNeoTotpSession {
                if viewModel.connectSheetMode == .kotakTotpOnly {
                    kotakTotpOnlyFields
                } else {
                    kotakFullFields
                }
            } else {
                hmacFields
            }

            if let message = viewModel.connectMessage {
                Text(message)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                    .foregroundStyle(StationDS.Accent.amber)
            }

            HStack {
                Button("Cancel") {
                    viewModel.isConnectSheetPresented = false
                }
                .keyboardShortcut(.cancelAction)

                Spacer()

                Button(viewModel.isConnecting ? "Connecting…" : primaryButtonTitle) {
                    Task { await submit() }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(viewModel.isConnecting)
            }
        }
        .padding(24)
        .frame(width: 420)
        .onChange(of: viewModel.isConnectSheetPresented) { _, presented in
            if presented {
                totpDraft = ""
                mpinDraft = ""
                secretFieldsEpoch += 1
            }
        }
        .onChange(of: viewModel.connectSecretFieldsEpoch) { _, epoch in
            totpDraft = ""
            if viewModel.connectSheetMode == .full {
                mpinDraft = ""
            }
            secretFieldsEpoch = epoch
        }
    }

    private var sheetTitle: String {
        if viewModel.connectSheetMode == .kotakTotpOnly {
            return "Refresh \(viewModel.connectBrokerDisplayName) session"
        }
        return "Connect \(viewModel.connectBrokerDisplayName)"
    }

    private var primaryButtonTitle: String {
        viewModel.connectSheetMode == .kotakTotpOnly ? "Refresh" : "Connect"
    }

    private func submit() async {
        if viewModel.connectAuthScheme == .kotakNeoTotpSession {
            let mpin = viewModel.connectSheetMode == .kotakTotpOnly
                ? viewModel.connectMpin
                : mpinDraft
            viewModel.updateKotakLoginFields(
                consumerKey: viewModel.connectConsumerKey,
                mobileNumber: viewModel.connectMobileNumber,
                ucc: viewModel.connectUcc,
                totp: totpDraft,
                mpin: mpin
            )
        }
        await viewModel.submitConnect()
    }

    @ViewBuilder
    private var hmacFields: some View {
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
    }

    @ViewBuilder
    private var kotakTotpOnlyFields: some View {
        lockedRow(label: "Consumer Key", value: maskMiddle(viewModel.connectConsumerKey))
        lockedRow(label: "Mobile", value: maskMiddle(viewModel.connectMobileNumber))
        lockedRow(label: "UCC", value: viewModel.connectUcc)
        lockedRow(label: "MPIN", value: "••••••")

        credentialField(
            title: "TOTP (fresh code)",
            text: $totpDraft,
            invalid: viewModel.connectInvalidFields.contains(.totp),
            secure: false
        )
        .id("kotak-totp-\(secretFieldsEpoch)")
    }

    @ViewBuilder
    private var kotakFullFields: some View {
        credentialField(
            title: "Consumer Key",
            text: Binding(
                get: { viewModel.connectConsumerKey },
                set: {
                    viewModel.updateKotakLoginFields(
                        consumerKey: $0,
                        mobileNumber: viewModel.connectMobileNumber,
                        ucc: viewModel.connectUcc,
                        totp: totpDraft,
                        mpin: mpinDraft
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.consumerKey)
        )
        credentialField(
            title: "Mobile (+country code)",
            text: Binding(
                get: { viewModel.connectMobileNumber },
                set: {
                    viewModel.updateKotakLoginFields(
                        consumerKey: viewModel.connectConsumerKey,
                        mobileNumber: $0,
                        ucc: viewModel.connectUcc,
                        totp: totpDraft,
                        mpin: mpinDraft
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.mobileNumber)
        )
        credentialField(
            title: "UCC",
            text: Binding(
                get: { viewModel.connectUcc },
                set: {
                    viewModel.updateKotakLoginFields(
                        consumerKey: viewModel.connectConsumerKey,
                        mobileNumber: viewModel.connectMobileNumber,
                        ucc: $0,
                        totp: totpDraft,
                        mpin: mpinDraft
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.ucc)
        )
        credentialField(
            title: "TOTP",
            text: $totpDraft,
            invalid: viewModel.connectInvalidFields.contains(.totp),
            secure: false
        )
        .id("kotak-totp-\(secretFieldsEpoch)")
        credentialField(
            title: "MPIN",
            text: $mpinDraft,
            invalid: viewModel.connectInvalidFields.contains(.mpin),
            secure: true
        )
        .id("kotak-mpin-\(secretFieldsEpoch)")
    }

    private func lockedRow(label: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label)
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .medium))
                .foregroundStyle(StationDS.Text.labels)
            Text(value)
                .font(StationDS.monoFont(StationDS.FontSize.body, weight: .regular))
                .foregroundStyle(StationDS.Text.secondary)
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(StationDS.Fill.input)
                .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
        }
    }

    private func maskMiddle(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count > 6 else { return String(repeating: "•", count: max(4, trimmed.count)) }
        let prefix = trimmed.prefix(3)
        let suffix = trimmed.suffix(3)
        return "\(prefix)…\(suffix)"
    }

    private func credentialField(
        title: String,
        text: Binding<String>,
        invalid: Bool,
        secure: Bool = false
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .medium))
                .foregroundStyle(StationDS.Text.labels)

            Group {
                if secure {
                    SecureField(title, text: text)
                } else {
                    TextField(title, text: text)
                }
            }
            .textFieldStyle(.plain)
            .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
            .foregroundStyle(StationDS.Text.primary)
            .padding(10)
            .background(StationDS.Fill.input)
            .overlay(
                RoundedRectangle(cornerRadius: StationDS.Radius.small)
                    .stroke(invalid ? StationDS.Accent.red : StationDS.Border.outlineBtn, lineWidth: StationDS.borderThin)
            )
            .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
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
