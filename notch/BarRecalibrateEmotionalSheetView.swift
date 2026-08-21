import SwiftUI

/// In-app-only emotional reload after the 10th scalp (#113 slice 7) — no telemetry; local acknowledge.
struct BarRecalibrateEmotionalSheetView: View {
    @Binding var calm: Int?
    @Binding var confidence: Int?
    var onDone: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Quick recalibrate")
                .font(.system(size: 15, weight: .bold, design: .rounded))
                .foregroundColor(.white.opacity(0.94))
            Text("Re-rate how you feel right now — 1 calmest, 5 highest load. Nothing is uploaded until you use web Bar workflows.")
                .font(.system(size: 11, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.72))
                .fixedSize(horizontal: false, vertical: true)

            scaleRow(title: "Psychological calm (1 is best)", selection: $calm)
            scaleRow(title: "Confidence (5 is best)", selection: $confidence)

            Button("Done") { onDone() }
                .buttonStyle(.plain)
                .font(.system(size: 13, weight: .bold, design: .rounded))
                .foregroundColor(BarDS.Fill.accentInk)
                .padding(.horizontal, 20)
                .padding(.vertical, 10)
                .background(BarDS.Accent.teal)
                .cornerRadius(10)
                .padding(.top, 6)
        }
        .padding(18)
        .frame(maxWidth: 400)
    }

    private func scaleRow(title: String, selection: Binding<Int?>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(.white.opacity(0.5))
            HStack(spacing: 6) {
                ForEach(1 ... 5, id: \.self) { n in
                    let on = selection.wrappedValue == n
                    Button {
                        selection.wrappedValue = n
                    } label: {
                        Text("\(n)")
                            .font(.system(size: 11, weight: .semibold, design: .monospaced))
                            .foregroundColor(on ? BarDS.Accent.teal : Color.white.opacity(0.75))
                            .frame(width: 32, height: 30)
                            .background(
                                RoundedRectangle(cornerRadius: 6)
                                    .fill(on ? BarDS.Accent.teal.opacity(0.14) : Color.white.opacity(0.06)),
                            )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }
}
