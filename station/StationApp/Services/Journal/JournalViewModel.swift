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
    private let sessionModel: SessionModel
    private let agentHealthy: () -> Bool
    private var lastPayload: JournalWeekPayload?
    private var positionsCancellable: AnyCancellable?

    public init(
        client: JournalAgentClient,
        sessionModel: SessionModel,
        agentHealthy: @escaping () -> Bool
    ) {
        self.client = client
        self.sessionModel = sessionModel
        self.agentHealthy = agentHealthy
        positionsCancellable = sessionModel.$positions
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.rebuild()
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
        if !agentHealthy() {
            lastPayload = nil
            rebuild()
            return
        }
        lastPayload = await client.fetchWeek()
        if selectedDay == nil {
            selectedDay = lastPayload?.days.last?.localDate ?? lastPayload?.items.last?.localDate
        }
        rebuild()
    }

    private func rebuild() {
        week = JournalWeek.build(
            payload: lastPayload,
            inventory: sessionModel.positions,
            citedTrips: [],
            selectedDay: selectedDay,
            facet: facet,
            query: query
        )
    }
}
