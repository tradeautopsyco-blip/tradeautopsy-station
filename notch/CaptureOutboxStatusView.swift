import SwiftUI

/// Outbox counts + named dead-letter reasons (Wave 0.2 capture ACK).
public struct CaptureOutboxStatusView: View {
    public let snapshot: CaptureOutboxStatusSnapshot?
    public let loadError: String?
    public var onRefresh: (() async -> Void)?

    public init(
        snapshot: CaptureOutboxStatusSnapshot?,
        loadError: String?,
        onRefresh: (() async -> Void)? = nil
    ) {
        self.snapshot = snapshot
        self.loadError = loadError
        self.onRefresh = onRefresh
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let loadError, !loadError.isEmpty {
                Text(loadError)
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
            } else if let snapshot {
                summaryRow(snapshot.counts)
                if !snapshot.deadLetters.isEmpty {
                    ForEach(snapshot.deadLetters.prefix(8)) { item in
                        deadLetterRow(item)
                    }
                } else if snapshot.counts.deadLetter == 0 {
                    Text("No failed captures on this Mac.")
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                        .foregroundColor(BarDS.Text.secondary)
                }
            } else {
                Text("Loading capture delivery…")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
            }
        }
        .task {
            await onRefresh?()
        }
    }

    private func summaryRow(_ counts: CaptureOutboxCounts) -> some View {
        let queued = counts.enqueued + counts.inflight
        let parts = [
            queued > 0 ? "\(queued) queued" : nil,
            counts.acked > 0 ? "\(counts.acked) acked" : nil,
            counts.deadLetter > 0 ? "\(counts.deadLetter) failed" : nil,
        ].compactMap { $0 }
        let line = parts.isEmpty ? "No captures in the local outbox." : parts.joined(separator: " · ")
        return Text(line)
            .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
            .foregroundColor(counts.deadLetter > 0 ? BarDS.Accent.amber : BarDS.Text.primary)
    }

    private func deadLetterRow(_ item: CaptureOutboxDeadLetter) -> some View {
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(item.draftText?.isEmpty == false ? item.draftText! : "Capture")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundColor(BarDS.Text.primary)
                    .lineLimit(1)
                if let key = item.idempotencyKey, !key.isEmpty {
                    Text(key)
                        .font(BarDS.monoFont(BarDS.FontSize.sectionLabel, weight: .regular))
                        .foregroundColor(BarDS.Text.hint)
                        .lineLimit(1)
                }
            }
            Spacer(minLength: 8)
            Text(item.reason)
                .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
                .multilineTextAlignment(.trailing)
        }
        .padding(.vertical, 6)
    }
}
