import Notch
import SwiftUI

/// Station Settings: local toolbar-capture outbox ACK / dead-letter visibility (Wave 0.2).
struct CaptureOutboxSettingsSection: View {
    @State private var snapshot: CaptureOutboxStatusSnapshot?
    @State private var loadError: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let loadError, !loadError.isEmpty {
                Text(loadError)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Accent.red)
            } else if let snapshot {
                Text(outboxSummary(snapshot.counts))
                    .font(StationDS.bodyFont(StationDS.FontSize.body))
                    .foregroundStyle(
                        snapshot.counts.deadLetter > 0
                            ? StationDS.Accent.amber
                            : StationDS.Text.primary
                    )
                ForEach(snapshot.deadLetters.prefix(10)) { item in
                    HStack(alignment: .top, spacing: 12) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.draftText?.isEmpty == false ? item.draftText! : "Capture")
                                .font(StationDS.bodyFont(StationDS.FontSize.body))
                                .foregroundStyle(StationDS.Text.primary)
                                .lineLimit(1)
                            if let key = item.idempotencyKey, !key.isEmpty {
                                Text(key)
                                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                                    .foregroundStyle(StationDS.Text.muted)
                                    .lineLimit(1)
                            }
                        }
                        Spacer(minLength: 8)
                        Text(item.reason)
                            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .medium))
                            .foregroundStyle(StationDS.Accent.red)
                            .multilineTextAlignment(.trailing)
                    }
                    .padding(.vertical, 4)
                }
                if snapshot.deadLetters.isEmpty, snapshot.counts.deadLetter == 0 {
                    Text("No failed captures in the local outbox.")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                }
            } else {
                Text("Loading capture delivery status…")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            }
        }
        .task { await refresh() }
    }

    private func outboxSummary(_ counts: CaptureOutboxCounts) -> String {
        let queued = counts.enqueued + counts.inflight
        let parts = [
            queued > 0 ? "\(queued) queued" : nil,
            counts.acked > 0 ? "\(counts.acked) acked" : nil,
            counts.deadLetter > 0 ? "\(counts.deadLetter) dead letter" : nil,
        ].compactMap { $0 }
        if parts.isEmpty { return "Capture outbox empty on this Mac." }
        return "Capture outbox · " + parts.joined(separator: " · ")
    }

    private func refresh() async {
        let secret = AgentDaemonSecret.resolveForSession()
        guard let snap = await CaptureOutboxStatusClient.fetch(
            port: AgentLoopback.port,
            daemonSecret: secret
        ) else {
            loadError = "Agent did not return capture outbox status."
            return
        }
        loadError = nil
        snapshot = snap
    }
}
