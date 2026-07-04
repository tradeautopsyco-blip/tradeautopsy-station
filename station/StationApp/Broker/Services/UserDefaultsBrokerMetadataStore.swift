import Foundation

public final class UserDefaultsBrokerMetadataStore: BrokerConnectionMetadataStoring, @unchecked Sendable {
    private let defaults: UserDefaults
    private let keyPrefix = "station.broker.metadata."

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    private func storageKey(for identity: BrokerConnectionIdentity) -> String {
        keyPrefix + "\(identity.environment).\(identity.brokerSlug).\(identity.brokerConnectionID.uuidString)"
    }

    public func load(for identity: BrokerConnectionIdentity) -> BrokerConnectionMetadata? {
        guard let data = defaults.data(forKey: storageKey(for: identity)) else { return nil }
        return try? JSONDecoder().decode(BrokerConnectionMetadata.self, from: data)
    }

    public func save(_ metadata: BrokerConnectionMetadata, for identity: BrokerConnectionIdentity) {
        guard let data = try? JSONEncoder().encode(metadata) else { return }
        defaults.set(data, forKey: storageKey(for: identity))
    }

    public func delete(for identity: BrokerConnectionIdentity) {
        defaults.removeObject(forKey: storageKey(for: identity))
    }
}
