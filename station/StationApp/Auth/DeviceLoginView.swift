import AppKit
import Notch
import SwiftUI

public struct DeviceLoginView: View {
    @ObservedObject private var viewModel: DeviceLoginViewModel

    public init(viewModel: DeviceLoginViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Station sign-in")
                .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .medium))
                .foregroundStyle(BarDS.Text.primary)

            Text("Sign in with WorkOS device login. Station shows your user code only — never paste a device code.")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundStyle(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)

            switch viewModel.phase {
            case .idle, .error:
                Button("Sign in with browser") {
                    Task { await viewModel.beginLogin() }
                }
                .buttonStyle(.borderedProminent)

            case .starting:
                ProgressView("Starting device login…")

            case .awaitingBrowser:
                if let userCode = viewModel.userCode {
                    Text(userCode)
                        .font(.system(.title2, design: .monospaced).weight(.semibold))
                        .foregroundStyle(BarDS.Text.primary)
                        .textSelection(.enabled)
                        .accessibilityIdentifier("deviceLoginUserCode")
                }
                Text("Confirm this code in the browser, then continue.")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundStyle(BarDS.Text.secondary)
                HStack(spacing: 8) {
                    Button("I've confirmed") {
                        Task { await viewModel.completeLogin() }
                    }
                    .buttonStyle(.borderedProminent)
                    Button("Open browser again") {
                        if let raw = viewModel.verificationURIComplete,
                           let url = URL(string: raw) {
                            NSWorkspace.shared.open(url)
                        }
                    }
                    .buttonStyle(.bordered)
                }

            case .completing:
                ProgressView("Finishing sign-in…")

            case .signedIn:
                if let email = viewModel.signedInEmail {
                    Text("Signed in as \(email)")
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                        .foregroundStyle(BarDS.Text.primary)
                        .accessibilityIdentifier("deviceLoginSignedInEmail")
                }
                Button("Sign out") {
                    Task { await viewModel.signOut() }
                }
                .buttonStyle(.bordered)
            }

            if let errorMessage = viewModel.errorMessage {
                Text(errorMessage)
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.sidebar)
        .task {
            await viewModel.refreshSession()
        }
    }
}
