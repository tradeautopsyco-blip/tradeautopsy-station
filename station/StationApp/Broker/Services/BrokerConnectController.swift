import Foundation

@MainActor
public final class BrokerConnectController {
    public private(set) var apiKey = ""
    public private(set) var apiSecret = ""
    public private(set) var consumerKey = ""
    public private(set) var mobileNumber = ""
    public private(set) var ucc = ""
    public private(set) var totp = ""
    public private(set) var mpin = ""
    public private(set) var permissionWarning: BrokerPermissionWarning?
    public private(set) var isValidating = false

    public let identity: BrokerConnectionIdentity
    public var behavioralAnalysisOptedOut: Bool
    public let authScheme: BrokerAuthScheme

    private let credentialStore: BrokerCredentialStoring
    private let validator: BrokerCredentialValidating
    private let syncControl: BrokerSyncControlling
    private let metadataStore: BrokerConnectionMetadataStoring
    private let runtimeClient: BrokerAgentRuntimeClient
    private let loginProfileStore: KotakLoginProfileStoring
    private let keychainItems: BrokerKeychainItemStoring
    private let openBrowserURL: (URL) -> Void

    public init(
        identity: BrokerConnectionIdentity,
        credentialStore: BrokerCredentialStoring,
        validator: BrokerCredentialValidating,
        syncControl: BrokerSyncControlling,
        metadataStore: BrokerConnectionMetadataStoring,
        runtimeClient: BrokerAgentRuntimeClient = LocalAgentBrokerRuntimeClient(),
        loginProfileStore: KotakLoginProfileStoring = KeychainKotakLoginProfileStore(),
        keychainItems: BrokerKeychainItemStoring = SecItemBrokerKeychainItemStore(),
        openBrowserURL: @escaping (URL) -> Void = BrokerConnectBrowser.openURL,
        behavioralAnalysisOptedOut: Bool = false
    ) {
        self.identity = identity
        self.credentialStore = credentialStore
        self.validator = validator
        self.syncControl = syncControl
        self.metadataStore = metadataStore
        self.runtimeClient = runtimeClient
        self.loginProfileStore = loginProfileStore
        self.keychainItems = keychainItems
        self.openBrowserURL = openBrowserURL
        self.behavioralAnalysisOptedOut = behavioralAnalysisOptedOut
        self.authScheme = BrokerConnectServices.authScheme(for: identity.brokerSlug)
        self.permissionWarning = metadataStore.load(for: identity)?.permissionWarning
    }

    public func updateFields(apiKey: String, apiSecret: String) {
        self.apiKey = apiKey
        self.apiSecret = apiSecret
    }

    public func updateKotakLoginFields(
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) {
        self.consumerKey = consumerKey
        self.mobileNumber = mobileNumber
        self.ucc = ucc
        self.totp = totp
        self.mpin = mpin
    }

    public func connect() async -> BrokerConnectOutcome {
        switch authScheme {
        case .hmacApiKeySecret:
            return await connectHmac()
        case .kotakNeoTotpSession:
            return await connectKotakTotp()
        case .kiteChecksumSession:
            return await connectZerodhaKite()
        case .upstoxOAuthBearerSession:
            return await connectUpstox()
        case .fyersOAuthJsonAppIdHashSession:
            return await connectFyers()
        case .growwChecksumSession:
            return await connectGroww()
        }
    }

    private func connectHmac() async -> BrokerConnectOutcome {
        let invalid = BrokerCredentialFieldValidator.invalidFields(apiKey: apiKey, apiSecret: apiSecret)
        guard invalid.isEmpty else {
            return .localValidationFailed(invalidFields: invalid)
        }
        let credentials = BrokerCredentials(
            apiKey: apiKey.trimmingCharacters(in: .whitespacesAndNewlines),
            apiSecret: apiSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        )

        isValidating = true
        defer { isValidating = false }

        let validation = await validator.validate(credentials: credentials, identity: identity)
        switch validation {
        case .success(let posture):
            return await handleSuccessfulValidation(credentials: credentials, posture: posture)
        case .transientFailure(let failure):
            return .validationTransientFailure(failure)
        case .permanentFailure(let failure):
            return .validationPermanentFailure(failure)
        }
    }

    private func connectKotakTotp() async -> BrokerConnectOutcome {
        let invalid = BrokerCredentialFieldValidator.invalidKotakLoginFields(
            consumerKey: consumerKey,
            mobileNumber: mobileNumber,
            ucc: ucc,
            totp: totp,
            mpin: mpin
        )
        guard invalid.isEmpty else {
            return .localValidationFailed(invalidFields: invalid)
        }

        isValidating = true
        let profileSnapshot = KotakLoginProfile(
            consumerKey: consumerKey.trimmingCharacters(in: .whitespacesAndNewlines),
            mobileNumber: mobileNumber.trimmingCharacters(in: .whitespacesAndNewlines),
            ucc: ucc.trimmingCharacters(in: .whitespacesAndNewlines),
            mpin: mpin.trimmingCharacters(in: .whitespacesAndNewlines)
        )
        defer {
            isValidating = false
            // Never leave TOTP/MPIN in controller memory after the attempt.
            totp = ""
            mpin = ""
        }

        let existingMetadata = metadataStore.load(for: identity)
        // Do not touch broker-credentials Keychain from Station for Kotak (cross-process ACL spam).
        // First Connect / remint auto-starts unless user previously paused.
        let shouldAutoStartSync = existingMetadata?.syncPaused != true

        do {
            try await runtimeClient.mintKotakSession(
                for: identity,
                consumerKey: profileSnapshot.consumerKey,
                mobileNumber: profileSnapshot.mobileNumber,
                ucc: profileSnapshot.ucc,
                totp: totp.trimmingCharacters(in: .whitespacesAndNewlines),
                mpin: profileSnapshot.mpin
            )
        } catch let BrokerAgentRuntimeError.kotakMintFailed(errorClass, message) {
            return mapKotakMintError(errorClass: errorClass, message: message)
        } catch BrokerAgentRuntimeError.requestFailed {
            return .validationTransientFailure(.networkUnavailable)
        } catch {
            return .validationTransientFailure(.networkUnavailable)
        }

        // Persist login profile (no TOTP) for Edit → Touch ID → TOTP-only remint.
        do {
            try loginProfileStore.save(profileSnapshot, for: identity)
        } catch let KotakLoginProfileStoreError.keychainError(status) {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .kotakMintRejected(
                    "Session minted but login profile Keychain save failed (status \(status)). Retry Connect."
                )
            )
        } catch {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .kotakMintRejected(
                    "Session minted but login profile Keychain save failed. Retry Connect so Edit can be TOTP-only."
                )
            )
        }

        // Verify vault via agent (in-process cache after mint) — never Station SecItem read.
        let present = await runtimeClient.vaultCredentialsPresent(for: identity)
        guard present else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .kotakMintRejected(
                    "Session minted but agent vault is empty — rebuild agent with keyring apple-native."
                )
            )
        }

        // Placeholder credentials: Kotak vault is agent-owned; Station must not rewrite it.
        let vaultMarker = BrokerCredentials(
            consumerKey: profileSnapshot.consumerKey,
            tradeToken: "agent-vault",
            sid: "agent-vault",
            baseUrl: "agent-vault",
            hsServerId: ""
        )
        return await persistMetadataAndMaybeStart(
            credentials: vaultMarker,
            warning: nil,
            shouldAutoStartSync: shouldAutoStartSync,
            existingMetadata: existingMetadata,
            rewriteCredentialVault: false
        )
    }

    private func connectZerodhaKite() async -> BrokerConnectOutcome {
        let invalid = BrokerCredentialFieldValidator.invalidFields(apiKey: apiKey, apiSecret: apiSecret)
        guard invalid.isEmpty else {
            return .localValidationFailed(invalidFields: invalid)
        }

        let trimmedKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedSecret = apiSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        let appCredentials = BrokerCredentials(kiteApiKey: trimmedKey, kiteApiSecret: trimmedSecret)

        isValidating = true
        defer { isValidating = false }

        let existingMetadata = metadataStore.load(for: identity)
        let shouldAutoStartSync = existingMetadata?.syncPaused != true

        do {
            try credentialStore.save(credentials: appCredentials, for: identity)
        } catch {
            return .validationTransientFailure(.networkUnavailable)
        }

        let begin: ZerodhaConnectBeginResult
        do {
            begin = try await runtimeClient.beginZerodhaConnect(
                for: identity,
                apiKey: trimmedKey,
                apiSecret: trimmedSecret
            )
        } catch let BrokerAgentRuntimeError.zerodhaBeginFailed(errorClass, message) {
            await teardownCredentialsBestEffort()
            return mapZerodhaBeginError(errorClass: errorClass, message: message)
        } catch BrokerAgentRuntimeError.requestFailed {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(.networkUnavailable)
        } catch {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(.networkUnavailable)
        }

        guard begin.redirectURI == ZerodhaKiteConnectContract.loopbackRedirectURI else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .kiteConnectRejected(
                    "Agent redirect URI mismatch — rebuild agent and register \(ZerodhaKiteConnectContract.loopbackRedirectURI) in Kite."
                )
            )
        }

        openBrowserURL(begin.loginURL)

        let vaultReady = await waitForAgentSessionVault()
        guard vaultReady else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .kiteConnectRejected(
                    "Kite login timed out or was not completed. Finish login in the browser, then Connect again."
                )
            )
        }

        return await persistMetadataAndMaybeStart(
            credentials: appCredentials,
            warning: nil,
            shouldAutoStartSync: shouldAutoStartSync,
            existingMetadata: existingMetadata,
            rewriteCredentialVault: false
        )
    }

    private func connectUpstox() async -> BrokerConnectOutcome {
        let invalid = BrokerCredentialFieldValidator.invalidFields(apiKey: apiKey, apiSecret: apiSecret)
        guard invalid.isEmpty else {
            return .localValidationFailed(invalidFields: invalid)
        }

        let trimmedClientId = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedClientSecret = apiSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        let appCredentials = BrokerCredentials(
            upstoxClientId: trimmedClientId,
            upstoxClientSecret: trimmedClientSecret
        )

        isValidating = true
        defer { isValidating = false }

        let existingMetadata = metadataStore.load(for: identity)
        let shouldAutoStartSync = existingMetadata?.syncPaused != true

        do {
            try credentialStore.save(credentials: appCredentials, for: identity)
        } catch {
            return .validationTransientFailure(.networkUnavailable)
        }

        let begin: UpstoxConnectBeginResult
        do {
            begin = try await runtimeClient.beginUpstoxConnect(
                for: identity,
                clientId: trimmedClientId,
                clientSecret: trimmedClientSecret
            )
        } catch let BrokerAgentRuntimeError.upstoxBeginFailed(errorClass, message) {
            await teardownCredentialsBestEffort()
            return mapUpstoxBeginError(errorClass: errorClass, message: message)
        } catch BrokerAgentRuntimeError.requestFailed {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(.networkUnavailable)
        } catch {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(.networkUnavailable)
        }

        guard begin.redirectURI == UpstoxConnectContract.loopbackRedirectURI else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .upstoxConnectRejected(
                    "Agent redirect URI mismatch — rebuild agent and register \(UpstoxConnectContract.loopbackRedirectURI) in Upstox."
                )
            )
        }

        openBrowserURL(begin.loginURL)

        let vaultReady = await waitForAgentSessionVault()
        guard vaultReady else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .upstoxConnectRejected(
                    "Upstox login timed out or was not completed. Finish login in the browser, then Connect again."
                )
            )
        }

        return await persistMetadataAndMaybeStart(
            credentials: appCredentials,
            warning: nil,
            shouldAutoStartSync: shouldAutoStartSync,
            existingMetadata: existingMetadata,
            rewriteCredentialVault: false
        )
    }

    private func connectGroww() async -> BrokerConnectOutcome {
        let invalid = BrokerCredentialFieldValidator.invalidFields(apiKey: apiKey, apiSecret: apiSecret)
        guard invalid.isEmpty else {
            return .localValidationFailed(invalidFields: invalid)
        }

        let trimmedKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedSecret = apiSecret.trimmingCharacters(in: .whitespacesAndNewlines)

        isValidating = true
        defer {
            isValidating = false
            apiSecret = ""
        }

        let existingMetadata = metadataStore.load(for: identity)
        let shouldAutoStartSync = existingMetadata?.syncPaused != true

        do {
            try await runtimeClient.connectGroww(
                for: identity,
                apiKey: trimmedKey,
                apiSecret: trimmedSecret
            )
        } catch let BrokerAgentRuntimeError.growwConnectFailed(errorClass, message) {
            return mapGrowwConnectError(errorClass: errorClass, message: message)
        } catch BrokerAgentRuntimeError.requestFailed {
            return .validationTransientFailure(.networkUnavailable)
        } catch {
            return .validationTransientFailure(.networkUnavailable)
        }

        let present = await runtimeClient.vaultCredentialsPresent(for: identity)
        guard present else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .growwConnectRejected(
                    "Session minted but agent vault is empty — rebuild agent with keyring apple-native."
                )
            )
        }

        let vaultMarker = BrokerCredentials(
            growwApiKey: trimmedKey,
            growwApiSecret: "agent-vault"
        )
        return await persistMetadataAndMaybeStart(
            credentials: vaultMarker,
            warning: nil,
            shouldAutoStartSync: shouldAutoStartSync,
            existingMetadata: existingMetadata,
            rewriteCredentialVault: false
        )
    }

    private func connectFyers() async -> BrokerConnectOutcome {
        let invalid = BrokerCredentialFieldValidator.invalidFields(apiKey: apiKey, apiSecret: apiSecret)
        guard invalid.isEmpty else {
            return .localValidationFailed(invalidFields: invalid)
        }

        let trimmedAppId = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedSecretId = apiSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        let appCredentials = BrokerCredentials(fyersAppId: trimmedAppId, fyersSecretId: trimmedSecretId)

        isValidating = true
        defer { isValidating = false }

        let existingMetadata = metadataStore.load(for: identity)
        let shouldAutoStartSync = existingMetadata?.syncPaused != true

        do {
            try credentialStore.save(credentials: appCredentials, for: identity)
        } catch {
            return .validationTransientFailure(.networkUnavailable)
        }

        let begin: FyersConnectBeginResult
        do {
            begin = try await runtimeClient.beginFyersConnect(
                for: identity,
                appId: trimmedAppId,
                secretId: trimmedSecretId
            )
        } catch let BrokerAgentRuntimeError.fyersBeginFailed(errorClass, message) {
            await teardownCredentialsBestEffort()
            return mapFyersBeginError(errorClass: errorClass, message: message)
        } catch BrokerAgentRuntimeError.requestFailed {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(.networkUnavailable)
        } catch {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(.networkUnavailable)
        }

        guard begin.redirectURI == FyersConnectContract.loopbackRedirectURI else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .fyersConnectRejected(
                    "Agent redirect URI mismatch — rebuild agent and register \(FyersConnectContract.loopbackRedirectURI) in Fyers."
                )
            )
        }

        openBrowserURL(begin.loginURL)

        let vaultReady = await waitForAgentSessionVault()
        guard vaultReady else {
            await teardownCredentialsBestEffort()
            return .validationTransientFailure(
                .fyersConnectRejected(
                    "Fyers login timed out or was not completed. Finish login in the browser, then Connect again."
                )
            )
        }

        return await persistMetadataAndMaybeStart(
            credentials: appCredentials,
            warning: nil,
            shouldAutoStartSync: shouldAutoStartSync,
            existingMetadata: existingMetadata,
            rewriteCredentialVault: false
        )
    }

    private func waitForAgentSessionVault() async -> Bool {
        let pollIntervalMs: UInt64 = 500
        let maxAttempts = 1_200 // ≈10 minutes (matches agent connect state TTL)
        for _ in 0..<maxAttempts {
            if Task.isCancelled { return false }
            if await runtimeClient.vaultCredentialsPresent(for: identity) {
                return true
            }
            try? await Task.sleep(for: .milliseconds(pollIntervalMs))
        }
        return false
    }

    private func mapZerodhaBeginError(errorClass: String, message: String) -> BrokerConnectOutcome {
        let safe = BrokerSecretGuard.sanitizeConnectMessage(message)
        switch errorClass {
        case "invalid_credentials":
            return .validationPermanentFailure(.kiteConnectRejected(safe))
        default:
            return .validationTransientFailure(.kiteConnectRejected(safe))
        }
    }

    private func mapUpstoxBeginError(errorClass: String, message: String) -> BrokerConnectOutcome {
        let safe = BrokerSecretGuard.sanitizeConnectMessage(message)
        switch errorClass {
        case "invalid_credentials":
            return .validationPermanentFailure(.upstoxConnectRejected(safe))
        default:
            return .validationTransientFailure(.upstoxConnectRejected(safe))
        }
    }

    private func mapFyersBeginError(errorClass: String, message: String) -> BrokerConnectOutcome {
        let safe = BrokerSecretGuard.sanitizeConnectMessage(message)
        switch errorClass {
        case "invalid_credentials":
            return .validationPermanentFailure(.fyersConnectRejected(safe))
        default:
            return .validationTransientFailure(.fyersConnectRejected(safe))
        }
    }

    private func mapGrowwConnectError(errorClass: String, message: String) -> BrokerConnectOutcome {
        let safe = BrokerSecretGuard.sanitizeConnectMessage(message)
        switch errorClass {
        case "invalid_credentials":
            return .validationPermanentFailure(.growwConnectRejected(safe))
        case "rate_limited":
            return .validationTransientFailure(.growwConnectRejected(safe))
        default:
            return .validationTransientFailure(.growwConnectRejected(safe))
        }
    }

    private func mapKotakMintError(errorClass: String, message: String) -> BrokerConnectOutcome {
        let safe = BrokerSecretGuard.sanitizeKotakMintMessage(message)
        switch errorClass {
        case "invalid_credentials", "totp_failed", "mpin_failed":
            return .validationPermanentFailure(.kotakMintRejected(safe))
        default:
            return .validationTransientFailure(.kotakMintRejected(safe))
        }
    }

    private func handleSuccessfulValidation(
        credentials: BrokerCredentials,
        posture: BrokerPermissionPosture
    ) async -> BrokerConnectOutcome {
        switch posture {
        case .withdrawDetected:
            return .blockedWithdrawPermission
        case .readOnlyConfirmed:
            return await persistAndStart(credentials: credentials, warning: nil)
        case .tradeEnabled:
            return await persistAndStart(credentials: credentials, warning: .tradeEnabled)
        case .unverifiable:
            return await persistAndStart(credentials: credentials, warning: .unverifiable)
        }
    }

    private func persistAndStart(
        credentials: BrokerCredentials,
        warning: BrokerPermissionWarning?
    ) async -> BrokerConnectOutcome {
        let existingMetadata = metadataStore.load(for: identity)
        let shouldAutoStartSync =
            !credentialStore.hasCredentials(for: identity)
            && existingMetadata?.syncPaused != true
        return await persistMetadataAndMaybeStart(
            credentials: credentials,
            warning: warning,
            shouldAutoStartSync: shouldAutoStartSync,
            existingMetadata: existingMetadata,
            rewriteCredentialVault: true
        )
    }

    private func persistMetadataAndMaybeStart(
        credentials: BrokerCredentials,
        warning: BrokerPermissionWarning?,
        shouldAutoStartSync: Bool,
        existingMetadata: BrokerConnectionMetadata?,
        rewriteCredentialVault: Bool
    ) async -> BrokerConnectOutcome {
        do {
            // HMAC: Station writes the vault. Kotak: agent mint already wrote it — skip rewrite
            // so tradeautopsy-agent keeps Keychain ACL ownership (no login-password prompt on Start).
            if rewriteCredentialVault {
                try credentialStore.save(credentials: credentials, for: identity)
            }
            let metadata = BrokerConnectionMetadata(
                permissionWarning: warning,
                lastValidatedAt: Date(),
                syncPaused: existingMetadata?.syncPaused ?? false
            )
            metadataStore.save(metadata, for: identity)
            permissionWarning = warning
            if shouldAutoStartSync {
                try await syncControl.startSync(for: identity)
                var startedMetadata = metadata
                startedMetadata.syncPaused = false
                metadataStore.save(startedMetadata, for: identity)
            }
            return .connected(permissionWarning: warning)
        } catch {
            return .validationTransientFailure(.networkUnavailable)
        }
    }

    public func deleteSavedCredentials() throws {
        try credentialStore.delete(for: identity)
        try? loginProfileStore.delete(for: identity)
        metadataStore.delete(for: identity)
        permissionWarning = nil
    }

    /// Async Delete / rollback helper — Station-owned Keychain goodbye, then agent cache clear.
    public func deleteSavedCredentialsFully() async throws {
        let teardown = BrokerCredentialTeardown(
            keychainItems: keychainItems,
            credentialStore: credentialStore,
            loginProfileStore: loginProfileStore,
            metadataStore: metadataStore,
            runtimeClient: runtimeClient
        )
        try await teardown.teardown(for: identity)
        permissionWarning = nil
    }

    /// Connect rollback — attempt full Keychain teardown; swallow errors so the Connect outcome wins.
    private func teardownCredentialsBestEffort() async {
        try? await deleteSavedCredentialsFully()
    }
}
