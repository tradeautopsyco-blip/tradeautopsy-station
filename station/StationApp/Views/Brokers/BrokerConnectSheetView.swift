import SwiftUI
import UniformTypeIdentifiers

struct BrokerConnectSheetView: View {
    @ObservedObject var viewModel: BrokersViewModel
    /// Local buffers for one-time secrets — SecureField must not bind only to cleared VM strings.
    @State private var totpDraft = ""
    @State private var mpinDraft = ""
    @State private var secretFieldsEpoch = 0

    var body: some View {
        Group {
            if let oauthRequest = viewModel.oauthWebLoginRequest {
                BrokerOAuthLoginSheet(
                    request: oauthRequest,
                    isFinishing: viewModel.oauthWebLoginFinishing
                ) { success in
                    viewModel.completeOAuthWebLogin(success: success)
                }
            } else {
                connectForm
            }
        }
        .onChange(of: viewModel.isConnectSheetPresented) { _, presented in
            if !presented {
                viewModel.noteConnectSheetDismissed()
                if viewModel.oauthWebLoginRequest != nil, !viewModel.oauthWebLoginFinishing {
                    viewModel.completeOAuthWebLogin(success: false)
                }
            }
        }
    }

    private var connectForm: some View {
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
                    Button("Change Kotak login details…") {
                        viewModel.beginChangeKotakLoginDetails()
                    }
                    .buttonStyle(.link)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                } else {
                    kotakFullFields
                }
            } else if viewModel.connectAuthScheme == .growwChecksumSession {
                growwFields
            } else if viewModel.connectAuthScheme == .dhanConsentSession {
                dhanFields
            } else if viewModel.connectAuthScheme == .okxPassphraseSession {
                okxFields
            } else if viewModel.connectAuthScheme == .coinbaseJwtEs256Session {
                coinbaseFields
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
        } else if viewModel.connectAuthScheme == .dhanConsentSession {
            viewModel.updateDhanConnectFields(
                dhanClientId: viewModel.connectConsumerKey,
                apiKey: viewModel.connectApiKey,
                apiSecret: viewModel.connectApiSecret
            )
        }
        await viewModel.submitConnect()
    }

    /// Groww key entry (B6 row 10, ADR 0014): API key + secret from the Groww
    /// Cloud API Keys page. Station sends them to the local agent over signed
    /// loopback — the agent stores them in Keychain and mints the session. No
    /// browser step; the secret is typed once and never retained in Station.
    @ViewBuilder
    private var growwFields: some View {
        Text("Enter your Groww API key and secret from the Groww Cloud API Keys page. No browser step — the local agent mints the session and confirms it automatically.")
            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
            .foregroundStyle(StationDS.Text.muted)
            .fixedSize(horizontal: false, vertical: true)

        Text("Requires an active Groww Trading API subscription (₹499 + taxes/month). Approval keys need daily approval on the Groww Cloud API Keys page.")
            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
            .foregroundStyle(StationDS.Text.muted)
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
    }

    @ViewBuilder
    private var dhanFields: some View {
        Text(
            "Register redirect URL \(DhanConnectContract.loopbackRedirectURI) in your Dhan consent app, then enter client ID and app credentials. Station opens Dhan consent login in your browser; the agent saves the session when you finish."
        )
        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
        .foregroundStyle(StationDS.Text.muted)
        .fixedSize(horizontal: false, vertical: true)

        credentialField(
            title: "Dhan Client ID",
            text: Binding(
                get: { viewModel.connectConsumerKey },
                set: {
                    viewModel.updateDhanConnectFields(
                        dhanClientId: $0,
                        apiKey: viewModel.connectApiKey,
                        apiSecret: viewModel.connectApiSecret
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.consumerKey)
        )

        credentialField(
            title: "App ID",
            text: Binding(
                get: { viewModel.connectApiKey },
                set: {
                    viewModel.updateDhanConnectFields(
                        dhanClientId: viewModel.connectConsumerKey,
                        apiKey: $0,
                        apiSecret: viewModel.connectApiSecret
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.apiKey)
        )

        credentialField(
            title: "App Secret",
            text: Binding(
                get: { viewModel.connectApiSecret },
                set: {
                    viewModel.updateDhanConnectFields(
                        dhanClientId: viewModel.connectConsumerKey,
                        apiKey: viewModel.connectApiKey,
                        apiSecret: $0
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.apiSecret),
            secure: true
        )
    }

    @ViewBuilder
    private var okxFields: some View {
        Text("Enter your OKX API key, secret, and passphrase from the OKX API management page. Credentials are stored in the Mac Keychain for the local agent.")
            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
            .foregroundStyle(StationDS.Text.muted)
            .fixedSize(horizontal: false, vertical: true)

        credentialField(
            title: "API Key",
            text: Binding(
                get: { viewModel.connectApiKey },
                set: {
                    viewModel.updateOkxConnectFields(
                        apiKey: $0,
                        apiSecret: viewModel.connectApiSecret,
                        passphrase: viewModel.connectPassphrase
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.apiKey)
        )

        credentialField(
            title: "API Secret",
            text: Binding(
                get: { viewModel.connectApiSecret },
                set: {
                    viewModel.updateOkxConnectFields(
                        apiKey: viewModel.connectApiKey,
                        apiSecret: $0,
                        passphrase: viewModel.connectPassphrase
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.apiSecret),
            secure: true
        )

        credentialField(
            title: "Passphrase",
            text: Binding(
                get: { viewModel.connectPassphrase },
                set: {
                    viewModel.updateOkxConnectFields(
                        apiKey: viewModel.connectApiKey,
                        apiSecret: viewModel.connectApiSecret,
                        passphrase: $0
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.passphrase),
            secure: true
        )
    }

    @ViewBuilder
    private var coinbaseFields: some View {
        Text("Enter your Coinbase Advanced Trade API key name and EC private key (PEM). The private key is stored only in the Mac Keychain — never in Station memory after Connect.")
            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
            .foregroundStyle(StationDS.Text.muted)
            .fixedSize(horizontal: false, vertical: true)

        credentialField(
            title: "API Key Name",
            text: Binding(
                get: { viewModel.connectApiKey },
                set: {
                    viewModel.updateCoinbaseConnectFields(
                        apiKey: $0,
                        pemPrivateKey: viewModel.connectPemPrivateKey
                    )
                }
            ),
            invalid: viewModel.connectInvalidFields.contains(.apiKey)
        )

        VStack(alignment: .leading, spacing: 6) {
            Text("PEM Private Key")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .medium))
                .foregroundStyle(StationDS.Text.labels)

            TextEditor(
                text: Binding(
                    get: { viewModel.connectPemPrivateKey },
                    set: {
                        viewModel.updateCoinbaseConnectFields(
                            apiKey: viewModel.connectApiKey,
                            pemPrivateKey: $0
                        )
                    }
                )
            )
            .font(StationDS.monoFont(StationDS.FontSize.bodySmall, weight: .regular))
            .foregroundStyle(StationDS.Text.primary)
            .frame(minHeight: 120)
            .padding(8)
            .background(StationDS.Fill.input)
            .overlay(
                RoundedRectangle(cornerRadius: StationDS.Radius.small)
                    .stroke(
                        viewModel.connectInvalidFields.contains(.pemPrivateKey)
                            ? StationDS.Accent.red
                            : StationDS.Border.outlineBtn,
                        lineWidth: StationDS.borderThin
                    )
            )
            .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
        }
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
        _ = provider.loadObject(ofClass: String.self) { value, _ in
            guard let value else { return }
            DispatchQueue.main.async {
                text.wrappedValue = value
            }
        }
    }
}
