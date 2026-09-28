import AppKit
import SwiftUI

public struct DeviceLoginView: View {
    @ObservedObject private var viewModel: DeviceLoginViewModel

    public init(viewModel: DeviceLoginViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Station sign-in")
                .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                .foregroundStyle(StationDS.Text.primary)

            Text("Sign in once in your browser. Station shows your pairing code only — never paste a device code.")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                .foregroundStyle(StationDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)

            switch viewModel.phase {
            case .idle, .error:
                Button("Sign in with browser") {
                    Task { await viewModel.beginLogin() }
                }
                .buttonStyle(.borderedProminent)
                .tint(StationDS.Accent.teal)

            case .starting:
                ProgressView("Starting device login…")
                    .tint(StationDS.Accent.teal)

            case .awaitingBrowser:
                if let userCode = viewModel.userCode {
                    Text(userCode)
                        .font(.system(.title2, design: .monospaced).weight(.semibold))
                        .foregroundStyle(StationDS.Text.primary)
                        .textSelection(.enabled)
                        .accessibilityIdentifier("deviceLoginUserCode")
                }
                Text("Finish sign-in in the browser — Station will connect automatically when the code is approved.")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                    .foregroundStyle(StationDS.Text.secondary)
                HStack(spacing: 8) {
                    if viewModel.phase == .completing {
                        ProgressView("Waiting for browser…")
                            .tint(StationDS.Accent.teal)
                    }
                    Button("Retry connection") {
                        Task { await viewModel.completeLogin() }
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(StationDS.Accent.teal)
                    .disabled(viewModel.phase == .completing)
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
                    .tint(StationDS.Accent.teal)

            case .signedIn:
                if let email = viewModel.signedInEmail {
                    Text("Signed in as \(email)")
                        .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
                        .accessibilityIdentifier("deviceLoginSignedInEmail")
                }
                Button("Sign out") {
                    Task { await viewModel.signOut() }
                }
                .buttonStyle(.bordered)
            }

            if let errorMessage = viewModel.errorMessage {
                Text(errorMessage)
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                    .foregroundStyle(StationDS.Accent.red)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(StationDS.Fill.sidebar)
        .task {
            await viewModel.refreshSession()
        }
    }
}
