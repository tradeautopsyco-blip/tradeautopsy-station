import Foundation
@testable import Station

final class FakeBrokerKeychainItemStore: BrokerKeychainItemStoring, @unchecked Sendable {
    private var items: [String: Data] = [:]
    /// When set for `service`, `deleteItem` throws instead of deleting.
    var deleteFailureByService: [String: BrokerCredentialTeardownError] = [:]
    private(set) var deleteCallCount = 0

    private func key(service: String, account: String) -> String {
        "\(service)|\(account)"
    }

    func seed(service: String, account: String, payload: Data = Data()) {
        items[key(service: service, account: account)] = payload
    }

    func deleteItem(service: String, account: String) throws {
        deleteCallCount += 1
        if let failure = deleteFailureByService[service] {
            throw failure
        }
        items.removeValue(forKey: key(service: service, account: account))
    }

    func hasItem(service: String, account: String) -> Bool {
        items[key(service: service, account: account)] != nil
    }

    func accounts(forService service: String) -> [String] {
        let prefix = "\(service)|"
        return items.keys.compactMap { keyed in
            guard keyed.hasPrefix(prefix) else { return nil }
            return String(keyed.dropFirst(prefix.count))
        }
    }
}
