import Foundation

public enum BrokerSyncStartError: Error, Equatable {
    case missingCredentials
}

/// Reads credentials fresh from Keychain and forwards start to the local agent (#13).
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
        guard let credentials = try credentialStore.read(for: identity) else {
            throw BrokerSyncStartError.missingCredentials
        }
        try await runtimeClient.startSync(for: identity, credentials: credentials)
    }
}
