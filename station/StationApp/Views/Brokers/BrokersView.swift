import SwiftUI

public struct BrokersView: View {
    @ObservedObject private var viewModel: BrokersViewModel

    public init(viewModel: BrokersViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        DeskPageShell(
            title: "Brokers",
            subtitle: "Connect a broker to sync fills, balances, and open orders."
        ) {
            VStack(alignment: .leading, spacing: DeskChrome.Space.x2) {
                if let keychainGrantHint = viewModel.keychainGrantHint {
                    Text(keychainGrantHint)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.footnote))
                        .foregroundStyle(StationDS.Text.muted)
                }

                if let syncActionMessage = viewModel.syncActionMessage {
                    Text(syncActionMessage)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.callout))
                        .foregroundStyle(StationDS.Accent.amber)
                }

                ForEach(viewModel.cards) { card in
                    BrokerCardView(
                        card: card,
                        onConnect: {
                            Task { await viewModel.beginConnect(for: card.id) }
                        },
                        onEdit: { viewModel.presentEditSheet(for: card.id) },
                        onStart: {
                        guard let identity = card.identity else { return }
                        Task { await viewModel.startSync(for: identity) }
                    },
                        onStop: {
                        guard let identity = card.identity else { return }
                        Task { await viewModel.stopSync(for: identity) }
                    },
                        onDelete: {
                        guard let identity = card.identity else { return }
                        viewModel.requestDelete(for: identity)
                    }
                    )
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .task {
            await viewModel.load()
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(10))
                guard !Task.isCancelled else { break }
                await viewModel.load()
            }
        }
        .sheet(isPresented: $viewModel.isConnectSheetPresented) {
            BrokerConnectSheetView(viewModel: viewModel)
        }
        .alert(
            "Delete broker connection?",
            isPresented: $viewModel.isDeleteConfirmationPresented
        ) {
            Button("Cancel", role: .cancel) {
                viewModel.cancelDelete()
            }
            Button("Delete", role: .destructive) {
                Task { await viewModel.confirmDelete() }
            }
        } message: {
            Text(BrokerDeleteConfirmation.message)
        }
    }
}
