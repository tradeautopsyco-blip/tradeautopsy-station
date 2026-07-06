import Notch
import SwiftUI

public struct BrokersView: View {
    @ObservedObject private var viewModel: BrokersViewModel

    public init(viewModel: BrokersViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Brokers")
                    .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .medium))
                    .foregroundStyle(BarDS.Text.primary)

                Text("Connect a broker to sync fills, balances, and open orders.")
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundStyle(BarDS.Text.muted)

                ForEach(viewModel.cards) { card in
                    BrokerCardView(
                        card: card,
                        onConnect: { viewModel.presentConnectSheet(for: card.id) },
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
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
        .task {
            await viewModel.load()
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
