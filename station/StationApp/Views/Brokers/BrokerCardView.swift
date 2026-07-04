import Notch
import SwiftUI

struct BrokerCardView: View {
    let card: BrokerCardPresentation
    let onConnect: () -> Void
    let onStart: () -> Void
    let onStop: () -> Void
    let onDelete: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(card.displayName)
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                        .foregroundStyle(BarDS.Text.primary)

                    Text(card.assetClass.capitalized)
                        .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                        .foregroundStyle(BarDS.Text.muted)
                }

                Spacer()

                statusBadge
            }

            if card.plannedLabel == nil {
                if let validated = card.lastValidatedAtText {
                    metadataRow(label: "Last validated", value: validated)
                }

                if let summary = card.lastSyncSummary {
                    metadataRow(label: "Sync", value: summary)
                }

                if card.isConnectable {
                    Button("Connect", action: onConnect)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                    .foregroundStyle(BarDS.Text.primary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(BarDS.Fill.input)
                    .overlay(
                        RoundedRectangle(cornerRadius: BarDS.Radius.small)
                            .stroke(BarDS.Border.outlineBtn, lineWidth: BarDS.borderThin)
                    )
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small))
                }

                if let warning = card.permissionWarning {
                    Text(BrokerPermissionWarningCopy.label(for: warning))
                        .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                        .foregroundStyle(BarDS.Accent.amber)
                }

                if card.identity != nil {
                    HStack(spacing: 8) {
                        controlButton(title: "Start", enabled: card.isStartEnabled, action: onStart)
                        controlButton(title: "Stop", enabled: card.isStopEnabled, action: onStop)
                        controlButton(
                            title: "Delete",
                            enabled: card.isDeleteEnabled,
                            action: onDelete
                        )
                    }
                }
            }
        }
        .padding(16)
        .background(BarDS.Fill.card)
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card))
        .opacity(card.plannedLabel != nil ? 0.55 : 1)
    }

    private var statusBadge: some View {
        Text(card.plannedLabel ?? card.statusLabel)
            .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
            .foregroundStyle(statusColor)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(statusColor.opacity(0.12))
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small))
    }

    private var statusColor: Color {
        switch card.status {
        case .syncing:
            return BarDS.Accent.green
        case .degraded, .rateLimited, .unavailableAgentOffline:
            return BarDS.Accent.amber
        case .failed:
            return BarDS.Accent.red
        case .paused, .readyToStart, .validating:
            return BarDS.Accent.blue
        case .notConfigured:
            return BarDS.Text.secondary
        }
    }

    private func metadataRow(label: String, value: String) -> some View {
        HStack(spacing: 6) {
            Text(label + ":")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                .foregroundStyle(BarDS.Text.labels)
            Text(value)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                .foregroundStyle(BarDS.Text.secondary)
        }
    }

    private func controlButton(title: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(title, action: action)
            .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
            .foregroundStyle(enabled ? BarDS.Text.primary : BarDS.Text.muted)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(BarDS.Fill.input)
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small)
                    .stroke(BarDS.Border.outlineBtn, lineWidth: BarDS.borderThin)
            )
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small))
            .disabled(!enabled)
    }
}
