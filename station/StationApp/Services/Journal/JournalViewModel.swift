import Combine
import Foundation

@MainActor
public final class JournalViewModel: ObservableObject {
    @Published public private(set) var week: JournalWeek = .empty()
    @Published public var facet: JournalFacet = .all {
        didSet { rebuild() }
    }
    @Published public var query: String = "" {
        didSet { rebuild() }
    }
    @Published public var selectedDay: String? {
        didSet { rebuild() }
    }
    @Published public var selectedCardId: String?
    @Published public var selectedSheet: Bool = false
    @Published public private(set) var isLoading = false

    private let client: JournalAgentClient
    private let inventoryClient: JournalInventoryAgentClient
    private let sessionModel: SessionModel
    private let agentHealthy: () -> Bool
    private let demoDeskStore: DemoDeskStore
    private var lastPayload: JournalWeekPayload?
    private var lastInventoryRows: [JournalClosedTripInventoryRow] = []
    private var positionsCancellable: AnyCancellable?
    private var demoCancellable: AnyCancellable?
    private var demoPositions: [DeskPosition] = []
    private var demoInventoryRows: [JournalClosedTripInventoryRow] = []

    public init(
        client: JournalAgentClient,
        inventoryClient: JournalInventoryAgentClient? = nil,
        sessionModel: SessionModel,
        agentHealthy: @escaping () -> Bool,
        demoDeskStore: DemoDeskStore? = nil
    ) {
        self.client = client
        self.inventoryClient = inventoryClient
            ?? LocalJournalInventoryAgentClient(isAgentHealthy: agentHealthy)
        self.sessionModel = sessionModel
        self.agentHealthy = agentHealthy
        let demo = demoDeskStore ?? .shared
        self.demoDeskStore = demo
        positionsCancellable = sessionModel.$positions
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self, !self.demoDeskStore.demoEnabled else { return }
                self.rebuild()
            }
        demoCancellable = demo.$demoEnabled
            .dropFirst()
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                Task { await self?.load() }
            }
    }

    public var sidebarDue: Bool { week.sidebarDue }

    /// Pending/matched plus unmatched — Detect join applies the Journal impulsive coverage rule.
    public var weekDeclarations: [JournalDeclarationCard] {
        lastPayload?.items ?? []
    }

    public var selectedCard: JournalDeclarationCard? {
        guard let selectedCardId else { return nil }
        return week.declarations.first { $0.id == selectedCardId }
            ?? lastPayload?.items.first { $0.id == selectedCardId }
    }

    public var exportURL: URL? {
        guard let path = week.sheet?.markdownExportPath, !path.isEmpty else { return nil }
        let base = sessionModel.webBaseURL.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        return URL(string: base + path)
    }

    public func load() async {
        isLoading = true
        defer { isLoading = false }
        if demoDeskStore.demoEnabled {
            let fixture = StationDemoDesk.build()
            lastPayload = fixture.journalPayload
            demoPositions = fixture.positions
            demoInventoryRows = fixture.journalTripInventoryRows
            selectedDay = lastPayload?.days.last?.localDate ?? lastPayload?.items.last?.localDate
            rebuild()
            return
        }
        demoPositions = []
        demoInventoryRows = []
        if !agentHealthy() {
            lastPayload = nil
            lastInventoryRows = []
            rebuild()
            return
        }
        lastPayload = await client.fetchWeek()
        lastInventoryRows = await inventoryClient.fetchWeekTripCites(
            weekStart: lastPayload?.weekStart,
            weekEnd: lastPayload?.weekEnd
        )
        if selectedDay == nil {
            selectedDay = lastPayload?.days.last?.localDate ?? lastPayload?.items.last?.localDate
        }
        rebuild()
    }

    private func rebuild() {
        let inventoryRows = demoDeskStore.demoEnabled ? demoInventoryRows : lastInventoryRows
        let citedTrips = JournalTripCiteInventory.citedTrips(
            rows: inventoryRows,
            deskQuoteCurrency: sessionModel.deskQuoteCurrency
        )
        week = JournalWeek.build(
            payload: lastPayload,
            inventory: demoDeskStore.demoEnabled ? demoPositions : sessionModel.positions,
            citedTrips: citedTrips,
            selectedDay: selectedDay,
            facet: facet,
            query: query
        )
    }
}
