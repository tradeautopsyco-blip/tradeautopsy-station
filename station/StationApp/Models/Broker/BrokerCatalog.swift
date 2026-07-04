import Foundation

public enum PlannedBrokerAvailability: Equatable, Sendable {
    case enabled
    case planned
}

public struct PlannedBrokerDescriptor: Equatable, Identifiable, Sendable {
    public var id: String { slug }
    public let slug: String
    public let displayName: String
    public let assetClass: String
    public let availability: PlannedBrokerAvailability

    public init(
        slug: String,
        displayName: String,
        assetClass: String,
        availability: PlannedBrokerAvailability
    ) {
        self.slug = slug
        self.displayName = displayName
        self.assetClass = assetClass
        self.availability = availability
    }
}

public enum BrokerCatalog {
    public static let v1: [PlannedBrokerDescriptor] = [
        PlannedBrokerDescriptor(
            slug: "binance_us",
            displayName: "Binance.US",
            assetClass: "crypto",
            availability: .enabled
        ),
        PlannedBrokerDescriptor(
            slug: "interactive_brokers",
            displayName: "Interactive Brokers",
            assetClass: "equities",
            availability: .planned
        ),
        PlannedBrokerDescriptor(
            slug: "zerodha_kite",
            displayName: "Zerodha Kite",
            assetClass: "equities",
            availability: .planned
        ),
    ]
}
