import Foundation
import Security

/// Launch reconciler: purge Keychain broker accounts whose connection UUID is not configured.
///
/// Historical dogfood / fixture IDs (e.g. `…099`) are test fixtures only — product policy is
/// reconcile-against-configured-set, not a hardcoded orphan list.
public enum BrokerCredentialOrphanCleanup {
    public static let brokerCredentialsService = KeychainBrokerCredentialStore.serviceName
    /// Matches agent `KOTAK_SESSION_KEYCHAIN_SERVICE`.
    public static let kotakSessionVaultService = "in.tradeautopsy.station.kotak-session-vault"
    public static let canonicalKotakConnectionID = BrokerConnectionIdentity.kotakNeoConnectionID

    /// Test fixture only — not product policy. See `reconcileVaultAccounts`.
    public static let orphanKotakConnectionIDFixture = UUID(
        uuidString: "00000000-0000-4000-8000-000000000099"
    )!

    public static let reconciledBrokerSlug = "kotak_neo"

    /// Purge accounts on contracted services whose connection id is outside `configuredConnectionIDs`.
    @discardableResult
    public static func reconcileVaultAccounts(
        configuredConnectionIDs: Set<UUID>,
        environments: [String] = ["prod", "paper"],
        keychainItems: BrokerKeychainItemStoring = SecItemBrokerKeychainItemStore(),
        brokerSlug: String = BrokerCredentialOrphanCleanup.reconciledBrokerSlug
    ) -> Int {
        var deleted = 0
        let services = [brokerCredentialsService, kotakSessionVaultService]
        let envSet = Set(environments)

        for service in services {
            for account in keychainItems.accounts(forService: service) {
                guard let parsed = parseConnectionAccount(account, brokerSlug: brokerSlug),
                      envSet.contains(parsed.environment),
                      !configuredConnectionIDs.contains(parsed.connectionID)
                else {
                    continue
                }
                do {
                    try keychainItems.deleteItem(service: service, account: account)
                    deleted += 1
                } catch {
                    // Best-effort launch reconcile — leave item for next launch / explicit teardown.
                    continue
                }
            }
        }
        return deleted
    }

    /// Backward-compatible entry used by older call sites — reconciles with empty configured set
    /// when callers do not yet pass live IDs (purges all `{env}.kotak_neo.{uuid}` leftovers).
    @discardableResult
    public static func purgeOrphanKotakVaultAccounts(
        environments: [String] = ["prod", "paper"],
        configuredConnectionIDs: Set<UUID> = [],
        keychainItems: BrokerKeychainItemStoring = SecItemBrokerKeychainItemStore()
    ) -> Int {
        reconcileVaultAccounts(
            configuredConnectionIDs: configuredConnectionIDs,
            environments: environments,
            keychainItems: keychainItems
        )
    }

    private static func parseConnectionAccount(
        _ account: String,
        brokerSlug: String
    ) -> (environment: String, connectionID: UUID)? {
        // `{env}.{broker_slug}.{connection_id}` — slug may contain underscores.
        let marker = ".\(brokerSlug)."
        guard let range = account.range(of: marker) else { return nil }
        let environment = String(account[..<range.lowerBound])
        let idString = String(account[range.upperBound...])
        guard !environment.isEmpty, let connectionID = UUID(uuidString: idString) else {
            return nil
        }
        return (environment, connectionID)
    }
}
