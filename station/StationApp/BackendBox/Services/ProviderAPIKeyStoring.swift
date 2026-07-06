import Foundation

public enum ProviderAPIKeyStoreError: Error, Equatable {
    case encodingFailed
    case keychainError(OSStatus)
    case keyNotFound
}

public protocol ProviderAPIKeyStoring: Sendable {
    func save(_ record: ProviderAPIKeyRecord, for identity: ProviderAPIKeyIdentity) throws
    func read(for identity: ProviderAPIKeyIdentity) throws -> ProviderAPIKeyRecord?
    func delete(for identity: ProviderAPIKeyIdentity) throws
    func listIdentities(in namespace: ProviderKeyNamespace) throws -> [ProviderAPIKeyIdentity]
    func hasKey(for identity: ProviderAPIKeyIdentity) -> Bool
}
