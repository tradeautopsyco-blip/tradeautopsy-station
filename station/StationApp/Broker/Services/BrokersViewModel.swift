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
    @Published public private(set) var connectMessage: String?
    @Published public private(set) var connectInvalidFields: Set<BrokerCredentialField> = []
    @Published public private(set) var isConnecting = false
    /// Surfaced when Start/Stop fails (was previously swallowed by `try?`).
    @Published public private(set) var syncActionMessage: String?
    /// Bumped when one-time secrets are cleared so the Connect sheet remounts empty SecureFields.
    @Published public private(set) var connectSecretFieldsEpoch = 0

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
        let configuredIDs = configuredConnectionIDsForReconcile()
        BrokerCredentialOrphanCleanup.reconcileVaultAccounts(
            configuredConnectionIDs: configuredIDs
        )
        let snapshot = await brokerControl.loadSnapshot()
        cards = Self.applyValidatingOverlay(
            cards: BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1),
            validatingSlug: isConnecting ? connectBrokerSlug : nil
        )
    }

    /// Connection UUIDs Station considers live — input for launch Keychain reconcile.
    private func configuredConnectionIDsForReconcile() -> Set<UUID> {
        var ids = Set<UUID>()
        for slug in ["binance_com", "kotak_neo"] {
            let identity = BrokerConnectServices.identity(for: slug)
            if slug == "kotak_neo" {
                if metadataStore.load(for: identity)?.lastValidatedAt != nil
                    || loginProfileStore.hasProfile(for: identity)
                {
                    ids.insert(identity.brokerConnectionID)
                }
            } else if credentialStore.hasCredentials(for: identity) {
                ids.insert(identity.brokerConnectionID)
            }
        }
        return ids
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
                permissionWarning: card.permissionWarning,
                identity: card.identity
            )
        }
    }

    /// Full Connect sheet (mode `.full`) — **first-time onboard only**.
    /// Required when there is no saved Kotak login profile, or HMAC has no credentials yet.
    /// Must NOT open when Connect can Start from an existing session vault / remint path.
    public func presentConnectSheet(for slug: String, prefillConsumerKeyFromVault: Bool = true) {
        connectSheetMode = .full
        connectBrokerSlug = slug
        connectApiKey = ""
        connectApiSecret = ""
        connectMobileNumber = ""
        connectUcc = ""
        connectTotp = ""
        connectMpin = ""
        connectMessage = nil
        connectInvalidFields = []
        connectController = makeConnectController(for: slug)
        connectController?.updateFields(apiKey: "", apiSecret: "")

        // Never read Kotak broker-credentials from Station (ACL password spam). HMAC may prefill apiKey.
        let identity = BrokerConnectServices.identity(for: slug)
        let savedConsumerKey = ""
        let savedApiKey: String
        if prefillConsumerKeyFromVault,
           BrokerConnectServices.authScheme(for: slug) == .hmacApiKeySecret {
            savedApiKey = (try? credentialStore.read(for: identity))?.apiKey ?? ""
        } else {
            savedApiKey = ""
        }
        connectApiKey = savedApiKey
        connectConsumerKey = savedConsumerKey
        connectController?.updateFields(apiKey: savedApiKey, apiSecret: "")
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
        } catch {
            syncActionMessage =
                "Start failed. If a Keychain password dialog appeared, click Always Allow, then Start again."
        }
        await load()
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
                isConnectSheetPresented = true
            } catch KotakLoginProfileStoreError.userCancelled {
                connectMessage = nil
            } catch {
                presentConnectSheet(for: slug, prefillConsumerKeyFromVault: false)
                connectMessage = "Could not unlock saved Kotak login. Use the full Connect form."
            }
            return
        }

        // HMAC: remint/edit reopens key/secret form.
        presentConnectSheet(for: slug)
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
        } else {
            connectController.updateFields(apiKey: connectApiKey, apiSecret: connectApiSecret)
        }

        isConnecting = true
        cards = Self.applyValidatingOverlay(cards: cards, validatingSlug: slug)
        defer { isConnecting = false }

        let outcome = await connectController.connect()
        switch outcome {
        case .localValidationFailed(let invalidFields):
            connectInvalidFields = invalidFields
            connectMessage = connectAuthScheme == .kotakNeoTotpSession
                ? "Enter consumer key, mobile, UCC, TOTP, and MPIN."
                : "Enter both API key and secret."
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
        case .validationPermanentFailure(let failure):
            connectMessage = permanentFailureMessage(failure, slug: slug)
            clearOneTimeKotakSecrets()
        case .connected:
            connectMessage = nil
            connectMpin = ""
            connectSheetMode = .full
            clearOneTimeKotakSecrets()
            isConnectSheetPresented = false
        }

        await load()
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
            if BrokerConnectServices.authScheme(for: identity.brokerSlug) == .kotakNeoTotpSession {
                syncActionMessage =
                    "Session needs a fresh TOTP — unlock with Touch ID, then enter the code."
                presentKotakTotpRemint(for: identity.brokerSlug)
            } else {
                syncActionMessage =
                    "Cannot Start — session vault missing. Use Edit / Connect with a fresh TOTP."
            }
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

    private func makeConnectController(for slug: String) -> BrokerConnectController {
        BrokerConnectController(
            identity: BrokerConnectServices.identity(for: slug),
            credentialStore: credentialStore,
            validator: BrokerConnectServices.validator(for: slug),
            syncControl: syncControl,
            metadataStore: metadataStore,
            runtimeClient: runtimeClient,
            loginProfileStore: loginProfileStore
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
        }
    }

    private func permanentFailureMessage(
        _ failure: BrokerCredentialValidationFailure,
        slug: String
    ) -> String {
        let brokerName = BrokerConnectServices.displayName(for: slug)
        switch failure {
        case .invalidCredentials:
            return connectAuthScheme == .kotakNeoTotpSession
                ? "Kotak login rejected. Check consumer key, mobile, UCC, TOTP, and MPIN."
                : "Credentials were rejected by \(brokerName)."
        case .kotakMintRejected(let detail):
            return detail.isEmpty
                ? "Kotak login rejected. Check consumer key, mobile, UCC, TOTP, and MPIN."
                : detail
        case .networkUnavailable, .rateLimited, .brokerUnavailable:
            return transientFailureMessage(failure, slug: slug)
        }
    }
}
