import SwiftUI

/// Broker-miss lane — founder enters a fill; agent stores locally + queues journal capture.
struct BarManualFillPanel: View {
    @ObservedObject var viewModel: NotchViewModel
    var liveState: BarLiveStateResponse?

    @State private var symbol: String = ""
    @State private var sideBuy: Bool = true
    @State private var quantity: String = ""
    @State private var price: String = ""
    @State private var filledAt: Date = Date()
    /// Nil = pending escrow. Set only when the founder picks a Console `trades.id`.
    @State private var linkedConsoleTradeId: String? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("MANUAL FILL")
                .font(.system(size: 10, weight: .semibold, design: .monospaced))
                .foregroundColor(Color.white.opacity(0.38))
                .tracking(0.6)

            Text("Broker miss — records on this Mac and queues Console journal. No orders.")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: 6) {
                manualField("Symbol", text: $symbol)
                sideToggle
            }

            HStack(spacing: 6) {
                manualField("Qty", text: $quantity)
                manualField("Price", text: $price)
            }

            Text("Link to today’s trade")
                .font(BarDS.monoFont(9, weight: .medium))
                .foregroundColor(BarDS.Text.hint)

            linkChip(title: "Pending — no Console trade", on: linkedConsoleTradeId == nil) {
                linkedConsoleTradeId = nil
            }
            ForEach(viewModel.manualFillConsoleTrades.prefix(8)) { trade in
                linkChip(title: tradeLabel(trade), on: linkedConsoleTradeId == trade.id) {
                    linkedConsoleTradeId = trade.id
                }
            }

            DatePicker(
                "Time",
                selection: $filledAt,
                displayedComponents: [.date, .hourAndMinute]
            )
            .labelsHidden()
            .datePickerStyle(.compact)
            .font(BarDS.monoFont(11, weight: .regular))
            .colorScheme(.dark)

            if let err = viewModel.manualFillLastError, !err.isEmpty {
                Text(err)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
            } else if let ok = viewModel.manualFillLastSuccess, !ok.isEmpty {
                Text(ok)
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.teal)
            }

            Button(action: submit) {
                HStack {
                    if viewModel.manualFillBusy {
                        ProgressView().controlSize(.small)
                    }
                    Text(viewModel.manualFillBusy ? "Queuing…" : "Queue to journal")
                        .font(BarDS.monoFont(11, weight: .semibold))
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 8)
                .background(Color.white.opacity(0.06))
                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            }
            .buttonStyle(.plain)
            .disabled(viewModel.manualFillBusy || !canSubmit)
        }
        .padding(10)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
        .onAppear {
            applyDefaults()
            Task { await viewModel.refreshManualFillConsoleTrades() }
        }
        .onChange(of: liveState?.pendingDeclaration?.id) { _, _ in applyDefaults() }
    }

    private var canSubmit: Bool {
        !symbol.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && Double(quantity.trimmingCharacters(in: .whitespacesAndNewlines)) != nil
            && Double(price.trimmingCharacters(in: .whitespacesAndNewlines)) != nil
    }

    private func applyDefaults() {
        let d = ManualFillJournalDraft.defaults(
            pending: liveState?.pendingDeclaration,
            fallbackSymbol: viewModel.barDeclarationSymbol,
            fallbackDeclarationId: viewModel.barPublishedDeclarationId
        )
        if symbol.isEmpty { symbol = d.symbol }
        sideBuy = d.sideBuy
        if quantity.isEmpty, !d.quantity.isEmpty { quantity = d.quantity }
        filledAt = d.filledAt
    }

    private func submit() {
        guard
            let qty = Double(quantity.trimmingCharacters(in: .whitespacesAndNewlines)),
            let px = Double(price.trimmingCharacters(in: .whitespacesAndNewlines)),
            qty > 0, px > 0
        else {
            viewModel.manualFillLastError = "Qty and price must be positive numbers."
            return
        }
        let decl = ManualFillJournalDraft.defaults(
            pending: liveState?.pendingDeclaration,
            fallbackSymbol: viewModel.barDeclarationSymbol,
            fallbackDeclarationId: viewModel.barPublishedDeclarationId
        ).declarationId
        Task {
            await viewModel.submitManualFillJournal(
                symbol: symbol,
                sideBuy: sideBuy,
                quantity: qty,
                price: px,
                filledAt: filledAt,
                declarationId: decl,
                consoleTradeId: linkedConsoleTradeId
            )
        }
    }

    private func manualField(_ title: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(BarDS.monoFont(9, weight: .medium))
                .foregroundColor(BarDS.Text.hint)
            TextField(title, text: text)
                .textFieldStyle(.plain)
                .font(BarDS.monoFont(12, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
                .background(Color.white.opacity(0.05))
                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
        }
    }

    private var sideToggle: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("Side")
                .font(BarDS.monoFont(9, weight: .medium))
                .foregroundColor(BarDS.Text.hint)
            HStack(spacing: 4) {
                sideChip("BUY", on: sideBuy) { sideBuy = true }
                sideChip("SELL", on: !sideBuy) { sideBuy = false }
            }
        }
    }

    private func tradeLabel(_ trade: ConsoleJournalTradeRow) -> String {
        let side = trade.side.trimmingCharacters(in: .whitespacesAndNewlines)
        let tail = String(trade.id.prefix(8))
        if side.isEmpty { return "\(trade.symbol) · \(tail)…" }
        return "\(trade.symbol) · \(side.uppercased()) · \(tail)…"
    }

    private func linkChip(title: String, on: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.monoFont(10, weight: .semibold))
                .foregroundColor(on ? BarDS.Accent.teal : BarDS.Text.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
                .background(on ? Color.white.opacity(0.08) : Color.white.opacity(0.03))
                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
        }
        .buttonStyle(.plain)
    }

    private func sideChip(_ label: String, on: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(label)
                .font(BarDS.monoFont(10, weight: .semibold))
                .foregroundColor(on ? BarDS.Accent.teal : BarDS.Text.secondary)
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
                .background(on ? Color.white.opacity(0.08) : Color.white.opacity(0.03))
                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
        }
        .buttonStyle(.plain)
    }
}
