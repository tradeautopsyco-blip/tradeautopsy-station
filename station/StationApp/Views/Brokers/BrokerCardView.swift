import SwiftUI

struct BrokerCardView: View {
    let card: BrokerCardPresentation
    let onConnect: () -> Void
    let onEdit: () -> Void
    let onStart: () -> Void
    let onStop: () -> Void
    let onDelete: () -> Void

    var body: some View {
        DeskCard(title: card.displayName, status: statusPill) {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x1) {
                if let booksLine = card.booksLine {
                    Text(booksLine)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }

                HStack(spacing: 6) {
                    Text(card.assetClass.capitalized)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.muted)
                    Text(card.quoteCurrency)
                        .font(DeskChrome.mono(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.secondary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(StationDS.Fill.input)
                        .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
                        .accessibilityLabel("Quote currency \(card.quoteCurrency)")
                }

                if card.plannedLabel == nil
                    || BrokerDogfoodProgram.showsConnectBetaBadge(slug: card.id)
                    || BrokerDogfoodProgram.allowsConnectWhilePlanned(slug: card.id) {
                    if let validated = card.lastValidatedAtText {
                        metadataRow(label: "Last validated", value: validated)
                    }

                    if let synced = card.lastSyncedAtText {
                        metadataRow(label: "Last synced", value: synced)
                    } else if let summary = card.lastSyncSummary {
                        metadataRow(label: "Sync", value: summary)
                    }

                    if let warning = card.permissionWarning {
                        Text(BrokerPermissionWarningCopy.label(for: warning))
                            .font(DeskChrome.sans(DeskChrome.TypeScale.callout))
                            .foregroundStyle(StationDS.Accent.amber)
                    }
                }
            }
        } footer: {
            if card.plannedLabel == nil
                || BrokerDogfoodProgram.showsConnectBetaBadge(slug: card.id)
                || BrokerDogfoodProgram.allowsConnectWhilePlanned(slug: card.id) {
                HStack(spacing: DeskChrome.Space.x1) {
                    if card.isConnectable {
                        Button("Connect", action: onConnect)
                            .buttonStyle(.deskPrimary)
                    }
                    if card.identity != nil {
                        if card.isEditEnabled {
                            Button("Edit", action: onEdit)
                                .buttonStyle(.deskSecondary)
                        }
                        Button("Start", action: onStart)
                            .buttonStyle(.deskSecondary)
                            .disabled(!card.isStartEnabled)
                        Button("Stop", action: onStop)
                            .buttonStyle(.deskSecondary)
                            .disabled(!card.isStopEnabled)
                        Button("Delete", role: .destructive, action: onDelete)
                            .buttonStyle(.deskDestructive)
                            .disabled(!card.isDeleteEnabled)
                    }
                }
            }
        }
        .opacity(card.plannedLabel != nil && card.identity == nil ? 0.55 : 1)
    }

    private var statusPill: DeskStatusPill {
        DeskStatusPill(text: card.plannedLabel ?? card.statusLabel, tone: pillTone)
    }

    private var pillTone: DeskStatusPill.Tone {
        switch card.status {
        case .connected, .syncing: return .success
        case .failed: return .danger
        case .degraded, .rateLimited, .unavailableAgentOffline: return .warning
        default: return .neutral
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

}
