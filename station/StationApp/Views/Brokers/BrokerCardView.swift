import SwiftUI

struct BrokerCardView: View {
    let card: BrokerCardPresentation
    let onConnect: () -> Void
    let onEdit: () -> Void
    let onStart: () -> Void
    let onStop: () -> Void
    let onDelete: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(card.displayName)
                        .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)

                    HStack(spacing: 6) {
                        Text(card.assetClass.capitalized)
                            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                            .foregroundStyle(StationDS.Text.muted)
                        Text(card.quoteCurrency)
                            .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .medium))
                            .foregroundStyle(StationDS.Text.secondary)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(StationDS.Fill.input)
                            .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
                            .accessibilityLabel("Quote currency \(card.quoteCurrency)")
                    }
                }

                Spacer()

                statusBadge
            }

            if card.plannedLabel == nil || BrokerDogfoodProgram.allowsConnectWhilePlanned(slug: card.id) {
                if let validated = card.lastValidatedAtText {
                    metadataRow(label: "Last validated", value: validated)
                }

                if let synced = card.lastSyncedAtText {
                    metadataRow(label: "Last synced", value: synced)
                } else if let summary = card.lastSyncSummary {
                    metadataRow(label: "Sync", value: summary)
                }

                if card.isConnectable {
                    Button("Connect", action: onConnect)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                    .foregroundStyle(StationDS.Text.primary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(StationDS.Fill.input)
                    .overlay(
                        RoundedRectangle(cornerRadius: StationDS.Radius.small)
                            .stroke(StationDS.Border.outlineBtn, lineWidth: StationDS.borderThin)
                    )
                    .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
                }

                if let warning = card.permissionWarning {
                    Text(BrokerPermissionWarningCopy.label(for: warning))
                        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                        .foregroundStyle(StationDS.Accent.amber)
                }

                if card.identity != nil {
                    HStack(spacing: 8) {
                        if card.isEditEnabled {
                            controlButton(title: "Edit", enabled: true, action: onEdit)
                        }
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
        .background(StationDS.Fill.card)
        .overlay(
            RoundedRectangle(cornerRadius: StationDS.Radius.card)
                .stroke(StationDS.Border.card, lineWidth: StationDS.borderThin)
        )
        .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.card))
        .opacity(card.plannedLabel != nil ? 0.55 : 1)
    }

    private var statusBadge: some View {
        Text(card.plannedLabel ?? card.statusLabel)
            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .medium))
            .foregroundStyle(statusColor)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(statusColor.opacity(0.12))
            .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
    }

    private var statusColor: Color {
        switch card.status {
        case .connected, .syncing:
            return StationDS.Accent.green
        case .degraded, .rateLimited, .unavailableAgentOffline:
            return StationDS.Accent.amber
        case .failed:
            return StationDS.Accent.red
        case .paused, .readyToStart, .validating:
            return StationDS.Accent.blue
        case .notConfigured:
            return StationDS.Text.secondary
        }
    }

    private func metadataRow(label: String, value: String) -> some View {
        HStack(spacing: 6) {
            Text(label + ":")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                .foregroundStyle(StationDS.Text.labels)
            Text(value)
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .regular))
                .foregroundStyle(StationDS.Text.secondary)
        }
    }

    private func controlButton(title: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(title, action: action)
            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
            .foregroundStyle(enabled ? StationDS.Text.primary : StationDS.Text.muted)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(StationDS.Fill.input)
            .overlay(
                RoundedRectangle(cornerRadius: StationDS.Radius.small)
                    .stroke(StationDS.Border.outlineBtn, lineWidth: StationDS.borderThin)
            )
            .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
            .disabled(!enabled)
    }
}
