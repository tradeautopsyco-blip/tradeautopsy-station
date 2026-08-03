import Foundation

public protocol BrokerEnvironmentStoring: Sendable {
    func loadActiveEnvironment() -> TradeAutopsyEnvironment
    func saveActiveEnvironment(_ environment: TradeAutopsyEnvironment)
}

public final class UserDefaultsBrokerEnvironmentStore: BrokerEnvironmentStoring, @unchecked Sendable {
    private let defaults: UserDefaults
    private let key = "tradeautopsy.active_environment"

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func loadActiveEnvironment() -> TradeAutopsyEnvironment {
        guard let raw = defaults.string(forKey: key),
              let environment = TradeAutopsyEnvironment(rawValue: raw)
        else {
            return .prod
        }
        return environment
    }

    public func saveActiveEnvironment(_ environment: TradeAutopsyEnvironment) {
        defaults.set(environment.rawValue, forKey: key)
    }
}

@MainActor
public final class BrokerEnvironmentController {
    public private(set) var activeEnvironment: TradeAutopsyEnvironment
    public let switchingEnabled: Bool

    private let environmentStore: BrokerEnvironmentStoring
    private let brokerControl: BrokerControlling
    private let metadataStore: BrokerConnectionMetadataStoring
    private let runtimeClient: BrokerAgentRuntimeClient

    public init(
        environmentStore: BrokerEnvironmentStoring,
        brokerControl: BrokerControlling,
        metadataStore: BrokerConnectionMetadataStoring,
        runtimeClient: BrokerAgentRuntimeClient,
        switchingEnabled: Bool = InternalBuildGate.environmentSwitchingEnabled()
    ) {
        self.environmentStore = environmentStore
        self.brokerControl = brokerControl
        self.metadataStore = metadataStore
        self.runtimeClient = runtimeClient
        self.switchingEnabled = switchingEnabled
        self.activeEnvironment = environmentStore.loadActiveEnvironment()
    }

    public func activeIdentity(
        brokerSlug: String = "binance_com",
        assetClass: String = "crypto_spot"
    ) -> BrokerConnectionIdentity {
        _ = (brokerSlug, assetClass)
        return BrokerConnectionIdentity.binanceCom(activeEnvironment)
    }

    public func switchEnvironment(
        to environment: TradeAutopsyEnvironment,
        connectController: BrokerConnectController,
        syncWasRunning: Bool
    ) async throws {
        guard switchingEnabled else { return }
        guard environment != activeEnvironment else { return }

        let currentIdentity = activeIdentity()
        if syncWasRunning {
            try await runtimeClient.stopSync(for: currentIdentity)
        }
        metadataStore.delete(for: currentIdentity)

        activeEnvironment = environment
        environmentStore.saveActiveEnvironment(environment)
    }
}

public extension BrokerConnectionIdentity {
    static func binanceUS(_ environment: TradeAutopsyEnvironment) -> BrokerConnectionIdentity {
        BrokerConnectionIdentity(
            brokerConnectionID: UUID(uuidString: "00000000-0000-4000-8000-000000000001")!,
            brokerSlug: "binance_us",
            assetClass: "crypto",
            environment: environment.rawValue
        )
    }

    static var binanceUSProd: BrokerConnectionIdentity {
        binanceUS(.prod)
    }

    static func binanceCom(_ environment: TradeAutopsyEnvironment) -> BrokerConnectionIdentity {
        BrokerConnectionIdentity(
            brokerConnectionID: UUID(uuidString: "00000000-0000-4000-8000-000000000002")!,
            brokerSlug: "binance_com",
            assetClass: "crypto_spot",
            environment: environment.rawValue
        )
    }

    static var binanceComProd: BrokerConnectionIdentity {
        binanceCom(.prod)
    }

    static let kotakNeoConnectionID = UUID(uuidString: "00000000-0000-4000-8000-000000000003")!

    static func kotakNeo(_ environment: TradeAutopsyEnvironment) -> BrokerConnectionIdentity {
        BrokerConnectionIdentity(
            brokerConnectionID: kotakNeoConnectionID,
            brokerSlug: "kotak_neo",
            assetClass: "equities",
            environment: environment.rawValue
        )
    }

    static var kotakNeoProd: BrokerConnectionIdentity {
        kotakNeo(.prod)
    }
}
