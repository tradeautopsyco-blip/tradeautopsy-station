import Foundation
@testable import Station

@MainActor
final class FakeBrokerAgentRuntimeClient: BrokerAgentRuntimeClient {
    var runtimeStatus: BrokerCardStatus = .connected
    var mintError: BrokerAgentRuntimeError?
    /// When set, successful mint writes a Kotak session blob (mirrors agent vault).
    var credentialStore: BrokerCredentialStoring?
    var vaultPresentOverride: Bool?
    var syncHealthOverride: BrokerSyncHealthSnapshot?
    /// Active slug returned from sync-state when override is nil (default first-pair COM).
    var syncHealthBrokerSlug: String? = "binance_com"
    var syncHealthLastSuccessAtMs: Int64?
    var syncHealthLastPollAtMs: Int64?

    private(set) var startSyncCallCount = 0
    private(set) var stopSyncCallCount = 0
    private(set) var fetchRuntimeStatusCallCount = 0
    private(set) var fetchSyncHealthCallCount = 0
    private(set) var mintKotakSessionCallCount = 0
    private(set) var beginZerodhaConnectCallCount = 0
    var beginZerodhaConnectResult: ZerodhaConnectBeginResult?
    var beginZerodhaConnectError: BrokerAgentRuntimeError?
    /// When true, vault presence flips after begin (simulates browser callback completing).
    var simulateZerodhaVaultAfterBegin = false
    private var zerodhaBeginCompleted = false
    private(set) var beginUpstoxConnectCallCount = 0
    var beginUpstoxConnectResult: UpstoxConnectBeginResult?
    var beginUpstoxConnectError: BrokerAgentRuntimeError?
    var simulateUpstoxVaultAfterBegin = false
    private var upstoxBeginCompleted = false
    private(set) var clearVaultCredentialsCallCount = 0
    private(set) var vaultCredentialsPresentCallCount = 0
    private(set) var lastStartedIdentity: BrokerConnectionIdentity?
    private(set) var lastStoppedIdentity: BrokerConnectionIdentity?
    private(set) var lastClearedIdentity: BrokerConnectionIdentity?
    /// When set, `clearVaultCredentials` throws (agent cache clear is non-authoritative).
    var clearVaultError: Error?

    func startSync(for identity: BrokerConnectionIdentity) async throws {
        startSyncCallCount += 1
        lastStartedIdentity = identity
        runtimeStatus = .syncing
        syncHealthBrokerSlug = identity.brokerSlug
    }

    func stopSync(for identity: BrokerConnectionIdentity) async throws {
        stopSyncCallCount += 1
        lastStoppedIdentity = identity
        runtimeStatus = .paused
    }

    func fetchRuntimeStatus(for identity: BrokerConnectionIdentity) async -> BrokerCardStatus? {
        fetchRuntimeStatusCallCount += 1
        return await fetchSyncHealth(for: identity)?.cardStatus
    }

    func fetchSyncHealth(for identity: BrokerConnectionIdentity) async -> BrokerSyncHealthSnapshot? {
        fetchSyncHealthCallCount += 1
        _ = identity
        if let syncHealthOverride {
            return syncHealthOverride
        }
        let (syncState, runtimeWire) = Self.wireLabels(for: runtimeStatus)
        return BrokerSyncHealthSnapshot(
            syncState: syncState,
            runtimeStatus: runtimeWire,
            lastError: nil,
            killDnsActive: false,
            brokerSlug: syncHealthBrokerSlug,
            lastSuccessAtMs: syncHealthLastSuccessAtMs,
            lastPollAtMs: syncHealthLastPollAtMs
        )
    }

    func beginZerodhaConnect(
        for identity: BrokerConnectionIdentity,
        apiKey: String,
        apiSecret: String
    ) async throws -> ZerodhaConnectBeginResult {
        _ = (identity, apiKey, apiSecret)
        beginZerodhaConnectCallCount += 1
        zerodhaBeginCompleted = true
        if let beginZerodhaConnectError {
            throw beginZerodhaConnectError
        }
        if let beginZerodhaConnectResult {
            return beginZerodhaConnectResult
        }
        return ZerodhaConnectBeginResult(
            state: "test-state",
            loginURL: URL(string: "https://kite.zerodha.com/connect/login?v=3&api_key=test")!,
            redirectURI: ZerodhaKiteConnectContract.loopbackRedirectURI
        )
    }

    func beginUpstoxConnect(
        for identity: BrokerConnectionIdentity,
        clientId: String,
        clientSecret: String
    ) async throws -> UpstoxConnectBeginResult {
        _ = (identity, clientId, clientSecret)
        beginUpstoxConnectCallCount += 1
        upstoxBeginCompleted = true
        if let beginUpstoxConnectError {
            throw beginUpstoxConnectError
        }
        if let beginUpstoxConnectResult {
            return beginUpstoxConnectResult
        }
        return UpstoxConnectBeginResult(
            state: "test-state",
            loginURL: URL(
                string: "https://api.upstox.com/v2/login/authorization/dialog?response_type=code&client_id=test"
            )!,
            redirectURI: UpstoxConnectContract.loopbackRedirectURI
        )
    }

    func mintKotakSession(
        for identity: BrokerConnectionIdentity,
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) async throws {
        _ = (mobileNumber, ucc, totp, mpin)
        mintKotakSessionCallCount += 1
        if let mintError {
            throw mintError
        }
        guard let credentialStore else { return }
        try credentialStore.save(
            credentials: BrokerCredentials(
                consumerKey: consumerKey,
                tradeToken: "minted-trade-token",
                sid: "minted-sid",
                baseUrl: "https://cis.kotaksecurities.com",
                hsServerId: ""
            ),
            for: identity
        )
    }

    func clearVaultCredentials(for identity: BrokerConnectionIdentity) async throws {
        clearVaultCredentialsCallCount += 1
        lastClearedIdentity = identity
        if let clearVaultError {
            throw clearVaultError
        }
        try credentialStore?.delete(for: identity)
    }

    func vaultCredentialsPresent(for identity: BrokerConnectionIdentity) async -> Bool {
        vaultCredentialsPresentCallCount += 1
        if let vaultPresentOverride {
            return vaultPresentOverride
        }
        if simulateZerodhaVaultAfterBegin, zerodhaBeginCompleted {
            return true
        }
        if simulateUpstoxVaultAfterBegin, upstoxBeginCompleted {
            return true
        }
        return credentialStore?.hasCredentials(for: identity) ?? false
    }

    private static func wireLabels(for status: BrokerCardStatus) -> (syncState: String, runtimeStatus: String) {
        switch status {
        case .connected:
            return ("synced", "syncing")
        case .syncing:
            return ("syncing", "syncing")
        case .degraded:
            return ("stale", "degraded")
        case .rateLimited:
            return ("stale", "rate_limited")
        case .paused:
            return ("disconnected", "paused")
        case .readyToStart, .notConfigured, .validating, .failed, .unavailableAgentOffline:
            return ("disconnected", "ready_to_start")
        }
    }
}
