import Foundation

public enum BrokerConnectSheetMode: Equatable, Sendable {
    /// First-time / full form (HMAC keys or Kotak consumer+mobile+UCC+TOTP+MPIN).
    case full
    /// Kotak remint: Touch ID unlocked profile; only TOTP is entered.
    case kotakTotpOnly
}

@MainActor
public final class BrokersViewModel: ObservableObject {
    @Published public private(set) var cards: [BrokerCardPresentation] = []
    @Published public var isConnectSheetPresented = false
    @Published public private(set) var connectSheetMode: BrokerConnectSheetMode = .full
    @Published public private(set) var connectBrokerSlug: String?
    @Published public private(set) var connectApiKey = ""
    @Published public private(set) var connectApiSecret = ""
    @Published public private(set) var connectConsumerKey = ""
    @Published public private(set) var connectMobileNumber = ""
    @Published public private(set) var connectUcc = ""
    @Published public private(set) var connectTotp = ""
    @Published public private(set) var connectMpin = ""
    @Published public private(set) var connectPassphrase = ""
    @Published public private(set) var connectPemPrivateKey = ""
    @Published public private(set) var connectMessage: String?
    @Published public private(set) var connectInvalidFields: Set<BrokerCredentialField> = []
    @Published public private(set) var isConnecting = false
    /// Surfaced when Start/Stop fails (was previously swallowed by `try?`).
    @Published public private(set) var syncActionMessage: String?
    /// Silent Keychain ACL probe — granted vs needs Always Allow. Station never writes ACL.
    @Published public private(set) var keychainGrantHint: String?
    /// Bumped when one-time secrets are cleared so the Connect sheet remounts empty SecureFields.
    @Published public private(set) var connectSecretFieldsEpoch = 0
    /// In-app broker OAuth (same Connect sheet — SwiftUI allows only one `.sheet`).
    @Published public private(set) var oauthWebLoginRequest: BrokerOAuthWebLoginPresenter.Request?
    /// After loopback callback, agent vault poll + metadata — keep OAuth sheet up instead of API-key form.
    @Published public private(set) var oauthWebLoginFinishing = false

    private var oauthWebLoginContinuation: CheckedContinuation<Bool, Never>?

    public var connectAuthScheme: BrokerAuthScheme {
        guard let slug = connectBrokerSlug else { return .hmacApiKeySecret }
        return BrokerConnectServices.authScheme(for: slug)
    }

    private let brokerControl: BrokerControlling
    private let credentialStore: BrokerCredentialStoring
    private let metadataStore: BrokerConnectionMetadataStoring
    private let syncControl: BrokerSyncControlling
    private let runtimeClient: BrokerAgentRuntimeClient
    private let loginProfileStore: KotakLoginProfileStoring
    private var connectController: BrokerConnectController?
    private var didReconcileVaultAccounts = false
    /// Set after Touch ID unlock for this Connect sheet; allows Flow B without a second LA prompt.
    private var kotakLoginUnlockedForSheet = false

    public var connectBrokerDisplayName: String {
        guard let slug = connectBrokerSlug else { return "Broker" }
        return BrokerConnectServices.displayName(for: slug)
    }

    public var connectDisclosure: String {
        if connectSheetMode == .kotakTotpOnly {
            return "Touch ID unlocked your saved Kotak login. Enter a fresh TOTP to refresh the session. MPIN and account fields stay in Keychain — never retyped."
        }
        return BrokerConnectDisclosure.message(
            behavioralAnalysisOptedOut: connectController?.behavioralAnalysisOptedOut ?? false
        )
    }

    /// Configured first-pair slugs (Keychain present) — input for desk honesty / dual strip.
    public var configuredBrokerSlugs: [String] {
        cards.compactMap { card in
            guard card.identity != nil else { return nil }
            return card.id
        }
    }

    public init(
        brokerControl: BrokerControlling,
        credentialStore: BrokerCredentialStoring,
        metadataStore: BrokerConnectionMetadataStoring,
        syncControl: BrokerSyncControlling,
        runtimeClient: BrokerAgentRuntimeClient = LocalAgentBrokerRuntimeClient(),
        loginProfileStore: KotakLoginProfileStoring = KeychainKotakLoginProfileStore()
    ) {
        self.brokerControl = brokerControl
        self.credentialStore = credentialStore
        self.metadataStore = metadataStore
        self.syncControl = syncControl
        self.runtimeClient = runtimeClient
        self.loginProfileStore = loginProfileStore
    }

    public func load() async {
        if !didReconcileVaultAccounts {
            let configuredIDs = configuredConnectionIDsForReconcile()
            BrokerCredentialOrphanCleanup.reconcileVaultAccounts(
                configuredConnectionIDs: configuredIDs
            )
            didReconcileVaultAccounts = true
            refreshKeychainGrantHint()
        }
        let snapshot = await brokerControl.loadSnapshot()
        cards = Self.applyValidatingOverlay(
            cards: BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1),
            validatingSlug: isConnecting ? connectBrokerSlug : nil
        )
    }

    /// Connection UUIDs Station considers live — input for launch Keychain reconcile.
    private func configuredConnectionIDsForReconcile() -> Set<UUID> {
        var ids = Set<UUID>()
        for slug in ["binance_com", "kotak_neo", "zerodha_kite", "upstox", "fyers", "groww", "dhan"] {
            let identity = BrokerConnectServices.identity(for: slug)
            if metadataStore.load(for: identity)?.lastValidatedAt != nil {
                ids.insert(identity.brokerConnectionID)
                continue
            }
            if slug == "kotak_neo" {
                if loginProfileStore.hasProfile(for: identity) {
                    ids.insert(identity.brokerConnectionID)
                }
            } else if credentialStore.hasCredentials(for: identity) {
                ids.insert(identity.brokerConnectionID)
            }
        }
        return ids
    }

    private func refreshKeychainGrantHint() {
        let identity = BrokerConnectServices.identity(for: "binance_com")
        switch credentialStore.accessGrant(for: identity) {
        case .granted:
            keychainGrantHint =
                "Mac Keychain access is granted for this app. If you need to change it, do that in Keychain Access — Station will not rewrite Always Allow."
        case .needsAlwaysAllow:
            keychainGrantHint =
                "Mac Keychain would prompt. Click Always Allow once (not Allow). Station does not change that for you."
        case .missing:
            keychainGrantHint = nil
        }
    }

    private static func applyValidatingOverlay(
        cards: [BrokerCardPresentation],
        validatingSlug: String?
    ) -> [BrokerCardPresentation] {
        guard let validatingSlug else { return cards }
        return cards.map { card in
            guard card.id == validatingSlug, card.status == .notConfigured else { return card }
            return BrokerCardPresentation(
                id: card.id,
                displayName: card.displayName,
                assetClass: card.assetClass,
                quoteCurrency: card.quoteCurrency,
                status: .validating,
                statusLabel: BrokerCardStatus.validating.rawValue,
                isConnectable: card.isConnectable,
                isEditEnabled: false,
                isStartEnabled: false,
                isStopEnabled: false,
                isDeleteEnabled: false,
                lastValidatedAtText: card.lastValidatedAtText,
                lastSyncSummary: card.lastSyncSummary,
                lastSyncedAtText: card.lastSyncedAtText,
                plannedLabel: card.plannedLabel,
                booksLine: card.booksLine,
                permissionWarning: card.permissionWarning,
                identity: card.identity
            )
        }
    }

    /// Full Connect sheet (mode `.full`) — **first-time onboard only**.
    /// Required when there is no saved Kotak login profile, or HMAC has no credentials yet.
    /// Must NOT open when Connect can Start from an existing session vault / remint path.
    public func presentConnectSheet(for slug: String, prefillConsumerKeyFromVault: Bool = true) {
        kotakLoginUnlockedForSheet = false
        connectSheetMode = .full
        connectBrokerSlug = slug
        connectApiKey = ""
        connectApiSecret = ""
        connectMobileNumber = ""
        connectUcc = ""
        connectTotp = ""
        connectMpin = ""
        connectPassphrase = ""
        connectPemPrivateKey = ""
        connectMessage = nil
        connectInvalidFields = []
        connectController = makeConnectController(for: slug)
        connectController?.updateFields(apiKey: "", apiSecret: "")
        connectController?.updateOkxFields(apiKey: "", apiSecret: "", passphrase: "")
        connectController?.updateCoinbaseFields(apiKey: "", pemPrivateKey: "")

        // Never read Kotak broker-credentials from Station (ACL password spam). HMAC may prefill apiKey.
        let identity = BrokerConnectServices.identity(for: slug)
        let savedConsumerKey = ""
        let savedApiKey: String
        let scheme = BrokerConnectServices.authScheme(for: slug)
        if prefillConsumerKeyFromVault,
           scheme == .hmacApiKeySecret || scheme == .kiteChecksumSession
            || scheme == .upstoxOAuthBearerSession || scheme == .fyersOAuthJsonAppIdHashSession
            || scheme == .growwChecksumSession || scheme == .dhanConsentSession
            || scheme == .okxPassphraseSession
            || scheme == .coinbaseJwtEs256Session {
            savedApiKey = (try? credentialStore.read(for: identity))?.apiKey ?? ""
        } else {
            savedApiKey = ""
        }
        connectApiKey = savedApiKey
        connectConsumerKey = savedConsumerKey
        connectController?.updateFields(apiKey: savedApiKey, apiSecret: "")
        connectController?.updateOkxFields(apiKey: savedApiKey, apiSecret: "", passphrase: "")
        connectController?.updateCoinbaseFields(apiKey: savedApiKey, pemPrivateKey: "")
        connectController?.updateKotakLoginFields(
            consumerKey: savedConsumerKey,
            mobileNumber: "",
            ucc: "",
            totp: "",
            mpin: ""
        )
        connectSecretFieldsEpoch += 1
        isConnectSheetPresented = true
    }

    /// Notch Offline Connect / desk “get this broker live”.
    /// Same Start semantics as the Brokers Start button (vault via `AgentBrokerSyncControl`):
    /// - No Kotak login profile → full Connect (first onboard)
    /// - Profile + session vault OK → `startSync`; if still disconnected → remint (unless kill switch)
    /// - Profile + vault missing → remint (`presentKotakTotpRemint`)
    /// - HMAC: Start if credentials present, else full Connect
    public func beginConnect(for slug: String) async {
        let identity = BrokerConnectServices.identity(for: slug)
        let scheme = BrokerConnectServices.authScheme(for: slug)

        if scheme == .kotakNeoTotpSession {
            if !loginProfileStore.hasProfile(for: identity) {
                presentConnectSheet(for: slug)
                return
            }
            syncActionMessage = nil
            do {
                try await brokerControl.startSync(for: identity)
            } catch BrokerSyncStartError.missingCredentials {
                syncActionMessage =
                    "Session needs a fresh TOTP — unlock with Touch ID, then enter the code."
                presentKotakTotpRemint(for: slug)
                await load()
                return
            } catch {
                syncActionMessage = "Start failed — enter a fresh TOTP to remint the session."
                presentKotakTotpRemint(for: slug)
                await load()
                return
            }

            await load()
            // Start clears lastError; first poll is often transitional `stale` before real posture.
            let health = await settleSyncHealth(for: identity)
            if let health {
                if health.killDnsActive {
                    syncActionMessage =
                        "Kill switch is blocking Kotak hosts. Wait for countdown, tap I'm Calm, then Connect again."
                    return
                }
                if health.isUbiHostBlocked {
                    // UBI allowlist rejected session baseUrl — not Kill DNS (dns may already be clear).
                    syncActionMessage =
                        "Kotak trade host rejected by Station allowlist — remint with a fresh TOTP (Edit / Re-auth)."
                    presentKotakTotpRemint(for: slug)
                    return
                }
                if health.isLiveEnoughForConnectSuccess {
                    syncActionMessage = "Sync started — session vault OK."
                    return
                }
                syncActionMessage =
                    "Session vault present but broker still offline — enter a fresh TOTP."
                presentKotakTotpRemint(for: slug)
                return
            }
            syncActionMessage =
                "Sync start accepted, but broker status could not be confirmed. Try Connect again or check Station Brokers."
            return
        }

        syncActionMessage = nil
        do {
            try await brokerControl.startSync(for: identity)
            syncActionMessage = "Sync started — credentials OK."
        } catch BrokerSyncStartError.missingCredentials {
            presentConnectSheet(for: slug)
        } catch BrokerAgentRuntimeError.syncStartFailed(let message) {
            syncActionMessage = Self.sanitizeAgentErrorForUI(message)
        } catch {
            syncActionMessage =
                "Start failed. If a Keychain password dialog appeared, click Always Allow, then Start again."
        }
        await load()
    }

    private static func sanitizeAgentErrorForUI(_ raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.localizedCaseInsensitiveContains("component")
            || trimmed.localizedCaseInsensitiveContains(".wasm")
        {
            return "Start failed — broker adapter missing from the app bundle. Rebuild Station in Xcode (Product → Build), quit, reopen, then Start again."
        }
        if trimmed.count > 240 {
            return String(trimmed.prefix(240)) + "…"
        }
        return trimmed.isEmpty
            ? "Start failed. Rebuild Station, quit, reopen, then Start again."
            : trimmed
    }

    /// Poll briefly so Connect does not treat post-Start transitional `stale` as success.
    private func settleSyncHealth(
        for identity: BrokerConnectionIdentity
    ) async -> BrokerSyncHealthSnapshot? {
        var last: BrokerSyncHealthSnapshot?
        for _ in 0..<6 {
            try? await Task.sleep(for: .milliseconds(150))
            last = await runtimeClient.fetchSyncHealth(for: identity)
            guard let health = last else { continue }
            if health.killDnsActive || health.isUbiHostBlocked || health.isDisconnected
                || health.isLiveEnoughForConnectSuccess
            {
                return health
            }
        }
        return last
    }

    /// TOTP remint / re-auth (mode `.kotakTotpOnly`).
    /// Touch ID unlocks saved login; sheet asks only for a fresh TOTP.
    /// Required for: explicit Notch Re-auth, Station Edit, Start/Connect when vault missing.
    /// Falls back to full Connect if profile missing/incomplete.
    public func presentKotakTotpRemint(for slug: String) {
        connectBrokerSlug = slug
        connectMessage = nil
        connectInvalidFields = []
        connectTotp = ""
        connectController = makeConnectController(for: slug)

        if BrokerConnectServices.authScheme(for: slug) == .kotakNeoTotpSession {
            let identity = BrokerConnectServices.identity(for: slug)
            do {
                guard loginProfileStore.hasProfile(for: identity) else {
                    presentConnectSheet(for: slug, prefillConsumerKeyFromVault: false)
                    connectMessage =
                        "Login not saved. Enter consumer key, mobile, UCC, MPIN, and a fresh TOTP once — then Edit will be TOTP-only."
                    return
                }
                guard let profile = try loginProfileStore.unlock(for: identity), profile.isComplete else {
                    presentConnectSheet(for: slug, prefillConsumerKeyFromVault: false)
                    connectMessage =
                        "Saved login profile incomplete. Enter consumer key, mobile, UCC, MPIN, and a fresh TOTP once."
                    return
                }
                connectSheetMode = .kotakTotpOnly
                connectConsumerKey = profile.consumerKey
                connectMobileNumber = profile.mobileNumber
                connectUcc = profile.ucc
                connectMpin = profile.mpin
                connectController?.updateKotakLoginFields(
                    consumerKey: profile.consumerKey,
                    mobileNumber: profile.mobileNumber,
                    ucc: profile.ucc,
                    totp: "",
                    mpin: profile.mpin
                )
                connectSecretFieldsEpoch += 1
                kotakLoginUnlockedForSheet = true
                isConnectSheetPresented = true
            } catch KotakLoginProfileStoreError.userCancelled {
                connectMessage = nil
                kotakLoginUnlockedForSheet = false
            } catch {
                presentConnectSheet(for: slug, prefillConsumerKeyFromVault: false)
                connectMessage = "Could not unlock saved Kotak login. Use the full Connect form."
                kotakLoginUnlockedForSheet = false
            }
            return
        }

        // HMAC: remint/edit reopens key/secret form.
        presentConnectSheet(for: slug)
    }

    /// Flow B: after Touch ID on this sheet, switch from TOTP-only remint to editable login fields.
    public func beginChangeKotakLoginDetails() {
        guard connectSheetMode == .kotakTotpOnly, kotakLoginUnlockedForSheet else { return }
        connectSheetMode = .full
        connectMessage = nil
        connectSecretFieldsEpoch += 1
    }

    /// Edit / remint — delegates to `presentKotakTotpRemint` (Kotak TOTP-only or HMAC full form).
    public func presentEditSheet(for slug: String) {
        presentKotakTotpRemint(for: slug)
    }

    public func updateConnectFields(apiKey: String, apiSecret: String) {
        connectApiKey = apiKey
        connectApiSecret = apiSecret
        connectController?.updateFields(apiKey: apiKey, apiSecret: apiSecret)
    }

    public func updateOkxConnectFields(apiKey: String, apiSecret: String, passphrase: String) {
        connectApiKey = apiKey
        connectApiSecret = apiSecret
        connectPassphrase = passphrase
        connectController?.updateOkxFields(apiKey: apiKey, apiSecret: apiSecret, passphrase: passphrase)
    }

    public func updateCoinbaseConnectFields(apiKey: String, pemPrivateKey: String) {
        connectApiKey = apiKey
        connectPemPrivateKey = pemPrivateKey
        connectController?.updateCoinbaseFields(apiKey: apiKey, pemPrivateKey: pemPrivateKey)
    }

    public func updateDhanConnectFields(dhanClientId: String, apiKey: String, apiSecret: String) {
        connectConsumerKey = dhanClientId
        connectApiKey = apiKey
        connectApiSecret = apiSecret
        connectController?.updateDhanFields(
            dhanClientId: dhanClientId,
            apiKey: apiKey,
            apiSecret: apiSecret
        )
    }

    public func updateKotakLoginFields(
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) {
        connectConsumerKey = consumerKey
        connectMobileNumber = mobileNumber
        connectUcc = ucc
        connectTotp = totp
        connectMpin = mpin
        connectController?.updateKotakLoginFields(
            consumerKey: consumerKey,
            mobileNumber: mobileNumber,
            ucc: ucc,
            totp: totp,
            mpin: mpin
        )
    }

    public func submitConnect() async {
        guard let connectController, let slug = connectBrokerSlug else { return }

        // Re-push published fields into the controller (sheet may have just updated drafts).
        if connectAuthScheme == .kotakNeoTotpSession {
            connectController.updateKotakLoginFields(
                consumerKey: connectConsumerKey,
                mobileNumber: connectMobileNumber,
                ucc: connectUcc,
                totp: connectTotp,
                mpin: connectMpin
            )
        } else if connectAuthScheme == .dhanConsentSession {
            connectController.updateDhanFields(
                dhanClientId: connectConsumerKey,
                apiKey: connectApiKey,
                apiSecret: connectApiSecret
            )
        } else if connectAuthScheme == .okxPassphraseSession {
            connectController.updateOkxFields(
                apiKey: connectApiKey,
                apiSecret: connectApiSecret,
                passphrase: connectPassphrase
            )
        } else if connectAuthScheme == .coinbaseJwtEs256Session {
            connectController.updateCoinbaseFields(
                apiKey: connectApiKey,
                pemPrivateKey: connectPemPrivateKey
            )
        } else {
            connectController.updateFields(apiKey: connectApiKey, apiSecret: connectApiSecret)
        }

        isConnecting = true
        cards = Self.applyValidatingOverlay(cards: cards, validatingSlug: slug)
        defer {
            isConnecting = false
            oauthWebLoginRequest = nil
            oauthWebLoginFinishing = false
        }

        let outcome = await connectController.connect()
        switch outcome {
        case .localValidationFailed(let invalidFields):
            connectInvalidFields = invalidFields
            switch connectAuthScheme {
            case .kotakNeoTotpSession:
                connectMessage = "Enter consumer key, mobile, UCC, TOTP, and MPIN."
            case .kiteChecksumSession:
                connectMessage = "Enter both Kite API key and secret."
            case .upstoxOAuthBearerSession:
                connectMessage = "Enter both Upstox API key and secret."
            case .fyersOAuthJsonAppIdHashSession:
                connectMessage = "Enter both Fyers app ID and secret ID."
            case .hmacApiKeySecret, .krakenSpotNonceSession:
                connectMessage = "Enter both API key and secret."
            case .okxPassphraseSession:
                connectMessage = "Enter API key, secret, and passphrase."
            case .coinbaseJwtEs256Session:
                connectMessage = "Enter API key name and PEM private key."
            case .growwChecksumSession:
                connectMessage = "Enter both Groww API key and secret."
            case .dhanConsentSession:
                connectMessage = "Enter Dhan client ID, app ID, and app secret."
            }
        case .blockedWithdrawPermission:
            connectMessage = "Withdraw permission detected. Use a key without withdraw access."
        case .validationTransientFailure(let failure):
            connectMessage = transientFailureMessage(failure, slug: slug)
            // Keep TOTP/MPIN drafts for retry on agent/network blips.
            // Only clear one-time secrets on permanent rejection / success.
            if case .kotakMintRejected = failure {
                // Upstream Kotak rejection — code may be spent; clear for a fresh TOTP.
                clearOneTimeKotakSecrets()
            }
            if case .kiteConnectRejected = failure {
                connectApiSecret = ""
                connectSecretFieldsEpoch += 1
            }
            if case .upstoxConnectRejected = failure {
                connectApiSecret = ""
                connectSecretFieldsEpoch += 1
            }
            if case .fyersConnectRejected = failure {
                connectApiSecret = ""
                connectSecretFieldsEpoch += 1
            }
            if case .growwConnectRejected = failure {
                connectApiSecret = ""
                connectSecretFieldsEpoch += 1
            }
            if case .dhanConnectRejected = failure {
                connectApiSecret = ""
                connectSecretFieldsEpoch += 1
            }
        case .validationPermanentFailure(let failure):
            connectMessage = permanentFailureMessage(failure, slug: slug)
            clearOneTimeKotakSecrets()
        case .connected:
            connectMessage = nil
            connectMpin = ""
            connectApiSecret = ""
            connectPassphrase = ""
            connectPemPrivateKey = ""
            connectSecretFieldsEpoch += 1
            connectSheetMode = .full
            clearOneTimeKotakSecrets()
            kotakLoginUnlockedForSheet = false
            isConnectSheetPresented = false
        }

        await load()
    }

    /// Call when the Connect sheet closes without a successful connect (Cancel).
    public func noteConnectSheetDismissed() {
        kotakLoginUnlockedForSheet = false
    }

    private func clearOneTimeKotakSecrets() {
        guard connectAuthScheme == .kotakNeoTotpSession else { return }
        connectTotp = ""
        // Keep MPIN in VM only while totp-only sheet is open; clear when sheet closes / full mode.
        if connectSheetMode == .full {
            connectMpin = ""
        }
        connectSecretFieldsEpoch += 1
        connectController?.updateKotakLoginFields(
            consumerKey: connectConsumerKey,
            mobileNumber: connectMobileNumber,
            ucc: connectUcc,
            totp: "",
            mpin: connectSheetMode == .kotakTotpOnly ? connectMpin : ""
        )
    }

    public func startSync(for identity: BrokerConnectionIdentity) async {
        syncActionMessage = nil
        do {
            try await brokerControl.startSync(for: identity)
        } catch BrokerSyncStartError.missingCredentials {
            switch BrokerConnectServices.authScheme(for: identity.brokerSlug) {
            case .kotakNeoTotpSession:
                syncActionMessage =
                    "Session needs a fresh TOTP — unlock with Touch ID, then enter the code."
                presentKotakTotpRemint(for: identity.brokerSlug)
            case .kiteChecksumSession:
                syncActionMessage =
                    "Kite session missing or expired — Connect again and finish browser login."
                presentConnectSheet(for: identity.brokerSlug)
            case .upstoxOAuthBearerSession:
                syncActionMessage =
                    "Upstox session missing or expired — Connect again and finish browser login."
                presentConnectSheet(for: identity.brokerSlug)
            case .fyersOAuthJsonAppIdHashSession:
                syncActionMessage =
                    "Fyers session missing or expired — Connect again and finish browser login."
                presentConnectSheet(for: identity.brokerSlug)
            case .growwChecksumSession:
                syncActionMessage =
                    "Groww session missing or expired — Connect again with API key and secret."
                presentConnectSheet(for: identity.brokerSlug)
            case .dhanConsentSession:
                syncActionMessage =
                    "Dhan session missing or expired — Connect again and finish browser login."
                presentConnectSheet(for: identity.brokerSlug)
            case .hmacApiKeySecret, .okxPassphraseSession, .krakenSpotNonceSession,
                 .coinbaseJwtEs256Session:
                syncActionMessage =
                    "Cannot Start — credentials missing. Use Connect to enter API keys again."
                presentConnectSheet(for: identity.brokerSlug)
            }
        } catch BrokerAgentRuntimeError.syncStartFailed(let message) {
            syncActionMessage = Self.sanitizeAgentErrorForUI(message)
        } catch {
            syncActionMessage =
                "Start failed. If a Keychain password dialog appeared, click Always Allow, then Start again."
        }
        await load()
    }

    public func stopSync(for identity: BrokerConnectionIdentity) async {
        syncActionMessage = nil
        do {
            try await brokerControl.stopSync(for: identity)
        } catch {
            syncActionMessage = "Stop failed. Retry, or quit and reopen Station."
        }
        await load()
    }

    @Published public var pendingDeleteIdentity: BrokerConnectionIdentity?
    @Published public var isDeleteConfirmationPresented = false

    public func requestDelete(for identity: BrokerConnectionIdentity) {
        pendingDeleteIdentity = identity
        isDeleteConfirmationPresented = true
    }

    public func confirmDelete() async {
        guard let identity = pendingDeleteIdentity else { return }
        defer {
            pendingDeleteIdentity = nil
            isDeleteConfirmationPresented = false
        }
        let controller = makeConnectController(for: identity.brokerSlug)
        do {
            try await brokerControl.deleteConnection(
                for: identity,
                connectController: controller
            )
            syncActionMessage = nil
        } catch {
            syncActionMessage = "Could not fully disconnect — Keychain clear failed. The broker stays configured until teardown succeeds."
        }
        await load()
    }

    public func cancelDelete() {
        pendingDeleteIdentity = nil
        isDeleteConfirmationPresented = false
    }

    public func completeOAuthWebLogin(success: Bool) {
        if success {
            oauthWebLoginFinishing = true
        } else {
            oauthWebLoginRequest = nil
            oauthWebLoginFinishing = false
        }
        oauthWebLoginContinuation?.resume(returning: success)
        oauthWebLoginContinuation = nil
    }

    private func runOAuthWebLogin(
        loginURL: URL,
        callbackPrefix: String,
        title: String
    ) async -> Bool {
        await withCheckedContinuation { continuation in
            oauthWebLoginContinuation = continuation
            oauthWebLoginRequest = BrokerOAuthWebLoginPresenter.Request(
                loginURL: loginURL,
                callbackPrefix: callbackPrefix,
                title: title
            )
        }
    }

    private func makeConnectController(for slug: String) -> BrokerConnectController {
        let displayName = BrokerConnectServices.displayName(for: slug)
        return BrokerConnectController(
            identity: BrokerConnectServices.identity(for: slug),
            credentialStore: credentialStore,
            validator: BrokerConnectServices.validator(for: slug),
            syncControl: syncControl,
            metadataStore: metadataStore,
            runtimeClient: runtimeClient,
            loginProfileStore: loginProfileStore,
            oauthWebLogin: { [weak self] loginURL, callbackPrefix in
                guard let self else { return false }
                return await self.runOAuthWebLogin(
                    loginURL: loginURL,
                    callbackPrefix: callbackPrefix,
                    title: "Sign in to \(displayName)"
                )
            }
        )
    }

    private func transientFailureMessage(
        _ failure: BrokerCredentialValidationFailure,
        slug: String
    ) -> String {
        let brokerName = BrokerConnectServices.displayName(for: slug)
        switch failure {
        case .networkUnavailable:
            return "Cannot reach the local Enforcer agent. If Station shows an agent warning, click Retry — then Connect again."
        case .rateLimited:
            return "\(brokerName) rate limited validation. Try again shortly."
        case .brokerUnavailable:
            return "\(brokerName) login is unavailable right now. Try again shortly."
        case .invalidCredentials:
            return "Could not validate credentials."
        case .kotakMintRejected(let detail):
            return detail.isEmpty ? "Kotak login failed. Try again shortly." : detail
        case .kiteConnectRejected(let detail):
            return detail.isEmpty
                ? "Kite login failed. Check API key and secret, then try Connect again."
                : detail
        case .upstoxConnectRejected(let detail):
            return detail.isEmpty
                ? "Upstox login failed. Check API key and secret, then try Connect again."
                : detail
        case .fyersConnectRejected(let detail):
            return detail.isEmpty
                ? "Fyers login failed. Check app ID and secret ID, then try Connect again."
                : detail
        case .growwConnectRejected(let detail):
            return detail.isEmpty
                ? "Groww login failed. Check API key and secret, then try Connect again."
                : detail
        case .dhanConnectRejected(let detail):
            return detail.isEmpty
                ? "Dhan login failed. Check client ID and app credentials, then try Connect again."
                : detail
        }
    }

    private func permanentFailureMessage(
        _ failure: BrokerCredentialValidationFailure,
        slug: String
    ) -> String {
        let brokerName = BrokerConnectServices.displayName(for: slug)
        switch failure {
        case .invalidCredentials:
            switch connectAuthScheme {
            case .kotakNeoTotpSession:
                return "Kotak login rejected. Check consumer key, mobile, UCC, TOTP, and MPIN."
            case .kiteChecksumSession:
                return "Kite API key or secret rejected. Check Kite developer settings and try again."
            case .upstoxOAuthBearerSession:
                return "Upstox API key or secret rejected. Check Upstox developer settings and try again."
            case .fyersOAuthJsonAppIdHashSession:
                return "Fyers app ID or secret ID rejected. Check Fyers developer settings and try again."
            case .hmacApiKeySecret, .okxPassphraseSession, .krakenSpotNonceSession,
                 .coinbaseJwtEs256Session:
                return "Credentials were rejected by \(brokerName)."
            case .growwChecksumSession:
                return "Groww API key or secret rejected. Check Groww Cloud API Keys and try again."
            case .dhanConsentSession:
                return "Dhan client ID or app credentials rejected. Check Dhan API settings and try again."
            }
        case .kotakMintRejected(let detail):
            return detail.isEmpty
                ? "Kotak login rejected. Check consumer key, mobile, UCC, TOTP, and MPIN."
                : detail
        case .kiteConnectRejected(let detail):
            return detail.isEmpty
                ? "Kite login rejected. Check API key and secret."
                : detail
        case .upstoxConnectRejected(let detail):
            return detail.isEmpty
                ? "Upstox login rejected. Check API key and secret."
                : detail
        case .fyersConnectRejected(let detail):
            return detail.isEmpty
                ? "Fyers login rejected. Check app ID and secret ID."
                : detail
        case .growwConnectRejected(let detail):
            return detail.isEmpty
                ? "Groww login rejected. Check API key and secret."
                : detail
        case .dhanConnectRejected(let detail):
            return detail.isEmpty
                ? "Dhan login rejected. Check client ID and app credentials."
                : detail
        case .networkUnavailable, .rateLimited, .brokerUnavailable:
            return transientFailureMessage(failure, slug: slug)
        }
    }
}
