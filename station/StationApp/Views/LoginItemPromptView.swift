import SwiftUI

public struct LoginItemPromptView: View {
    public var onEnable: () -> Void
    public var onSkip: () -> Void

    public init(onEnable: @escaping () -> Void, onSkip: @escaping () -> Void) {
        self.onEnable = onEnable
        self.onSkip = onSkip
    }

    public var body: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Launch at Login")
                    .font(.headline)
                Text("Launch TradeAutopsy Station at login so the agent is ready before Kite opens.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Spacer(minLength: 8)

            HStack(spacing: 8) {
                Button("Skip", action: onSkip)
                    .buttonStyle(.bordered)
                Button("Enable", action: onEnable)
                    .buttonStyle(.borderedProminent)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.accentColor.opacity(0.08))
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(Color.primary.opacity(0.08))
                .frame(height: 1)
        }
    }
}
