@testable import Station
import Foundation

final class FakeKotakLoginProfileStore: KotakLoginProfileStoring, @unchecked Sendable {
    private var profiles: [String: KotakLoginProfile] = [:]
    private(set) var saveCallCount = 0
    private(set) var unlockCallCount = 0
    private(set) var deleteCallCount = 0
    var unlockError: Error?
    var saveError: Error?
    var requireUnlockPrompt = false

    func save(_ profile: KotakLoginProfile, for identity: BrokerConnectionIdentity) throws {
        saveCallCount += 1
        if let saveError { throw saveError }
        profiles[key(identity)] = profile
    }

    func unlock(for identity: BrokerConnectionIdentity) throws -> KotakLoginProfile? {
        unlockCallCount += 1
        if let unlockError { throw unlockError }
        return profiles[key(identity)]
    }

    func hasProfile(for identity: BrokerConnectionIdentity) -> Bool {
        profiles[key(identity)] != nil
    }

    func delete(for identity: BrokerConnectionIdentity) throws {
        deleteCallCount += 1
        profiles.removeValue(forKey: key(identity))
    }

    private func key(_ identity: BrokerConnectionIdentity) -> String {
        "\(identity.environment).\(identity.brokerSlug).login"
    }
}
