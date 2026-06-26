import SwiftUI

/// Fullscreen psychology layer — 90s forced pause before dismiss restores broker access (#189).
struct KillSwitchOverlayView: View {
    @ObservedObject var viewModel: NotchViewModel

    private var countdownSecs: Int? { viewModel.killSwitchCountdownSecs }
    private var levelLabel: String {
        let lv = viewModel.killSwitchLevel?.uppercased() ?? "L3"
        return lv
    }

    var body: some View {
        ZStack {
            Color.black.opacity(0.94)
                .ignoresSafeArea()

            VStack(spacing: 16) {
                Text("KILL SWITCH ACTIVE")
                    .font(.system(size: 22, weight: .bold))
                    .foregroundStyle(Color.taDanger)
                    .tracking(2)

                Text(levelLabel == "L2" ? "Intervention — forced pause before you trade again."
                    : "Broker domains blocked. Take a breath before you resume.")
                    .font(.system(size: 13))
                    .foregroundStyle(Color.white.opacity(0.6))
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 360)

                VStack(spacing: 6) {
                    Text("New entries: BLOCKED")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(Color.white.opacity(0.75))
                    Text("Exit existing positions: ALLOWED")
                        .font(.system(size: 12))
                        .foregroundStyle(Color.white.opacity(0.55))
                }

                Text(KillSwitchOverlayPresentation.formattedCountdown(
                    secs: countdownSecs ?? 0
                ))
                    .font(.system(size: 42, weight: .bold, design: .monospaced))
                    .foregroundStyle(Color.taDanger)
                    .monospacedDigit()
                    .accessibilityLabel("Countdown \(KillSwitchOverlayPresentation.formattedCountdown(secs: countdownSecs ?? 0))")

                Button {
                    Task { await viewModel.dismissKillSwitchFromOverlay() }
                } label: {
                    Text(
                        viewModel.killSwitchDismissBusy
                            ? "Unlocking…"
                            : KillSwitchOverlayPresentation.calmButtonTitle(countdownSecs: countdownSecs)
                    )
                    .font(.system(size: 13))
                    .padding(.horizontal, 24)
                    .padding(.vertical, 8)
                }
                .buttonStyle(.plain)
                .foregroundStyle(Color.white.opacity(
                    KillSwitchOverlayPresentation.calmButtonEnabled(countdownSecs: countdownSecs) ? 0.9 : 0.35
                ))
                .overlay(
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .stroke(Color.white.opacity(0.3), lineWidth: 1)
                )
                .disabled(
                    viewModel.killSwitchDismissBusy
                        || !KillSwitchOverlayPresentation.calmButtonEnabled(countdownSecs: countdownSecs)
                )
            }
            .padding(32)
        }
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isModal)
    }
}
