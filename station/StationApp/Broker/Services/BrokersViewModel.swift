import Foundation

@MainActor
public final class BrokersViewModel: ObservableObject {
    @Published public private(set) var cards: [BrokerCardPresentation] = []
    @Published public var isConnectSheetPresented = false
    @Published public private(set) var connectApiKey = ""
    @Published public private(set) var connectApiSecret = ""
    @Published public private(set) var connectMessage: String?
    @Published public private(set) var connectInvalidFields: Set<BrokerCredentialField> = []
    @Published public private(set) var isConnecting = false

    private let brokerControl: BrokerControlling
    private let connectController: BrokerConnectController

    public var connectDisclosure: String {
        BrokerConnectDisclosure.message(
            behavioralAnalysisOptedOut: connectController.behavioralAnalysisOptedOut
        )
    }

    public init(
        brokerControl: BrokerControlling,
        connectController: BrokerConnectController
    ) {
        self.brokerControl = brokerControl
        self.connectController = connectController
    }

    public func load() async {
        let snapshot = await brokerControl.loadSnapshot()
        cards = Self.applyValidatingOverlay(
            cards: BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1),
            isValidating: isConnecting
        )
    }

    private static func applyValidatingOverlay(
        cards: [BrokerCardPresentation],
        isValidating: Bool
    ) -> [BrokerCardPresentation] {
        guard isValidating else { return cards }
        return cards.map { card in
            guard card.id == "binance_us", card.status == .notConfigured else { return card }
            return BrokerCardPresentation(
                id: card.id,
                displayName: card.displayName,
                assetClass: card.assetClass,
                status: .validating,
                statusLabel: BrokerCardStatus.validating.rawValue,
                isConnectable: card.isConnectable,
                isStartEnabled: false,
                isStopEnabled: false,
                isDeleteEnabled: false,
                lastValidatedAtText: card.lastValidatedAtText,
                lastSyncSummary: card.lastSyncSummary,
                plannedLabel: card.plannedLabel,
                permissionWarning: card.permissionWarning,
                identity: card.identity
            )
        }
    }

    public func presentConnectSheet() {
        connectApiKey = ""
        connectApiSecret = ""
        connectMessage = nil
        connectInvalidFields = []
        connectController.updateFields(apiKey: "", apiSecret: "")
        isConnectSheetPresented = true
    }

    public func updateConnectFields(apiKey: String, apiSecret: String) {
        connectApiKey = apiKey
        connectApiSecret = apiSecret
        connectController.updateFields(apiKey: apiKey, apiSecret: apiSecret)
    }

    public func submitConnect() async {
        isConnecting = true
        cards = Self.applyValidatingOverlay(cards: cards, isValidating: true)
        defer { isConnecting = false }

        let outcome = await connectController.connect()
        switch outcome {
        case .localValidationFailed(let invalidFields):
            connectInvalidFields = invalidFields
            connectMessage = "Enter both API key and secret."
        case .blockedWithdrawPermission:
            connectMessage = "Withdraw permission detected. Use a key without withdraw access."
        case .validationTransientFailure(let failure):
            connectMessage = transientFailureMessage(failure)
        case .validationPermanentFailure(let failure):
            connectMessage = permanentFailureMessage(failure)
        case .connected:
            connectMessage = nil
            isConnectSheetPresented = false
        }

        await load()
    }

    public func startSync(for identity: BrokerConnectionIdentity) async {
        try? await brokerControl.startSync(for: identity)
        await load()
    }

    public func stopSync(for identity: BrokerConnectionIdentity) async {
        try? await brokerControl.stopSync(for: identity)
        await load()
    }

    @Published public var pendingDeleteIdentity: BrokerConnectionIdentity?
    @Published public var isDeleteConfirmationPresented = false

    public func requestDelete(for identity: BrokerConnectionIdentity) {
        pendingDeleteIdentity = identity
        isDeleteConfirmationPresented = true
    }

    public func confirmDelete() async {
        guard let identity = pendingDeleteIdentity else { return }
        defer {
            pendingDeleteIdentity = nil
            isDeleteConfirmationPresented = false
        }
        try? await brokerControl.deleteConnection(
            for: identity,
            connectController: connectController
        )
        await load()
    }

    public func cancelDelete() {
        pendingDeleteIdentity = nil
        isDeleteConfirmationPresented = false
    }

    private func transientFailureMessage(_ failure: BrokerCredentialValidationFailure) -> String {
        switch failure {
        case .networkUnavailable:
            return "Network unavailable. Your typed values stay in this session; try again when online."
        case .rateLimited:
            return "Binance.US rate limited validation. Try again shortly."
        case .brokerUnavailable:
            return "Binance.US validation is unavailable right now. Your typed values stay in this session."
        case .invalidCredentials:
            return "Could not validate credentials."
        }
    }

    private func permanentFailureMessage(_ failure: BrokerCredentialValidationFailure) -> String {
        switch failure {
        case .invalidCredentials:
            return "Credentials were rejected by Binance.US."
        case .networkUnavailable, .rateLimited, .brokerUnavailable:
            return transientFailureMessage(failure)
        }
    }
}
