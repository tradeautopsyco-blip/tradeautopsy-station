import Foundation

@MainActor
public final class BrokerConnectController {
    public private(set) var apiKey = ""
    public private(set) var apiSecret = ""
    public private(set) var consumerKey = ""
    public private(set) var tradeToken = ""
    public private(set) var sid = ""
    public private(set) var baseUrl = ""
    public private(set) var permissionWarning: BrokerPermissionWarning?
    public private(set) var isValidating = false

    public let identity: BrokerConnectionIdentity
    public var behavioralAnalysisOptedOut: Bool
    public let authScheme: BrokerAuthScheme

    private let credentialStore: BrokerCredentialStoring
    private let validator: BrokerCredentialValidating
    private let syncControl: BrokerSyncControlling
    private let metadataStore: BrokerConnectionMetadataStoring

    public init(
        identity: BrokerConnectionIdentity,
        credentialStore: BrokerCredentialStoring,
        validator: BrokerCredentialValidating,
        syncControl: BrokerSyncControlling,
        metadataStore: BrokerConnectionMetadataStoring,
        behavioralAnalysisOptedOut: Bool = false
    ) {
        self.identity = identity
        self.credentialStore = credentialStore
        self.validator = validator
        self.syncControl = syncControl
        self.metadataStore = metadataStore
        self.behavioralAnalysisOptedOut = behavioralAnalysisOptedOut
        self.authScheme = BrokerConnectServices.authScheme(for: identity.brokerSlug)
        self.permissionWarning = metadataStore.load(for: identity)?.permissionWarning
    }

    public func updateFields(apiKey: String, apiSecret: String) {
        self.apiKey = apiKey
        self.apiSecret = apiSecret
    }

    public func updateKotakFields(
        consumerKey: String,
        tradeToken: String,
        sid: String,
        baseUrl: String
    ) {
        self.consumerKey = consumerKey
        self.tradeToken = tradeToken
        self.sid = sid
        self.baseUrl = baseUrl
    }

    public func connect() async -> BrokerConnectOutcome {
        let credentials: BrokerCredentials
        switch authScheme {
        case .hmacApiKeySecret:
            let invalid = BrokerCredentialFieldValidator.invalidFields(apiKey: apiKey, apiSecret: apiSecret)
            guard invalid.isEmpty else {
                return .localValidationFailed(invalidFields: invalid)
            }
            credentials = BrokerCredentials(
                apiKey: apiKey.trimmingCharacters(in: .whitespacesAndNewlines),
                apiSecret: apiSecret.trimmingCharacters(in: .whitespacesAndNewlines)
            )
        case .kotakNeoTotpSession:
            let invalid = BrokerCredentialFieldValidator.invalidKotakFields(
                consumerKey: consumerKey,
                tradeToken: tradeToken,
                sid: sid,
                baseUrl: baseUrl
            )
            guard invalid.isEmpty else {
                return .localValidationFailed(invalidFields: invalid)
            }
            credentials = BrokerCredentials(
                consumerKey: consumerKey.trimmingCharacters(in: .whitespacesAndNewlines),
                tradeToken: tradeToken.trimmingCharacters(in: .whitespacesAndNewlines),
                sid: sid.trimmingCharacters(in: .whitespacesAndNewlines),
                baseUrl: baseUrl.trimmingCharacters(in: .whitespacesAndNewlines)
            )
        }

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
        do {
            let existingMetadata = metadataStore.load(for: identity)
            let shouldAutoStartSync =
                !credentialStore.hasCredentials(for: identity)
                && existingMetadata?.syncPaused != true
            try credentialStore.save(credentials: credentials, for: identity)
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
        metadataStore.delete(for: identity)
        permissionWarning = nil
    }
}
