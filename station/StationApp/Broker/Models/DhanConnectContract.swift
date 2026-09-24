import Foundation

/// ADR 0009 consent flow — agent owns the loopback callback; Swift never touches
/// `tokenId`/`state`. Swift opens the agent-minted `consent_login_url` in the
/// browser, then polls agent vault presence for Keychain confirmation.
public enum DhanConnectContract {
    /// Must match the Dhan consent-app Redirect URL and the agent callback route.
    /// Documented here for parity with sibling contracts; Swift never opens it.
    public static let loopbackRedirectURI =
        "http://127.0.0.1:\(AgentLoopback.port)/api/daemon/broker/dhan/callback"

    /// Official Dhan consent-login host (B6 row 0/10).
    public static let consentLoginHost = "auth.dhan.co"
    /// Consent-app login path (B6 row 10: `GET /login/consentApp-login?consentAppId=`).
    public static let consentLoginPathPrefix = "/login/consentApp-login"

    /// Guards the `connect/begin` response: the login URL must be the official
    /// Dhan consent-login page. Anything else fails closed before opening a browser.
    public static func isValidConsentLoginURL(_ url: URL) -> Bool {
        guard let host = url.host?.lowercased(), host == consentLoginHost else {
            return false
        }
        guard url.scheme?.lowercased() == "https" else {
            return false
        }
        return url.path.hasPrefix(consentLoginPathPrefix)
    }
}
