import Foundation
@testable import Station

final class FakeProviderAPIKeyStore: ProviderAPIKeyStoring, @unchecked Sendable {
    private var storage: [String: ProviderAPIKeyRecord] = [:]
    private(set) var saveCallCount = 0
    private(set) var deleteCallCount = 0

    private func storageKey(for identity: ProviderAPIKeyIdentity) -> String {
        "\(identity.namespace.rawValue).\(identity.providerSlug).\(identity.keyID.uuidString)"
    }

    func save(_ record: ProviderAPIKeyRecord, for identity: ProviderAPIKeyIdentity) throws {
        saveCallCount += 1
        storage[storageKey(for: identity)] = record
    }

    func read(for identity: ProviderAPIKeyIdentity) throws -> ProviderAPIKeyRecord? {
        storage[storageKey(for: identity)]
    }

    func delete(for identity: ProviderAPIKeyIdentity) throws {
        deleteCallCount += 1
        storage.removeValue(forKey: storageKey(for: identity))
    }

    func listIdentities(in namespace: ProviderKeyNamespace) throws -> [ProviderAPIKeyIdentity] {
        let prefix = "\(namespace.rawValue)."
        return storage.keys.compactMap { key in
            guard key.hasPrefix(prefix) else { return nil }
            let parts = key.split(separator: ".", maxSplits: 2).map(String.init)
            guard parts.count == 3,
                  let namespace = ProviderKeyNamespace(rawValue: parts[0]),
                  let keyID = UUID(uuidString: parts[2])
            else { return nil }
            return ProviderAPIKeyIdentity(
                namespace: namespace,
                providerSlug: parts[1],
                keyID: keyID
            )
        }
    }

    func hasKey(for identity: ProviderAPIKeyIdentity) -> Bool {
        storage[storageKey(for: identity)] != nil
    }
}
