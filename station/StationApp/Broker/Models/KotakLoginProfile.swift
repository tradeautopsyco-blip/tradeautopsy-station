import Foundation

/// Durable Kotak Neo login fields stored once in Keychain (never TOTP).
/// Remint unlocks this blob with Touch ID / device passcode, then asks only for a fresh TOTP.
public struct KotakLoginProfile: Equatable, Sendable, Codable {
    public let consumerKey: String
    public let mobileNumber: String
    public let ucc: String
    public let mpin: String

    public init(consumerKey: String, mobileNumber: String, ucc: String, mpin: String) {
        self.consumerKey = consumerKey
        self.mobileNumber = mobileNumber
        self.ucc = ucc
        self.mpin = mpin
    }

    public var isComplete: Bool {
        !consumerKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !mobileNumber.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !ucc.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !mpin.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

public protocol KotakLoginProfileStoring: Sendable {
    func save(_ profile: KotakLoginProfile, for identity: BrokerConnectionIdentity) throws
    /// Loads profile; may prompt Touch ID / device passcode when access-controlled.
    func unlock(for identity: BrokerConnectionIdentity) throws -> KotakLoginProfile?
    /// Existence check without prompting biometric UI when possible.
    func hasProfile(for identity: BrokerConnectionIdentity) -> Bool
    func delete(for identity: BrokerConnectionIdentity) throws
}
