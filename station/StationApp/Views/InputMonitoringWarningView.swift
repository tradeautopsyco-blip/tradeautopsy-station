import SwiftUI

public struct InputMonitoringWarningView: View {
    public let warning: InputMonitoringWarning
    public var onDismiss: (() -> Void)?

    public init(warning: InputMonitoringWarning, onDismiss: (() -> Void)? = nil) {
        self.warning = warning
        self.onDismiss = onDismiss
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(warning.message)
                .font(.callout)
                .foregroundStyle(.primary)

            HStack(spacing: 12) {
                Link("Open System Settings", destination: warning.systemSettingsURL)
                    .font(.caption)

                if let onDismiss {
                    Button("Dismiss", action: onDismiss)
                        .font(.caption)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.yellow.opacity(0.15))
    }
}
