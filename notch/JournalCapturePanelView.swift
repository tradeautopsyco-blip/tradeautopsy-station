import SwiftUI

struct JournalCapturePanelView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceMotion) private var accessibilityReduceMotion
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @FocusState private var editorFocused: Bool

    private var linkLocked: Bool {
        viewModel.journalCaptureLinkLocked
    }

    var body: some View {
        Group {
            if viewModel.daemonProtocolError == .protoVersion {
                protoVersionGate
            } else {
                captureScrollContent
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            viewModel.updateDictationReduceMotion(accessibilityReduceMotion)
        }
        .onChange(of: accessibilityReduceMotion) { v in
            viewModel.updateDictationReduceMotion(v)
        }
    }

    private var protoVersionGate: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 8) {
                Image(systemName: "arrow.up.circle.fill")
                    .foregroundColor(Color(hex: "#F5A524"))
                    .accessibilityHidden(true)
                Text("Agent update required — restart the app.")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.9))
                    .accessibilityLabel("Agent update required. Restart the app.")
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color(hex: "#F5A524").opacity(0.08))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(Color(hex: "#F5A524").opacity(0.28), lineWidth: 0.5)
            )
            .cornerRadius(10)

            Button {
                withAnimation(NotchTheme.springExpand) {
                    viewModel.isExpanded = false
                }
            } label: {
                Text("Close")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundColor(Color(hex: "#050505"))
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 10)
                    .background(Color(hex: "#00E5C0"))
                    .cornerRadius(10)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Close capture panel")

            Spacer()
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 16)
    }

    private var captureScrollContent: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                captureHeader

                if let banner = viewModel.journalCaptureBanner, !banner.isEmpty {
                    HStack(spacing: 6) {
                        Image(systemName: "bolt.fill")
                            .foregroundColor(Color(hex: "#00E5C0"))
                            .font(.system(size: 10))
                            .accessibilityHidden(true)
                        Text(banner)
                            .font(.system(size: 10, weight: .medium, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.85))
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(journalChromeFill(opacity: 0.05))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color(hex: "#00E5C0").opacity(0.25), lineWidth: 0.5)
                    )
                    .cornerRadius(10)
                }

                Text("What happened?")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.9))

                TextEditor(text: $viewModel.journalCaptureDraft)
                    .focused($editorFocused)
                    .font(.system(size: 12, weight: .regular, design: .rounded))
                    .foregroundColor(.white)
                    .frame(minHeight: 110, maxHeight: 220)
                    .padding(10)
                    .background(journalChromeFill(opacity: 0.06))
                    .cornerRadius(12)
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(Color.white.opacity(0.1), lineWidth: 0.5)
                    )
                    .accessibilityLabel("Journal entry")

                Text("Link to trade")
                    .font(.system(size: 9, weight: .semibold, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.45))

                tradeLinkControls

                Toggle(isOn: $viewModel.journalCaptureExplicitPending) {
                    Text("Save as pending (no trade link yet)")
                        .font(.system(size: 10, weight: .medium, design: .rounded))
                        .foregroundColor(Color.white.opacity(0.65))
                }
                .disabled(linkLocked)
                .accessibilityLabel("Save capture as pending without trade link")
                .accessibilityValue(viewModel.journalCaptureExplicitPending ? "on" : "off")

                if linkLocked {
                    Text("Policy C: trade link and pending toggle stay fixed while this note has text. Clear the note to change them.")
                        .font(.system(size: 9, weight: .medium, design: .rounded))
                        .foregroundColor(Color.white.opacity(0.4))
                }

                if viewModel.journalCaptureLastPendingCaptureId != nil {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Screenshot")
                            .font(.system(size: 9, weight: .semibold, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.45))
                        Button {
                            Task { await viewModel.attachJournalCaptureScreenshotToPending() }
                        } label: {
                            HStack(spacing: 8) {
                                Image(systemName: "camera.viewfinder")
                                    .font(.system(size: 12))
                                    .accessibilityHidden(true)
                                if viewModel.journalCaptureScreenshotBusy {
                                    ProgressView()
                                        .scaleEffect(0.65)
                                        .tint(Color(hex: "#00E5C0"))
                                }
                                Text(
                                    viewModel.journalCaptureScreenshotBusy
                                        ? "Capturing or uploading…"
                                        : "Attach region screenshot"
                                )
                                .font(.system(size: 10, weight: .semibold, design: .rounded))
                            }
                            .foregroundColor(Color(hex: "#00E5C0"))
                            .padding(.horizontal, 12)
                            .padding(.vertical, 8)
                            .background(journalChromeFill(opacity: 0.06))
                            .cornerRadius(10)
                            .overlay(
                                RoundedRectangle(cornerRadius: 10)
                                    .stroke(Color(hex: "#00E5C0").opacity(0.35), lineWidth: 0.5)
                            )
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Attach region screenshot to pending capture")
                        .disabled(viewModel.journalCaptureScreenshotBusy)

                        if let sErr = viewModel.journalCaptureScreenshotError, !sErr.isEmpty {
                            Text(sErr)
                                .font(.system(size: 10, weight: .medium, design: .rounded))
                                .foregroundColor(Color(hex: "#FF3B30"))
                                .accessibilityLabel(sErr)
                        }
                    }
                }

                if viewModel.dictationPermissionDenied || viewModel.dictationOnDeviceOnlyUnsupported {
                    dictationAlerts
                } else {
                    DictationMicAndWaveform(viewModel: viewModel, reduceMotion: accessibilityReduceMotion)
                }

                statusRows

                HStack(spacing: 12) {
                    Button {
                        viewModel.persistJournalCaptureDraftLocally()
                    } label: {
                        Text("Save draft")
                            .font(.system(size: 11, weight: .semibold, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.85))
                            .padding(.horizontal, 16)
                            .padding(.vertical, 10)
                            .background(journalChromeFill(opacity: 0.08))
                            .cornerRadius(10)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Save capture draft locally")

                    Button {
                        Task { await viewModel.finalizeJournalCapture() }
                    } label: {
                        HStack(spacing: 6) {
                            if viewModel.journalCaptureBusy {
                                ProgressView()
                                    .scaleEffect(0.7)
                                    .tint(Color(hex: "#00E5C0"))
                            }
                            Text("Finalize")
                                .font(.system(size: 11, weight: .bold, design: .rounded))
                        }
                        .foregroundColor(Color(hex: "#050505"))
                        .padding(.horizontal, 18)
                        .padding(.vertical, 10)
                        .background(Color(hex: "#00E5C0"))
                        .cornerRadius(10)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Finalize journal capture")
                    .disabled(viewModel.journalCaptureBusy)

                    Spacer()

                    Button {
                        withAnimation(NotchTheme.springExpand) {
                            viewModel.isExpanded = false
                        }
                    } label: {
                        Text("Close")
                            .font(.system(size: 10, weight: .medium, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.45))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Close capture panel")
                }

                if !viewModel.sessionAuthenticated {
                    Button {
                        Task { await viewModel.beginOAuthSignIn() }
                    } label: {
                        Text("Sign In")
                            .font(.system(size: 11, weight: .semibold, design: .rounded))
                            .foregroundColor(.black)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                            .background(Color(hex: "#00E5C0"))
                            .cornerRadius(10)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Sign in to TradeAutopsy")
                }
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)
        }
    }

    @ViewBuilder
    private func journalChromeFill(opacity: Double) -> some View {
        if reduceTransparency {
            Color(hex: "#121212")
        } else {
            Color.white.opacity(opacity)
        }
    }

    private var tradeLinkControls: some View {
        Group {
            if viewModel.recentTrades.isEmpty {
                TextField("Trade UUID (optional if pending)", text: $viewModel.journalCaptureTradeIdRaw)
                    .font(.system(size: 11, weight: .regular, design: .monospaced))
                    .foregroundColor(.white)
                    .textFieldStyle(.plain)
                    .padding(10)
                    .background(journalChromeFill(opacity: linkLocked ? 0.03 : 0.06))
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color.white.opacity(0.08), lineWidth: 0.5)
                    )
                    .disabled(linkLocked)
                    .accessibilityLabel("Trade UUID optional")
            } else {
                Menu {
                    ForEach(viewModel.recentTrades) { trade in
                        Button("\(trade.symbol) \(trade.side.uppercased()) \(trade.qty)") {
                            guard !linkLocked else { return }
                            viewModel.journalCaptureTradeIdRaw = trade.id
                        }
                    }
                    Divider()
                    Button("Enter ID manually") {}
                } label: {
                    HStack {
                        if let sel = viewModel.recentTrades.first(where: { $0.id == viewModel.journalCaptureTradeIdRaw }) {
                            Text("\(sel.symbol) \(sel.side.uppercased()) \(sel.qty)")
                                .font(.system(size: 11, weight: .medium, design: .monospaced))
                                .foregroundColor(.white)
                        } else {
                            Text("Select recent trade (optional)")
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundColor(Color.white.opacity(0.35))
                        }
                        Spacer()
                        Image(systemName: "chevron.up.chevron.down")
                            .font(.system(size: 9))
                            .foregroundColor(Color.white.opacity(0.3))
                            .accessibilityHidden(true)
                    }
                    .padding(10)
                    .background(journalChromeFill(opacity: linkLocked ? 0.03 : 0.06))
                    .cornerRadius(10)
                    .overlay(RoundedRectangle(cornerRadius: 10).stroke(Color.white.opacity(0.08), lineWidth: 0.5))
                }
                .disabled(linkLocked)
                .accessibilityLabel("Link trade")

                Text("Or enter UUID:")
                    .font(.system(size: 9, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.4))

                TextField("Trade UUID (optional if pending)", text: $viewModel.journalCaptureTradeIdRaw)
                    .font(.system(size: 11, weight: .regular, design: .monospaced))
                    .foregroundColor(.white)
                    .textFieldStyle(.plain)
                    .padding(10)
                    .background(journalChromeFill(opacity: linkLocked ? 0.03 : 0.06))
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color.white.opacity(0.08), lineWidth: 0.5)
                    )
                    .disabled(linkLocked)
                    .accessibilityLabel("Enter trade UUID manually")
            }
        }
    }

    private var captureHeader: some View {
        HStack {
            Text("CAPTURE")
                .font(.system(size: 9, weight: .bold, design: .rounded))
                .foregroundColor(Color(hex: "#00E5C0"))
                .tracking(1.0)
            Spacer()
        }
    }

    @ViewBuilder
    private var dictationAlerts: some View {
        if viewModel.dictationPermissionDenied {
            HStack(spacing: 8) {
                Image(systemName: "mic.slash")
                    .foregroundColor(Color(hex: "#F5A524"))
                    .accessibilityHidden(true)
                Text("Mic or dictation blocked.")
                    .font(.system(size: 9, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.55))
                Button("Settings") { viewModel.openDictationPrivacySettings() }
                    .buttonStyle(.plain)
                    .font(.system(size: 9, weight: .semibold))
                    .foregroundColor(Color(hex: "#00E5C0"))
                    .accessibilityLabel("Open dictation privacy settings")
            }
        }
        if viewModel.dictationOnDeviceOnlyUnsupported {
            Text("On-device dictation unavailable on this Mac/locale.")
                .font(.system(size: 9, weight: .medium, design: .rounded))
                .foregroundColor(Color.white.opacity(0.45))
        }
    }

    @ViewBuilder
    private var statusRows: some View {
        if let err = viewModel.journalCaptureLastError, !err.isEmpty {
            Text(err)
                .font(.system(size: 10, weight: .medium, design: .rounded))
                .foregroundColor(Color(hex: "#FF3B30"))
                .accessibilityLabel(err)
        }
        if let ok = viewModel.journalCaptureLastSuccess, !ok.isEmpty {
            Text(ok)
                .font(.system(size: 10, weight: .medium, design: .rounded))
                .foregroundColor(Color(hex: "#00E5C0"))
                .accessibilityLabel(ok)
        }
    }
}
