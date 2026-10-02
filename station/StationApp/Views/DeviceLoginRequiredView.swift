import SwiftUI

/// Persistent banner for "agent is up but the Station session is dead" — the
/// quiet refresh loop can burn the Keychain refresh family without the user
/// ever opening Settings, so the signed-out state lives here too.
public struct DeviceLoginRequiredView: View {
    public var onSignIn: () -> Void

    public init(onSignIn: @escaping () -> Void) {
        self.onSignIn = onSignIn
    }

    public var body: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Device login required")
                    .font(.headline)
                Text("Station's saved session was revoked — market data keeps flowing, but Console calls need a fresh sign-in.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Spacer(minLength: 8)

            Button("Sign in", action: onSignIn)
                .buttonStyle(.borderedProminent)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.orange.opacity(0.08))
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(Color.primary.opacity(0.08))
                .frame(height: 1)
        }
    }
}
