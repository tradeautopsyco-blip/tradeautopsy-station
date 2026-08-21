import Foundation
@testable import Station

final class FakeBrokerCredentialStore: BrokerCredentialStoring, @unchecked Sendable {
    private var storage: [String: BrokerCredentials] = [:]
    private(set) var saveCallCount = 0
    private(set) var deleteCallCount = 0
    private(set) var readCallCount = 0
    private(set) var hasCredentialsCallCount = 0
    var nextAccessGrant: BrokerKeychainAccessGrant?

    private func storageKey(for identity: BrokerConnectionIdentity) -> String {
        "\(identity.environment).\(identity.brokerSlug).\(identity.brokerConnectionID.uuidString)"
    }

    func save(credentials: BrokerCredentials, for identity: BrokerConnectionIdentity) throws {
        saveCallCount += 1
        storage[storageKey(for: identity)] = credentials
    }

    func read(for identity: BrokerConnectionIdentity) throws -> BrokerCredentials? {
        readCallCount += 1
        return storage[storageKey(for: identity)]
    }

    func delete(for identity: BrokerConnectionIdentity) throws {
        deleteCallCount += 1
        storage.removeValue(forKey: storageKey(for: identity))
    }

    func hasCredentials(for identity: BrokerConnectionIdentity) -> Bool {
        hasCredentialsCallCount += 1
        return storage[storageKey(for: identity)] != nil
    }

    func accessGrant(for identity: BrokerConnectionIdentity) -> BrokerKeychainAccessGrant {
        if let nextAccessGrant { return nextAccessGrant }
        return storage[storageKey(for: identity)] != nil ? .granted : .missing
    }
}
