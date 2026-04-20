import SwiftUI

struct TAILeftView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Circle()
                    .fill(Color(hex: "#00E5C0"))
                    .frame(width: 6, height: 6)
                    .glowEffect(Color(hex: "#00E5C0").opacity(0.6), radius: 4)
                Text("TAI")
                    .font(.system(size: 9, weight: .bold, design: .rounded))
                    .foregroundColor(Color(hex: "#00E5C0"))
                    .tracking(1.2)
                    .textCase(.uppercase)
                Text("COACHING")
                    .font(.system(size: 8, weight: .medium, design: .monospaced))
                    .foregroundColor(Color.white.opacity(0.25))
                    .tracking(1.0)
                    .textCase(.uppercase)
            }

            VStack(spacing: 6) {
                ForEach(Array(viewModel.taiMessages.suffix(3))) { msg in
                    Group {
                        if msg.role == "assistant" {
                            HStack(alignment: .top, spacing: 6) {
                                ZStack {
                                    Circle()
                                        .fill(Color(hex: "#00E5C0").opacity(0.1))
                                        .frame(width: 18, height: 18)
                                    Image(systemName: "brain.head.profile")
                                        .font(.system(size: 8, weight: .medium))
                                        .foregroundColor(Color(hex: "#00E5C0"))
                                }
                                Text(msg.content)
                                    .font(.system(size: 10, weight: .regular, design: .rounded))
                                    .foregroundColor(Color.white.opacity(0.75))
                                    .lineLimit(3)
                                    .fixedSize(horizontal: false, vertical: true)
                                Spacer()
                            }
                        } else {
                            HStack {
                                Spacer()
                                Text(msg.content)
                                    .font(.system(size: 10, weight: .regular, design: .rounded))
                                    .foregroundColor(Color.white.opacity(0.5))
                                    .padding(8)
                                    .background(Color.white.opacity(0.05))
                                    .cornerRadius(8)
                                    .lineLimit(2)
                            }
                        }
                    }
                }
            }

            Spacer()
        }
        .padding(12)
        .glassCard(radius: 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

struct TAIRightView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("QUICK ASK")

            HStack(spacing: 8) {
                TextField("Ask TAI...", text: $viewModel.taiInput)
                    .font(.system(size: 11, weight: .regular, design: .rounded))
                    .foregroundColor(.white)
                    .textFieldStyle(.plain)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 8)
                    .background(Color.white.opacity(0.05))
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color.white.opacity(0.08), lineWidth: 0.5)
                    )

                Button {
                    let t = viewModel.taiInput
                    Task { await viewModel.sendTAIMessage(t) }
                } label: {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.system(size: 22))
                        .foregroundColor(
                            viewModel.taiInput.isEmpty
                                ? Color.white.opacity(0.2)
                                : Color(hex: "#00E5C0")
                        )
                        .glowEffect(
                            viewModel.taiInput.isEmpty
                                ? Color.clear
                                : Color(hex: "#00E5C0").opacity(0.3),
                            radius: 8
                        )
                }
                .buttonStyle(.plain)
                .disabled(viewModel.taiInput.isEmpty)
            }

            divider()

            sectionHeader("SUGGESTIONS")
            VStack(spacing: 5) {
                ForEach([
                    "Why am I tilting?",
                    "Should I trade now?",
                    "Show my patterns",
                ], id: \.self) { suggestion in
                    Button {
                        Task { await viewModel.sendTAIMessage(suggestion) }
                    } label: {
                        HStack {
                            Text(suggestion)
                                .font(.system(size: 10, weight: .regular, design: .rounded))
                                .foregroundColor(Color.white.opacity(0.5))
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(.system(size: 8))
                                .foregroundColor(Color.white.opacity(0.2))
                        }
                        .padding(.horizontal, 10)
                        .padding(.vertical, 7)
                        .background(Color.white.opacity(0.03))
                        .cornerRadius(8)
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.white.opacity(0.05), lineWidth: 0.5)
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
