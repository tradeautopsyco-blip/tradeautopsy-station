import Foundation
import LocalAuthentication
import Security

public enum KotakLoginProfileStoreError: Error, Equatable {
    case encodingFailed
    case keychainError(OSStatus)
    case accessControlUnavailable
    case userCancelled
    case biometryUnavailable
}

/// Keychain store for Kotak login profile (consumer / mobile / UCC / MPIN — never TOTP).
///
/// Uses the same **login** Keychain style as `KeychainBrokerCredentialStore` (no
/// `kSecUseDataProtectionKeychain`). DP Keychain requires entitlements that ad-hoc Station
/// builds do not have — that caused Connect to mint then fail with "login profile Keychain save failed".
///
/// Edit unlocks with one Touch ID / device passcode via LocalAuthentication, then TOTP-only remint.
public final class KeychainKotakLoginProfileStore: KotakLoginProfileStoring, @unchecked Sendable {
    public static let serviceName = "in.tradeautopsy.station.kotak-login-profile"

    public init() {}

    public func save(_ profile: KotakLoginProfile, for identity: BrokerConnectionIdentity) throws {
        let data = try encode(profile)
        let query = baseQuery(for: identity)
        // Clear both login-keychain and any leftover DP-keychain item from earlier builds.
        SecItemDelete(query as CFDictionary)
        var dpQuery = query
        dpQuery[kSecUseDataProtectionKeychain as String] = true
        SecItemDelete(dpQuery as CFDictionary)

        var addQuery = query
        addQuery[kSecValueData as String] = data
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly

        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KotakLoginProfileStoreError.keychainError(status)
        }
    }

    public func unlock(for identity: BrokerConnectionIdentity) throws -> KotakLoginProfile? {
        guard hasProfile(for: identity) else {
            return nil
        }

        try authenticateDeviceOwner(
            reason: "Unlock saved Kotak login to enter a fresh TOTP"
        )

        var query = baseQuery(for: identity)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess, let data = item as? Data else {
            throw KotakLoginProfileStoreError.keychainError(status)
        }
        return try decode(data)
    }

    public func hasProfile(for identity: BrokerConnectionIdentity) -> Bool {
        var query = baseQuery(for: identity)
        query[kSecReturnAttributes as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        return status == errSecSuccess
    }

    public func delete(for identity: BrokerConnectionIdentity) throws {
        let query = baseQuery(for: identity)
        let status = SecItemDelete(query as CFDictionary)
        var dpQuery = query
        dpQuery[kSecUseDataProtectionKeychain as String] = true
        _ = SecItemDelete(dpQuery as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KotakLoginProfileStoreError.keychainError(status)
        }
    }

    private func authenticateDeviceOwner(reason: String) throws {
        let box = AuthBox()
        let sem = DispatchSemaphore(value: 0)
        // LocalAuthentication UI runs on the main run loop. `BrokersViewModel` calls
        // `unlock` on @MainActor — blocking the main thread on `sem.wait()` deadlocks
        // Touch ID / passcode and freezes Station until timeout.
        DispatchQueue.main.async {
            let context = LAContext()
            var laError: NSError?
            guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &laError) else {
                box.error = .biometryUnavailable
                sem.signal()
                return
            }
            context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason) { success, error in
                defer { sem.signal() }
                guard success else {
                    if let la = error as? LAError, la.code == .userCancel || la.code == .appCancel {
                        box.error = .userCancelled
                    } else {
                        box.error = .userCancelled
                    }
                    return
                }
            }
        }

        let deadline = Date().addingTimeInterval(120)
        if Thread.isMainThread {
            while sem.wait(timeout: .now()) == .timedOut {
                if Date() >= deadline {
                    throw KotakLoginProfileStoreError.userCancelled
                }
                RunLoop.main.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05))
            }
        } else {
            let waitResult = sem.wait(timeout: .now() + 120)
            if waitResult == .timedOut {
                throw KotakLoginProfileStoreError.userCancelled
            }
        }
        if let authError = box.error {
            throw authError
        }
    }

    private func baseQuery(for identity: BrokerConnectionIdentity) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.serviceName,
            kSecAttrAccount as String: account(for: identity),
        ]
    }

    private func account(for identity: BrokerConnectionIdentity) -> String {
        "\(identity.environment).\(identity.brokerSlug).login"
    }

    private func encode(_ profile: KotakLoginProfile) throws -> Data {
        guard let data = try? JSONEncoder().encode(profile) else {
            throw KotakLoginProfileStoreError.encodingFailed
        }
        return data
    }

    private func decode(_ data: Data) throws -> KotakLoginProfile {
        guard let profile = try? JSONDecoder().decode(KotakLoginProfile.self, from: data) else {
            throw KotakLoginProfileStoreError.encodingFailed
        }
        return profile
    }
}

private final class AuthBox: @unchecked Sendable {
    var error: KotakLoginProfileStoreError?
}

public enum StationBiometricGate {
    public static func canEvaluateDeviceOwnerAuth() -> Bool {
        let context = LAContext()
        var error: NSError?
        return context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
    }
}
