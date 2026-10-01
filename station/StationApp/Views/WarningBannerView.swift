import SwiftUI

public struct WarningBannerView: View {
    public let warning: AgentHealthWarning
    public var onRetry: (() -> Void)?

    public init(warning: AgentHealthWarning, onRetry: (() -> Void)? = nil) {
        self.warning = warning
        self.onRetry = onRetry
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(warning.message)
                .font(.callout)
                .foregroundStyle(.primary)

            if warning.reason == .portCollisionNonAgent {
                Text("Check DAEMON_PORT and ensure port 9137 is free or used by tradeautopsy-agent.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if warning.reason == .killSwitchLatched {
                Text("The kill switch latch keeps the Enforcer alive with DNS teeth. Dismiss the kill from the desk before restarting.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if let logPath = warning.logPath {
                Text("Log: \(logPath.path)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if warning.canRetry, let onRetry {
                Button("Retry", action: onRetry)
                    .buttonStyle(.borderedProminent)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.orange.opacity(0.15))
    }
}
