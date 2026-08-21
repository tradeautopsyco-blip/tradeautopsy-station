import SwiftUI
import AppKit
import UniformTypeIdentifiers

/// PLAN Live capture card — Live trade only. Same job as HTML `#live-capture`.
struct BarLiveCaptureCard: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Live capture")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                .foregroundColor(BarDS.Accent.teal)
                .padding(.bottom, 4)
            Text(
                viewModel.stagedCapturePreview == nil
                    ? "Fn hold for voice. Paste a chart on the pill or this card — it stays on this Mac until you tap a trade."
                    : "On this Mac until you link a trade."
            )
            .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
            .foregroundColor(BarDS.Text.secondary)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.bottom, 10)

            if let preview = viewModel.stagedCapturePreview {
                Image(nsImage: preview)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(maxHeight: 96)
                    .frame(maxWidth: .infinity)
                    .background(BarDS.Fill.sidebar)
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                            .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
                    )
                    .accessibilityLabel("Staged screenshot")
                    .padding(.bottom, 8)
                Text("Caption")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .padding(.bottom, 6)
                BarInputField(placeholder: "Optional", text: $viewModel.journalCaptureDraft)
                    .onTapGesture {
                        viewModel.dictationUsesCaptureDraft = true
                    }
            }

            HStack(spacing: 6) {
                captureToolChip(
                    title: "Voice",
                    on: viewModel.dictationUsesCaptureDraft,
                    disabled: false
                ) {
                    viewModel.dictationUsesCaptureDraft = true
                }
                .accessibilityLabel("Voice capture")
                captureToolChip(
                    title: "Screenshot",
                    on: false,
                    disabled: viewModel.journalCaptureScreenshotBusy
                ) {
                    Task { await viewModel.stageScreenshotFromRegion() }
                }
                .accessibilityLabel("Capture screenshot")
            }
            .padding(.bottom, 8)

            if viewModel.stagedCapturePreview != nil {
                BarBigButton(label: "Keep on this Mac", style: .outline) {
                    _ = viewModel.keepStagedCaptureInTray()
                }
                .accessibilityLabel("Keep screenshot locally")
            }

            UnpostedCaptureTrayView(viewModel: viewModel)

            if viewModel.dictationUsesCaptureDraft {
                DictationMicAndWaveform(viewModel: viewModel, reduceMotion: reduceMotion)
                    .scaleEffect(0.92)
                    .padding(.top, 4)
            }
            if let sErr = viewModel.journalCaptureScreenshotError, !sErr.isEmpty {
                Text(sErr)
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
                    .padding(.top, 4)
            }
            if let err = viewModel.journalCaptureLastError, !err.isEmpty {
                Text(err)
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
                    .padding(.top, 4)
            }
            if let ok = viewModel.journalCaptureLastSuccess, !ok.isEmpty {
                Text(ok)
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.green.opacity(0.85))
                    .padding(.top, 4)
            }
        }
        .padding(.vertical, 13)
        .padding(.horizontal, 15)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Accent.teal.opacity(0.25), lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 8)
        .onPasteCommand(of: [.png, .tiff, .jpeg]) { _ in
            viewModel.ingestPastedImage()
        }
        .onDrop(of: [.image, .fileURL], isTargeted: nil) { providers in
            viewModel.ingestPastedImage()
            return !providers.isEmpty
        }
    }

    private func captureToolChip(title: String, on: Bool, disabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(on ? BarDS.Text.primary : BarDS.Text.secondary)
                .padding(.horizontal, 6)
                .padding(.vertical, 10)
                .frame(maxWidth: .infinity)
                .background(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .fill(on ? Color.white.opacity(0.08) : Color.white.opacity(0.03))
                )
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .stroke(
                            on ? Color.white.opacity(0.20) : BarDS.Border.input,
                            lineWidth: BarDS.borderThin
                        )
                )
        }
        .buttonStyle(.plain)
        .disabled(disabled)
    }
}
