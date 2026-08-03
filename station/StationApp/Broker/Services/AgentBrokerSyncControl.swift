import Foundation

/// Confirms credentials exist, then starts sync by connection identity only (R6).
/// Secrets stay in Keychain; agent loads them via host vault — never posted on the wire.
///
/// Kotak: never Station-read `broker-credentials` (cross-process ACL prompts). Presence is
/// agent-side (cached after mint). COM/HMAC: Station Keychain presence first, else agent.
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
        let present: Bool
        if identity.brokerSlug == "kotak_neo" {
            present = await runtimeClient.vaultCredentialsPresent(for: identity)
        } else if (try? credentialStore.read(for: identity)) != nil {
            present = true
        } else {
            present = await runtimeClient.vaultCredentialsPresent(for: identity)
        }
        guard present else {
            throw BrokerSyncStartError.missingCredentials
        }
        try await runtimeClient.startSync(for: identity)
    }
}

public enum BrokerSyncStartError: Error, Equatable {
    case missingCredentials
}
