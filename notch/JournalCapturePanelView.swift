import SwiftUI

struct JournalCapturePanelView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceMotion) private var accessibilityReduceMotion
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @FocusState private var editorFocused: Bool

    private var linkLocked: Bool {
        viewModel.journalCaptureLinkLocked
    }

    private var showSessionBanner: Bool {
        !viewModel.isAuthenticated
            || viewModel.sessionState == "expired"
            || viewModel.sessionState == "signed_out"
    }

    var body: some View {
        captureScrollContent
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .onAppear {
                viewModel.updateDictationReduceMotion(accessibilityReduceMotion)
            }
            .onChange(of: accessibilityReduceMotion) { v in
                viewModel.updateDictationReduceMotion(v)
            }
    }

    private var captureScrollContent: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                if showSessionBanner {
                    HStack(spacing: 8) {
                        Image(systemName: "person.crop.circle.badge.exclamationmark")
                            .foregroundColor(Color(hex: "#FF9500"))
                            .font(.system(size: 12))
                            .accessibilityHidden(true)
                        Text("Session expired")
                            .font(.system(size: 10, weight: .semibold, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.85))
                        Spacer()
                        Button("Sign in") {
                            viewModel.openDeepLink(viewModel.webBaseURL + "/login?toolbar_reauth=1")
                        }
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundColor(Color(hex: "#00E5C0"))
                        .buttonStyle(.plain)
                        .accessibilityLabel("Sign in to TradeAutopsy")
                    }
                    .padding(10)
                    .background(Color(hex: "#FF9500").opacity(0.08))
                    .cornerRadius(10)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color(hex: "#FF9500").opacity(0.2), lineWidth: 0.5)
                    )
                }

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
                    .accessibilityLabel("Journal entry text field")
                    .accessibilityHint("Describe what happened in this trade")

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
                .accessibilityLabel("Save as pending capture")
                .accessibilityHint("Use when no trade is linked yet")
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
                        .accessibilityLabel("Attach screenshot")
                        .accessibilityHint("Captures a region of your screen and attaches it to this journal entry")
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
                    .accessibilityLabel("Save draft locally")
                    .accessibilityHint("Saves this text on your Mac without sending it to the server")

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
                    .accessibilityHint("Saves this note permanently and queues it for processing")
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
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .background(
                Group {
                    if reduceTransparency {
                        Color(hex: "#0d0d0d").opacity(0.97)
                    } else {
                        Color.clear.background(.ultraThinMaterial).opacity(0.18)
                    }
                }
            )
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
                    .accessibilityLabel("Trade UUID text field")
            } else {
                Picker("Link to trade", selection: $viewModel.journalCaptureTradeIdRaw) {
                    Text("None").tag("")
                    ForEach(viewModel.recentTrades) { t in
                        Text("\(t.symbol) \(t.side) \(t.qty) @ \(String(format: "%.2f", t.price))")
                            .tag(t.id)
                            .font(.system(size: 11, design: .monospaced))
                    }
                }
                .pickerStyle(.menu)
                .disabled(linkLocked)
                .font(.system(size: 11, design: .monospaced))
                .foregroundColor(.white)
                .accessibilityLabel("Link to recent trade")
            }
        }
    }

    private var captureHeader: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("CAPTURE")
                    .font(.system(size: 9, weight: .bold, design: .rounded))
                    .foregroundColor(Color(hex: "#00E5C0"))
                    .tracking(1.0)
                Spacer()
            }
            Text("Cross-link with Circuit: finalize from hosted web capture when signed in.")
                .font(.system(size: 8, weight: .medium, design: .rounded))
                .foregroundColor(Color.white.opacity(0.42))
                .fixedSize(horizontal: false, vertical: true)
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
