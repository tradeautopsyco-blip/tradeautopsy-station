import SwiftUI
import AppKit

struct UnpostedCaptureTrayView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if viewModel.unpostedSweepDropped > 0 {
                Text("Oldest local shots were dropped (20 or 7 days).")
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.amber)
                    .padding(.bottom, 8)
            }
            captureDeliverySummary
            if !viewModel.unpostedCaptures.isEmpty {
                escrowRow(
                    key: "Unposted",
                    value: "\(viewModel.unpostedCaptures.count) · this Mac only",
                    valueColor: BarDS.Accent.teal
                )
                ForEach(viewModel.unpostedCaptures) { rec in
                    unpostedItemRow(rec)
                }
                Text("Link to today’s trade")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .padding(.top, 8)
                    .padding(.bottom, 6)
                BarInputField(placeholder: "Search symbol", text: $viewModel.unpostedSearch)
                ForEach(viewModel.linkableRecentTrades.prefix(8)) { trade in
                    tradeLinkRow(trade)
                }
            }
        }
        .task {
            await viewModel.fetchRecentTrades()
            await viewModel.refreshCaptureOutboxStatus()
        }
    }

    @ViewBuilder
    private var captureDeliverySummary: some View {
        if let err = viewModel.captureOutboxStatusError, !err.isEmpty {
            Text(err)
                .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
                .padding(.bottom, 6)
        } else if let status = viewModel.captureOutboxStatus {
            let queued = status.counts.enqueued + status.counts.inflight
            if queued > 0 || status.counts.deadLetter > 0 {
                let parts = [
                    queued > 0 ? "\(queued) delivering" : nil,
                    status.counts.deadLetter > 0 ? "\(status.counts.deadLetter) failed" : nil,
                ].compactMap { $0 }
                Text("Capture outbox · \(parts.joined(separator: " · "))")
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(status.counts.deadLetter > 0 ? BarDS.Accent.amber : BarDS.Text.secondary)
                    .padding(.bottom, 6)
            }
            ForEach(status.deadLetters.prefix(4)) { item in
                HStack {
                    Text(item.draftText?.isEmpty == false ? item.draftText! : "Failed capture")
                        .lineLimit(1)
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                        .foregroundColor(BarDS.Text.primary)
                    Spacer(minLength: 8)
                    Text(item.reason)
                        .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                        .foregroundColor(BarDS.Accent.red)
                        .lineLimit(1)
                }
                .padding(.bottom, 4)
            }
        }
    }

    private func unpostedItemRow(_ rec: UnpostedCaptureRecord) -> some View {
        HStack(alignment: .center, spacing: 8) {
            Image(nsImage: NSImage(contentsOf: viewModel.unpostedFileURL(for: rec)) ?? NSImage())
                .resizable()
                .aspectRatio(contentMode: .fill)
                .frame(width: 36, height: 28)
                .clipped()
                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
                )
            Text(rec.caption.isEmpty ? "Chart" : rec.caption)
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundColor(BarDS.Text.primary)
                .lineLimit(1)
            Spacer(minLength: 8)
            if let err = rec.lastError, !err.isEmpty {
                Text(err)
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
                    .lineLimit(2)
                    .help(err)
            } else {
                Text(Self.timeFmt.string(from: rec.createdAt))
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
            }
            Button("Delete") {
                viewModel.deleteUnpostedCapture(rec.id)
            }
            .buttonStyle(.plain)
            .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
            .foregroundColor(BarDS.Text.hint)
            .accessibilityLabel("Delete local screenshot")
        }
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(BarDS.Border.row)
                .frame(height: BarDS.borderThin)
        }
    }

    private func tradeLinkRow(_ trade: RecentTradeRow) -> some View {
        let hasChart = viewModel.tradeHasChart(trade.id)
        let busy = viewModel.unpostedSendBusyId != nil
        return Button {
            viewModel.requestLinkUnposted(to: trade.id)
        } label: {
            HStack {
                Text(tradeRowLabel(trade))
                    .font(BarDS.monoFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundColor(BarDS.Text.primary)
                Spacer()
                if viewModel.unpostedSendBusyId != nil, viewModel.replaceConfirmTradeId == nil {
                    ProgressView().controlSize(.mini)
                } else {
                    Text(hasChart ? "Has chart" : "Send")
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                        .foregroundColor(hasChart ? BarDS.Accent.amber : BarDS.Accent.teal)
                }
            }
            .padding(.vertical, 10)
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(BarDS.Border.row)
                    .frame(height: BarDS.borderThin)
            }
        }
        .buttonStyle(.plain)
        .disabled(busy)
        .accessibilityLabel(hasChart ? "Replace chart on \(trade.symbol)" : "Send chart to \(trade.symbol)")
    }

    private func tradeRowLabel(_ trade: RecentTradeRow) -> String {
        let side = trade.side.trimmingCharacters(in: .whitespacesAndNewlines)
        if side.isEmpty { return trade.symbol }
        return "\(trade.symbol) · \(side.uppercased())"
    }

    private func escrowRow(key: String, value: String, valueColor: Color) -> some View {
        HStack {
            Text(key)
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
            Spacer()
            Text(value)
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                .foregroundColor(valueColor)
        }
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(BarDS.Border.row)
                .frame(height: BarDS.borderThin)
        }
    }

    private static let timeFmt: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "HH:mm"
        return f
    }()
}
