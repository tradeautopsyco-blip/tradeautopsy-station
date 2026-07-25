import Foundation

public enum BrokerSyncStartError: Error, Equatable {
    case missingCredentials
}

/// Confirms Keychain has credentials, then starts sync by connection identity only (R6).
/// Secrets stay in Keychain; agent loads them via host vault — never posted on the wire.
@MainActor
public final class AgentBrokerSyncControl: BrokerSyncControlling {
    private let credentialStore: BrokerCredentialStoring
    private let runtimeClient: BrokerAgentRuntimeClient

    public init(
        credentialStore: BrokerCredentialStoring = KeychainBrokerCredentialStore(),
        runtimeClient: BrokerAgentRuntimeClient = LocalAgentBrokerRuntimeClient()
    ) {
        self.credentialStore = credentialStore
        self.runtimeClient = runtimeClient
    }

    public func startSync(for identity: BrokerConnectionIdentity) async throws {
        guard try credentialStore.read(for: identity) != nil else {
            throw BrokerSyncStartError.missingCredentials
        }
        try await runtimeClient.startSync(for: identity)
    }
}
